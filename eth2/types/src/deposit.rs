use super::{DepositData, Hash256};
use crate::test_utils::TestRandom;
use rand::RngCore;
use serde_derive::Serialize;
use ssz::{hash, Decodable, DecodeError, Encodable, SszStream, TreeHash};

#[derive(Debug, PartialEq, Clone, Serialize)]
pub struct Deposit {
    pub branch: Vec<Hash256>,
    pub index: u64,
    pub deposit_data: DepositData,
}

impl Encodable for Deposit {
    fn ssz_append(&self, s: &mut SszStream) {
        s.append_vec(&self.branch);
        s.append(&self.index);
        s.append(&self.deposit_data);
    }
}

impl Decodable for Deposit {
    fn ssz_decode(bytes: &[u8], i: usize) -> Result<(Self, usize), DecodeError> {
        let (branch, i) = <_>::ssz_decode(bytes, i)?;
        let (index, i) = <_>::ssz_decode(bytes, i)?;
        let (deposit_data, i) = <_>::ssz_decode(bytes, i)?;

        Ok((
            Self {
                branch,
                index,
                deposit_data,
            },
            i,
        ))
    }
}

impl TreeHash for Deposit {
    fn hash_tree_root(&self) -> Vec<u8> {
        let mut result: Vec<u8> = vec![];
        result.append(&mut self.branch.hash_tree_root());
        result.append(&mut self.index.hash_tree_root());
        result.append(&mut self.deposit_data.hash_tree_root());
        hash(&result)
    }
}

impl<T: RngCore> TestRandom<T> for Deposit {
    fn random_for_test(rng: &mut T) -> Self {
        Self {
            branch: <_>::random_for_test(rng),
            index: <_>::random_for_test(rng),
            deposit_data: <_>::random_for_test(rng),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{SeedableRng, TestRandom, XorShiftRng};
    use ssz::ssz_encode;

    #[test]
    pub fn test_ssz_round_trip() {
        let mut rng = XorShiftRng::from_seed([42; 16]);
        let original = Deposit::random_for_test(&mut rng);

        let bytes = ssz_encode(&original);
        let (decoded, _) = <_>::ssz_decode(&bytes, 0).unwrap();

        assert_eq!(original, decoded);
    }

    #[test]
    pub fn test_hash_tree_root() {
        let mut rng = XorShiftRng::from_seed([42; 16]);
        let original = Deposit::random_for_test(&mut rng);

        let result = original.hash_tree_root();

        assert_eq!(result.len(), 32);
        // TODO: Add further tests
        // https://github.com/sigp/lighthouse/issues/170
    }
}
