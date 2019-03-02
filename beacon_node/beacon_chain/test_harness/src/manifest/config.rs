use super::yaml_helpers::{as_u64, as_usize, as_vec_u64};
use bls::create_proof_of_possession;
use types::*;
use yaml_rust::Yaml;

pub type DepositTuple = (u64, Deposit, Keypair);
pub type ProposerSlashingTuple = (u64, u64);
pub type AttesterSlashingTuple = (u64, Vec<u64>);

#[derive(Debug)]
pub struct Config {
    pub deposits_for_chain_start: usize,
    pub epoch_length: Option<u64>,
    pub num_slots: u64,
    pub skip_slots: Option<Vec<u64>>,
    pub deposits: Option<Vec<DepositTuple>>,
    pub proposer_slashings: Option<Vec<ProposerSlashingTuple>>,
    pub attester_slashings: Option<Vec<AttesterSlashingTuple>>,
}

impl Config {
    pub fn from_yaml(yaml: &Yaml) -> Self {
        Self {
            deposits_for_chain_start: as_usize(&yaml, "deposits_for_chain_start")
                .expect("Must specify validator count"),
            epoch_length: as_u64(&yaml, "epoch_length"),
            num_slots: as_u64(&yaml, "num_slots").expect("Must specify `config.num_slots`"),
            skip_slots: as_vec_u64(yaml, "skip_slots"),
            deposits: parse_deposits(&yaml),
            proposer_slashings: parse_proposer_slashings(&yaml),
            attester_slashings: parse_attester_slashings(&yaml),
        }
    }
}

fn parse_attester_slashings(yaml: &Yaml) -> Option<Vec<AttesterSlashingTuple>> {
    let mut slashings = vec![];

    for slashing in yaml["attester_slashings"].as_vec()? {
        let slot = as_u64(slashing, "slot").expect("Incomplete attester_slashing (slot)");
        let validator_indices = as_vec_u64(slashing, "validator_indices")
            .expect("Incomplete attester_slashing (validator_indices)");

        slashings.push((slot, validator_indices));
    }

    Some(slashings)
}

fn parse_proposer_slashings(yaml: &Yaml) -> Option<Vec<ProposerSlashingTuple>> {
    let mut slashings = vec![];

    for slashing in yaml["proposer_slashings"].as_vec()? {
        let slot = as_u64(slashing, "slot").expect("Incomplete proposer slashing (slot)_");
        let validator_index = as_u64(slashing, "validator_index")
            .expect("Incomplete proposer slashing (validator_index)");

        slashings.push((slot, validator_index));
    }

    Some(slashings)
}

fn parse_deposits(yaml: &Yaml) -> Option<Vec<DepositTuple>> {
    let mut deposits = vec![];

    for deposit in yaml["deposits"].as_vec()? {
        let keypair = Keypair::random();
        let proof_of_possession = create_proof_of_possession(&keypair);

        let slot = as_u64(deposit, "slot").expect("Incomplete deposit");
        let deposit = Deposit {
            branch: vec![],
            index: as_u64(deposit, "merkle_index").unwrap(),
            deposit_data: DepositData {
                amount: 32_000_000_000,
                timestamp: 1,
                deposit_input: DepositInput {
                    pubkey: keypair.pk.clone(),
                    withdrawal_credentials: Hash256::zero(),
                    proof_of_possession,
                },
            },
        };

        deposits.push((slot, deposit, keypair));
    }

    Some(deposits)
}
