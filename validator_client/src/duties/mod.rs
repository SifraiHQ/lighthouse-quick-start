use self::traits::{BeaconNode, BeaconNodeError};
use bls::PublicKey;
use slot_clock::SlotClock;
use spec::ChainSpec;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

mod service;
mod test_node;
mod traits;

#[derive(Debug, PartialEq, Clone, Copy, Default)]
pub struct EpochDuties {
    pub block_production_slot: Option<u64>,
    pub shard: Option<u64>,
}

impl EpochDuties {
    pub fn is_block_production_slot(&self, slot: u64) -> bool {
        match self.block_production_slot {
            Some(s) if s == slot => true,
            _ => false,
        }
    }
}

type EpochDutiesMap = HashMap<(PublicKey, u64), EpochDuties>;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum PollOutcome {
    NoChange,
    NewDuties,
    DutiesChanged,
    UnknownValidatorOrEpoch,
}

#[derive(Debug, PartialEq)]
pub enum Error {
    SlotClockError,
    SlotUnknowable,
    EpochMapPoisoned,
    SlotClockPoisoned,
    EpochLengthIsZero,
    BeaconNodeError(BeaconNodeError),
}

pub struct DutiesManager<T: SlotClock, U: BeaconNode> {
    pub duties_map: Arc<RwLock<EpochDutiesMap>>,
    pub pubkey: PublicKey,
    pub spec: Arc<ChainSpec>,
    pub slot_clock: Arc<RwLock<T>>,
    pub beacon_node: Arc<U>,
}

impl<T: SlotClock, U: BeaconNode> DutiesManager<T, U> {
    pub fn poll(&self) -> Result<PollOutcome, Error> {
        let slot = self
            .slot_clock
            .read()
            .map_err(|_| Error::SlotClockPoisoned)?
            .present_slot()
            .map_err(|_| Error::SlotClockError)?
            .ok_or(Error::SlotUnknowable)?;

        let epoch = slot
            .checked_div(self.spec.epoch_length)
            .ok_or(Error::EpochLengthIsZero)?;

        if let Some(duties) = self.beacon_node.request_shuffling(epoch, &self.pubkey)? {
            let mut map = self
                .duties_map
                .write()
                .map_err(|_| Error::EpochMapPoisoned)?;

            // If these duties were known, check to see if they're updates or identical.
            let result = if let Some(known_duties) = map.get(&(self.pubkey.clone(), epoch)) {
                if *known_duties == duties {
                    Ok(PollOutcome::NoChange)
                } else {
                    Ok(PollOutcome::DutiesChanged)
                }
            } else {
                Ok(PollOutcome::NewDuties)
            };
            map.insert((self.pubkey.clone(), epoch), duties);
            result
        } else {
            Ok(PollOutcome::UnknownValidatorOrEpoch)
        }
    }

}

impl From<BeaconNodeError> for Error {
    fn from(e: BeaconNodeError) -> Error {
        Error::BeaconNodeError(e)
    }
}

#[cfg(test)]
mod tests {
    use super::test_node::TestBeaconNode;
    use super::*;
    use slot_clock::TestingSlotClock;
    use bls::Keypair;

    // TODO: implement more thorough testing.
    //
    // These tests should serve as a good example for future tests.

    #[test]
    pub fn polling() {
        let spec = Arc::new(ChainSpec::foundation());
        let duties_map = Arc::new(RwLock::new(EpochDutiesMap::new()));
        let keypair = Keypair::random();
        let slot_clock = Arc::new(RwLock::new(TestingSlotClock::new(0)));
        let beacon_node = Arc::new(TestBeaconNode::default());

        let manager = DutiesManager {
            spec: spec.clone(),
            pubkey: keypair.pk.clone(),
            duties_map: duties_map.clone(),
            slot_clock: slot_clock.clone(),
            beacon_node: beacon_node.clone(),
        };

        // Configure response from the BeaconNode.
        beacon_node.set_next_shuffling_result(Ok(Some(EpochDuties{
            block_production_slot: Some(10),
            shard: Some(12),
        })));

        // Get the duties for the first time...
        assert_eq!(manager.poll(), Ok(PollOutcome::NewDuties));
        // Get the same duties again...
        assert_eq!(manager.poll(), Ok(PollOutcome::NoChange));

        // Return new duties.
        beacon_node.set_next_shuffling_result(Ok(Some(EpochDuties{
            block_production_slot: Some(11),
            shard: Some(12),
        })));
        assert_eq!(manager.poll(), Ok(PollOutcome::DutiesChanged));

        // Return no duties.
        beacon_node.set_next_shuffling_result(Ok(None));
        assert_eq!(manager.poll(), Ok(PollOutcome::UnknownValidatorOrEpoch));
    }
}
