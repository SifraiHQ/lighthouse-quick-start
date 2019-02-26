use crate::*;
use bls::create_proof_of_possession;

/// Builds a `BeaconState` for use in testing or benchmarking.
///
/// Building the `BeaconState` is a three step processes:
///
/// 1. Create a new `BeaconStateBuilder`.
/// 2. Run the `genesis` function to generate a new BeaconState.
/// 3. (Optional) Use builder functions to modify the `BeaconState`.
/// 4. Call `build()` to obtain a cloned `BeaconState`.
///
/// Step (2) is necessary because step (3) requires an existing `BeaconState` object. (2) is not
/// included in (1) to allow for modifying params before generating the `BeaconState` (e.g., the
/// spec).
///
/// Step (4) produces a clone of the BeaconState and doesn't consume the `BeaconStateBuilder` to
/// allow access to the `keypairs` and `spec`.
pub struct BeaconStateBuilder {
    pub state: Option<BeaconState>,
    pub genesis_time: u64,
    pub initial_validator_deposits: Vec<Deposit>,
    pub latest_eth1_data: Eth1Data,
    pub spec: ChainSpec,
    pub keypairs: Vec<Keypair>,
}

impl BeaconStateBuilder {
    /// Create a new builder with the given number of validators.
    pub fn with_random_validators(validator_count: usize) -> Self {
        let genesis_time = 10_000_000;
        let keypairs: Vec<Keypair> = (0..validator_count)
            .collect::<Vec<usize>>()
            .iter()
            .map(|_| Keypair::random())
            .collect();
        let initial_validator_deposits = keypairs
            .iter()
            .map(|keypair| Deposit {
                branch: vec![], // branch verification is not specified.
                index: 0,       // index verification is not specified.
                deposit_data: DepositData {
                    amount: 32_000_000_000, // 32 ETH (in Gwei)
                    timestamp: genesis_time - 1,
                    deposit_input: DepositInput {
                        pubkey: keypair.pk.clone(),
                        withdrawal_credentials: Hash256::zero(), // Withdrawal not possible.
                        proof_of_possession: create_proof_of_possession(&keypair),
                    },
                },
            })
            .collect();
        let latest_eth1_data = Eth1Data {
            deposit_root: Hash256::zero(),
            block_hash: Hash256::zero(),
        };
        let spec = ChainSpec::foundation();

        Self {
            state: None,
            genesis_time,
            initial_validator_deposits,
            latest_eth1_data,
            spec,
            keypairs,
        }
    }

    /// Runs the `BeaconState::genesis` function and produces a `BeaconState`.
    pub fn genesis(&mut self) -> Result<(), BeaconStateError> {
        let state = BeaconState::genesis(
            self.genesis_time,
            self.initial_validator_deposits.clone(),
            self.latest_eth1_data.clone(),
            &self.spec,
        )?;

        self.state = Some(state);

        Ok(())
    }

    /// Sets the `BeaconState` to be in the last slot of the given epoch.
    ///
    /// Sets all justification/finalization parameters to be be as "perfect" as possible (i.e.,
    /// highest justified and finalized slots, full justification bitfield, etc).
    pub fn teleport_to_end_of_epoch(&mut self, epoch: Epoch) {
        let state = self.state.as_mut().expect("Genesis required");

        let slot = epoch.end_slot(self.spec.epoch_length);

        state.slot = slot;
        state.validator_registry_update_epoch = epoch - 1;

        state.previous_calculation_epoch = epoch - 1;
        state.current_calculation_epoch = epoch;

        state.previous_epoch_seed = Hash256::from(&b"previous_seed"[..]);
        state.current_epoch_seed = Hash256::from(&b"current_seed"[..]);

        state.previous_justified_epoch = epoch - 2;
        state.justified_epoch = epoch - 1;
        state.justification_bitfield = u64::max_value();
        state.finalized_epoch = epoch - 1;
    }

    /// Creates a full set of attestations for the `BeaconState`. Each attestation has full
    /// participation from its committee and references the expected beacon_block hashes.
    ///
    /// These attestations should be fully conducive to justification and finalization.
    pub fn insert_attestations(&mut self) {
        let state = self.state.as_mut().expect("Genesis required");

        state
            .build_epoch_cache(RelativeEpoch::Previous, &self.spec)
            .unwrap();
        state
            .build_epoch_cache(RelativeEpoch::Current, &self.spec)
            .unwrap();

        let current_epoch = state.current_epoch(&self.spec);
        let previous_epoch = state.previous_epoch(&self.spec);
        let current_epoch_depth =
            (state.slot - current_epoch.end_slot(self.spec.epoch_length)).as_usize();

        let previous_epoch_slots = previous_epoch.slot_iter(self.spec.epoch_length);
        let current_epoch_slots = current_epoch
            .slot_iter(self.spec.epoch_length)
            .take(current_epoch_depth);

        for slot in previous_epoch_slots.chain(current_epoch_slots) {
            let committees = state
                .get_crosslink_committees_at_slot(slot, &self.spec)
                .unwrap()
                .clone();

            for (committee, shard) in committees {
                state
                    .latest_attestations
                    .push(committee_to_pending_attestation(
                        state, &committee, shard, slot, &self.spec,
                    ))
            }
        }
    }

    /// Returns a cloned `BeaconState`.
    pub fn build(&self) -> Result<BeaconState, BeaconStateError> {
        match &self.state {
            Some(state) => Ok(state.clone()),
            None => panic!("Genesis required"),
        }
    }
}

/// Builds a valid PendingAttestation with full participation for some committee.
fn committee_to_pending_attestation(
    state: &BeaconState,
    committee: &Vec<usize>,
    shard: u64,
    slot: Slot,
    spec: &ChainSpec,
) -> PendingAttestation {
    let current_epoch = state.current_epoch(spec);
    let previous_epoch = state.previous_epoch(spec);

    let mut aggregation_bitfield = Bitfield::new();
    let mut custody_bitfield = Bitfield::new();

    for (i, _) in committee.iter().enumerate() {
        aggregation_bitfield.set(i, true);
        custody_bitfield.set(i, true);
    }

    let is_previous_epoch = state.slot.epoch(spec.epoch_length) != slot.epoch(spec.epoch_length);

    let justified_epoch = if is_previous_epoch {
        state.previous_justified_epoch
    } else {
        state.justified_epoch
    };

    let epoch_boundary_root = if is_previous_epoch {
        *state
            .get_block_root(previous_epoch.start_slot(spec.epoch_length), spec)
            .unwrap()
    } else {
        *state
            .get_block_root(current_epoch.start_slot(spec.epoch_length), spec)
            .unwrap()
    };

    let justified_block_root = *state
        .get_block_root(justified_epoch.start_slot(spec.epoch_length), &spec)
        .unwrap();

    PendingAttestation {
        aggregation_bitfield,
        data: AttestationData {
            slot,
            shard,
            beacon_block_root: *state.get_block_root(slot, spec).unwrap(),
            epoch_boundary_root,
            shard_block_root: Hash256::zero(),
            latest_crosslink: Crosslink {
                epoch: slot.epoch(spec.epoch_length),
                shard_block_root: Hash256::zero(),
            },
            justified_epoch,
            justified_block_root,
        },
        custody_bitfield,
        inclusion_slot: slot,
    }
}
