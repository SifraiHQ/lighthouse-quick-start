use super::ssz::decode::{
    decode_length,
    Decodable,
};
use super::utils::hash::canonical_hash;
use super::attestation_record::MIN_SSZ_ATTESTION_RECORD_LENGTH;

#[derive(Debug, PartialEq)]
pub enum BlockValidatorError {
    TooShort,
    TooLong,
    NoAttestationRecords,
    BadPowHash,
    SlotTooLow,
    SlotTooHigh,
}

const LENGTH_BYTES: usize = 4;
const MIN_SSZ_BLOCK_LENGTH: usize = {
    32 +    // parent_hash
    8 +     // slot_number
    32 +    // randao_reveal
    LENGTH_BYTES +     // attestations (assuming zero)
    32 +    // pow_chain_ref
    32 +    // active_state_root
    32      // crystallized_state_root
};
const MAX_SSZ_BLOCK_LENGTH: usize = MIN_SSZ_BLOCK_LENGTH + (1 << 24);

/// Allows for reading of block values directly from serialized
/// ssz bytes.
#[derive(Debug, PartialEq)]
pub struct SszBlock<'a> {
    ssz: &'a [u8],
    attestation_len: usize,
    pub len: usize,
}

impl<'a> SszBlock<'a> {
    pub fn from_slice(vec: &'a [u8])
        -> Result<Self, BlockValidatorError>
    {
        let ssz = &vec[..];
        let len = vec.len();
        /*
         * Ensure the SSZ is long enough to be a block.
         */
        if len < MIN_SSZ_BLOCK_LENGTH {
            return Err(BlockValidatorError::TooShort);
        }
        /*
         * Ensure the SSZ slice isn't longer than is possible for a block.
         */
        if len > MAX_SSZ_BLOCK_LENGTH {
            return Err(BlockValidatorError::TooLong);
        }
        /*
         * Determine how many bytes are used to store attestation records
         * and ensure that length is enough to store at least one attestation
         * record.
         */
        let attestation_len = decode_length(ssz, 72, LENGTH_BYTES)
            .map_err(|_| BlockValidatorError::TooShort)?;
        if attestation_len < MIN_SSZ_ATTESTION_RECORD_LENGTH {
            return Err(BlockValidatorError::NoAttestationRecords)
        }
        /*
         * Ensure that the SSZ is long enough, now we know the
         * length of the attestation records field.
         */
        if len < (76 + attestation_len + 96) {
            return Err(BlockValidatorError::TooShort);
        }
        Ok(Self{
            ssz,
            attestation_len,
            len,
        })
    }

    pub fn block_hash(&self) -> Vec<u8> {
        canonical_hash(self.ssz)
    }

    pub fn parent_hash(&self) -> &[u8] {
        &self.ssz[0..32]
    }

    pub fn slot_number(&self) -> u64 {
        u64::ssz_decode(&self.ssz, 32).unwrap().0
    }

    pub fn randao_reveal(&self) -> &[u8] {
        &self.ssz[40..72]
    }

    pub fn attestations(&self) -> &[u8] {
        let start = 72 + LENGTH_BYTES;
        &self.ssz[start..(start + self.attestation_len)]
    }

    pub fn pow_chain_ref(&self) -> &[u8] {
        &self.ssz[(self.len - 96)..(self.len - 64)]
    }

    pub fn act_state_root(&self) -> &[u8] {
        &self.ssz[(self.len - 64)..(self.len - 32)]
    }

    pub fn cry_state_root(&self) -> &[u8] {
        &self.ssz[(self.len - 32)..(self.len)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::block::Block;
    use super::super::attestation_record::AttestationRecord;
    use super::super::ssz::SszStream;

    fn get_block_ssz(b: &Block) -> Vec<u8> {
        let mut ssz_stream = SszStream::new();
        ssz_stream.append(b);
        ssz_stream.drain()
    }

    #[test]
    fn test_ssz_block_zero_attestation_records() {
        let mut b = Block::zero();
        b.attestations = vec![];
        let ssz = get_block_ssz(&b);

        assert_eq!(
            SszBlock::from_slice(&ssz[..]),
            Err(BlockValidatorError::NoAttestationRecords)
        );
    }

    #[test]
    fn test_ssz_block_single_attestation_record() {
        let mut b = Block::zero();
        b.attestations = vec![AttestationRecord::zero()];
        let ssz = get_block_ssz(&b);

        assert_eq!(
            SszBlock::from_slice(&ssz[..]),
            Err(BlockValidatorError::NoAttestationRecords)
        );
    }
}
