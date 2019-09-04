use crate::*;
use eth2_interop_keypairs::keypair;
use log::debug;
use rayon::prelude::*;

/// Generates `validator_count` keypairs where the secret key is derived solely from the index of
/// the validator.
///
/// Uses the `eth2_interop_keypairs` crate to generate keys.
pub fn generate_deterministic_keypairs(validator_count: usize) -> Vec<Keypair> {
    debug!(
        "Generating {} deterministic validator keypairs...",
        validator_count
    );

    let keypairs: Vec<Keypair> = (0..validator_count)
        .collect::<Vec<usize>>()
        .into_par_iter()
        .map(generate_deterministic_keypair)
        .collect();

    keypairs
}

/// Generates a single deterministic keypair, where the secret key is `validator_index`.
///
/// This is used for testing only, and not to be used in production!
pub fn generate_deterministic_keypair(validator_index: usize) -> Keypair {
    let raw = keypair(validator_index);
    Keypair {
        pk: PublicKey::from_raw(raw.pk),
        sk: SecretKey::from_raw(raw.sk),
    }
}
