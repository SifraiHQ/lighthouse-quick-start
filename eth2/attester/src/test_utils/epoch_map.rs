use crate::{DutiesReader, DutiesReaderError};
use std::collections::HashMap;
use types::{Epoch, Slot};

pub struct EpochMap {
    slots_per_epoch: u64,
    validator_index: Option<u64>,
    map: HashMap<Epoch, (Slot, u64)>,
}

impl EpochMap {
    pub fn new(slots_per_epoch: u64) -> Self {
        Self {
            slots_per_epoch,
            validator_index: None,
            map: HashMap::new(),
        }
    }

    pub fn insert_attestation_shard(&mut self, slot: Slot, shard: u64) {
        let epoch = slot.epoch(self.slots_per_epoch);
        self.map.insert(epoch, (slot, shard));
    }

    pub fn set_validator_index(&mut self, index: Option<u64>) {
        self.validator_index = index;
    }
}

impl DutiesReader for EpochMap {
    fn attestation_shard(&self, slot: Slot) -> Result<Option<u64>, DutiesReaderError> {
        let epoch = slot.epoch(self.slots_per_epoch);

        match self.map.get(&epoch) {
            Some((attest_slot, attest_shard)) if *attest_slot == slot => Ok(Some(*attest_shard)),
            Some((attest_slot, _attest_shard)) if *attest_slot != slot => Ok(None),
            _ => Err(DutiesReaderError::UnknownEpoch),
        }
    }

    fn validator_index(&self) -> Option<u64> {
        self.validator_index
    }
}
