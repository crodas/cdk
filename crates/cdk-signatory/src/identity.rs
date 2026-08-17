//! Mint identity key
//!
//! The mint has one key that is not tied to any keyset: the identity key whose
//! public half is published as `SignatoryKeysets::pubkey` and, from there, as
//! the mint's `pubkey`. It is derived from the seed alone, so it is reproducible
//! and stable across restarts and across instances sharing one seed.
//!
//! The construction follows NUT-12's deterministic nonce derivation, the shape
//! the NUT-06 review asked for:
//!
//! ```text
//! secret_key = HMAC_SHA256(key = seed, data = "Cashu_MintIdentity_v1" || ctr)
//! ```
//!
//! with `ctr` a single counter byte starting at `0x00`, incremented and retried
//! while the digest is not a valid secp256k1 scalar.
use bitcoin::secp256k1;
use bitcoin::secp256k1::hashes::{hmac, sha256, Hash, HashEngine, HmacEngine};
use cdk_common::SecretKey;
use thiserror::Error as ThisError;

/// Purpose string separating the identity key from every other key derived from
/// the same seed. Sized and shaped like NUT-12's `Cashu_DLEQ_R_v1`.
const IDENTITY_PURPOSE: &[u8] = b"Cashu_MintIdentity_v1";

/// Identity key derivation error.
#[derive(Debug, ThisError)]
pub enum Error {
    /// No counter byte produced a scalar in `1..n-1`.
    #[error("mint identity key derivation exhausted the counter byte")]
    CounterExhausted,
}

impl From<Error> for cdk_common::Error {
    fn from(err: Error) -> Self {
        cdk_common::Error::Custom(err.to_string())
    }
}

/// HMAC the seed under `counter` and read the digest as a secp256k1 scalar.
///
/// `None` means the digest landed outside `1..n-1`, the only way a 32-byte
/// slice can be rejected, so the caller retries with the next counter instead
/// of reducing modulo `n`. Reducing would leave the derivation undefined across
/// implementations.
fn candidate(seed: &[u8], counter: u8) -> Option<SecretKey> {
    let mut engine = HmacEngine::<sha256::Hash>::new(seed);
    engine.input(IDENTITY_PURPOSE);
    engine.input(&[counter]);
    let digest = hmac::Hmac::<sha256::Hash>::from_engine(engine).to_byte_array();

    secp256k1::SecretKey::from_slice(&digest)
        .ok()
        .map(Into::into)
}

/// Derive the mint's identity key from the seed.
pub fn derive_identity_key(seed: &[u8]) -> Result<SecretKey, Error> {
    (0..=u8::MAX)
        .find_map(|counter| candidate(seed, counter))
        .ok_or(Error::CounterExhausted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derivation_matches_the_published_vector() {
        let key = derive_identity_key(b"test-seed-for-identity").expect("derive");
        assert_eq!(
            key.to_secret_hex(),
            "6f8874fe245baf7977e12b4ae0340af9c3f1d4fafe9ad9a8ab7d2ae5ee6359ad"
        );
        assert_eq!(
            key.public_key().to_hex(),
            "0288398305e49ccf96eb12b60b8e45482baff87ca3fe9cf7ca176b7770e62c63a0"
        );
    }

    /// The seed NUT-06's worked example uses, run through the layout proposed
    /// on the PR. Publishing this is what lets another implementation match.
    #[test]
    fn nut06_example_seed_vector() {
        let key = derive_identity_key(b"NUT-06 example mint seed").expect("derive");
        assert_eq!(
            key.to_secret_hex(),
            "07091030b15900b89ccbd28f837853351d2494b6f9861906ef2044a5dec3aa0b"
        );
        assert_eq!(
            key.public_key().to_hex(),
            "028816795b14f5f4eac4702acfdaa972bd3dd891eb6bc060a28feceb91f27de1d5"
        );
    }

    #[test]
    fn derivation_is_deterministic() {
        let first = derive_identity_key(b"test-seed-for-identity").expect("derive");
        let second = derive_identity_key(b"test-seed-for-identity").expect("derive");
        assert_eq!(first.to_secret_bytes(), second.to_secret_bytes());
    }

    #[test]
    fn different_seeds_derive_different_keys() {
        let first = derive_identity_key(b"seed-one").expect("derive");
        let second = derive_identity_key(b"seed-two").expect("derive");
        assert_ne!(first.to_secret_bytes(), second.to_secret_bytes());
    }

    #[test]
    fn the_first_valid_counter_wins() {
        let seed = b"test-seed-for-identity";
        let zero = candidate(seed, 0).expect("counter 0 is valid for this seed");
        let derived = derive_identity_key(seed).expect("derive");

        assert_eq!(derived.to_secret_bytes(), zero.to_secret_bytes());
    }

    #[test]
    fn each_counter_derives_a_different_key() {
        let seed = b"test-seed-for-identity";
        let zero = candidate(seed, 0).expect("counter 0");
        let one = candidate(seed, 1).expect("counter 1");

        assert_ne!(zero.to_secret_bytes(), one.to_secret_bytes());
    }

    #[test]
    fn identity_key_is_not_the_bip32_master_key() {
        use bitcoin::bip32::Xpriv;

        let seed = b"test-seed-for-identity";
        let xpriv = Xpriv::new_master(bitcoin::Network::Bitcoin, seed).expect("xpriv");
        let identity = derive_identity_key(seed).expect("derive");

        assert_ne!(
            identity.to_secret_bytes(),
            xpriv.private_key.secret_bytes(),
            "the identity key must not be the keyset derivation root"
        );
    }

    #[test]
    fn signature_verifies_against_derived_pubkey() {
        let key = derive_identity_key(b"test-seed-for-identity").expect("derive");
        let payload = b"an arbitrary stream of bytes";
        let signature = key.sign(payload).expect("sign");

        key.public_key()
            .verify(payload, &signature)
            .expect("signature should verify");
        assert!(key.public_key().verify(b"tampered", &signature).is_err());
    }
}
