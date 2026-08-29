use std::collections::HashMap;

use cdk_common::dhke::construct_proofs;
use cdk_common::secret::Secret;
use cdk_common::wallet::KeysetLoadPolicy;

use crate::nuts::{nut12, BlindSignature, BlindedMessage, Id, KeySet, Keys, Proofs, SecretKey};
use crate::util::unix_time;
use crate::wallet::Wallet;
use crate::{Amount, Error};

/// How strictly returned signature amounts must match requested output amounts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SignatureAmountValidation {
    /// Returned signature amounts must exactly match the requested output amounts.
    Exact,
    /// A zero-amount requested output is a placeholder and may receive any amount.
    AllowZeroAmountPlaceholder,
}

/// Whether the mint may answer with a keyset other than the one requested.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SignatureKeysetValidation {
    /// The signature must carry the keyset the wallet requested.
    Exact,
    /// The mint may sign under a different keyset of the same unit.
    ///
    /// A blinded secret does not commit to a keyset, so a mint whose original
    /// keyset can no longer sign may issue NUT-08 change under its current one
    /// rather than forfeit it. NUT-09 restore inherits the same leeway because
    /// the mint looks signatures up by blinded secret alone, so a scan of the
    /// original keyset's counters returns whatever keyset ended up signing.
    AllowSubstitution,
}

/// Validate mint-returned blind signatures against the wallet's requested outputs.
///
/// The mint controls the `amount` and `keyset_id` fields in each returned
/// [`BlindSignature`], so callers must verify those fields against the
/// corresponding premint blinded message before constructing wallet proofs.
///
/// Use [`SignatureAmountValidation::Exact`] for mint/swap responses where the
/// wallet requested a specific denomination. Use
/// [`SignatureAmountValidation::AllowZeroAmountPlaceholder`] for NUT-08/NUT-09
/// style outputs where the wallet sends amount `0` and the mint fills in the
/// actual change or restored amount. DLEQ proofs are optional for compatibility,
/// but when present they are verified after the signature metadata has been
/// cross-checked.
///
/// Returns the keys of every keyset that signed, so the caller can unblind each
/// signature against the keyset that actually produced it without loading keys
/// a second time. Pass the result to [`construct_proofs_per_keyset`].
pub(crate) async fn validate_mint_response_signatures<'a>(
    wallet: &Wallet,
    signatures: &[BlindSignature],
    blinded_messages: impl IntoIterator<Item = &'a BlindedMessage>,
    amount_validation: SignatureAmountValidation,
    keyset_validation: SignatureKeysetValidation,
    policy: KeysetLoadPolicy,
) -> Result<HashMap<Id, Keys>, Error> {
    let blinded_messages = blinded_messages.into_iter().collect::<Vec<_>>();

    if signatures.len() != blinded_messages.len() {
        return Err(Error::InvalidMintResponse(format!(
            "mint signatures ({}) does not match secrets sent ({})",
            signatures.len(),
            blinded_messages.len()
        )));
    }

    let mut keys_by_keyset: HashMap<Id, Keys> = HashMap::new();

    for (sig, blinded_message) in signatures.iter().zip(blinded_messages) {
        let amount_matches = match amount_validation {
            SignatureAmountValidation::Exact => sig.amount == blinded_message.amount,
            SignatureAmountValidation::AllowZeroAmountPlaceholder => {
                blinded_message.amount == Amount::ZERO || sig.amount == blinded_message.amount
            }
        };

        if !amount_matches {
            return Err(Error::InvalidMintResponse(format!(
                "mint signature amount ({}) does not match requested amount ({})",
                sig.amount, blinded_message.amount
            )));
        }

        if sig.keyset_id != blinded_message.keyset_id {
            if keyset_validation == SignatureKeysetValidation::Exact {
                return Err(Error::InvalidMintResponse(format!(
                    "mint signature keyset ({}) does not match requested keyset ({})",
                    sig.keyset_id, blinded_message.keyset_id
                )));
            }

            tracing::warn!(
                "Mint signed an output requested on keyset {} with keyset {}",
                blinded_message.keyset_id,
                sig.keyset_id
            );
        }

        let keys = match keys_by_keyset.get(&sig.keyset_id) {
            Some(keys) => keys,
            None => {
                let keyset = signing_keyset(wallet, sig.keyset_id, policy).await?;

                if keyset.unit != wallet.unit {
                    return Err(Error::InvalidMintResponse(format!(
                        "mint signature keyset {} is unit {}, not {}",
                        sig.keyset_id, keyset.unit, wallet.unit
                    )));
                }

                if keyset
                    .final_expiry
                    .is_some_and(|expiry| expiry < unix_time())
                {
                    return Err(Error::InvalidMintResponse(format!(
                        "mint signature keyset {} is expired",
                        sig.keyset_id
                    )));
                }

                keys_by_keyset.entry(sig.keyset_id).or_insert(keyset.keys)
            }
        };

        let key = keys.amount_key(sig.amount).ok_or(Error::AmountKey)?;
        match sig.verify_dleq(key, blinded_message.blinded_secret) {
            Ok(_) | Err(nut12::Error::MissingDleqProof) => (),
            Err(_) => return Err(Error::CouldNotVerifyDleq),
        }
    }

    Ok(keys_by_keyset)
}

/// Load the keyset that signed a response, refreshing once if it is unknown.
///
/// A substituted keyset can be one the wallet has never fetched, and a cache-only
/// caller must not fail a melt whose payment already settled just because it has
/// not seen the mint's current keyset.
async fn signing_keyset(
    wallet: &Wallet,
    keyset_id: Id,
    policy: KeysetLoadPolicy,
) -> Result<KeySet, Error> {
    match wallet.keyset_with_policy(keyset_id, policy).await {
        Err(Error::UnknownKeySet) => {
            wallet
                .keyset_with_policy(keyset_id, KeysetLoadPolicy::Refresh)
                .await
        }
        other => other,
    }
}

/// Unblind each signature with the keys of the keyset that signed it.
///
/// Outputs are requested under a single keyset, but the mint may answer under
/// another, so one [`Keys`] for the whole batch is not enough. Use the map
/// returned by [`validate_mint_response_signatures`].
pub(crate) fn construct_proofs_per_keyset(
    promises: Vec<BlindSignature>,
    rs: Vec<SecretKey>,
    secrets: Vec<Secret>,
    keys_by_keyset: &HashMap<Id, Keys>,
) -> Result<Proofs, Error> {
    if promises.len() != rs.len() || promises.len() != secrets.len() {
        return Err(Error::InvalidMintResponse(format!(
            "mismatched counts: {} signatures, {} blinding factors, {} secrets",
            promises.len(),
            rs.len(),
            secrets.len()
        )));
    }

    let mut proofs = Proofs::with_capacity(promises.len());

    for ((promise, r), secret) in promises.into_iter().zip(rs).zip(secrets) {
        let keys = keys_by_keyset
            .get(&promise.keyset_id)
            .ok_or(Error::UnknownKeySet)?;

        proofs.extend(construct_proofs(
            vec![promise],
            vec![r],
            vec![secret],
            keys,
        )?);
    }

    Ok(proofs)
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;
    use std::sync::Arc;

    use cdk_common::SecretKey;

    use super::*;
    use crate::wallet::test_utils::{
        create_test_db, create_test_wallet_with_mock, make_inactive_keyset, test_keyset,
        MockMintConnector,
    };

    async fn wallet_with_two_keysets() -> (Wallet, Id, Id) {
        let mock = Arc::new(MockMintConnector::new());
        let requested = test_keyset();
        let mut other = make_inactive_keyset();
        other.active = Some(true);
        let (requested_id, other_id) = (requested.id, other.id);
        assert_ne!(requested_id, other_id);
        *mock.keysets.lock().unwrap() = vec![requested, other];

        let wallet = create_test_wallet_with_mock(create_test_db().await, mock).await;

        (wallet, requested_id, other_id)
    }

    fn blank_output(keyset_id: Id) -> BlindedMessage {
        BlindedMessage::new(Amount::ZERO, keyset_id, SecretKey::generate().public_key())
    }

    fn signature(keyset_id: Id, amount: Amount) -> BlindSignature {
        BlindSignature {
            amount,
            keyset_id,
            c: SecretKey::generate().public_key(),
            dleq: None,
        }
    }

    /// Swap and issue pin the keyset they asked for; only change and restore
    /// tolerate the mint answering on a different one.
    #[tokio::test]
    async fn exact_rejects_a_signature_from_another_keyset() {
        let (wallet, requested, other) = wallet_with_two_keysets().await;
        let output = blank_output(requested);

        let result = validate_mint_response_signatures(
            &wallet,
            &[signature(other, Amount::from(2))],
            [&output],
            SignatureAmountValidation::AllowZeroAmountPlaceholder,
            SignatureKeysetValidation::Exact,
            Default::default(),
        )
        .await;

        assert!(matches!(result, Err(Error::InvalidMintResponse(_))));
    }

    #[tokio::test]
    async fn substitution_accepts_a_known_keyset_of_the_same_unit() {
        let (wallet, requested, other) = wallet_with_two_keysets().await;
        let output = blank_output(requested);

        let keys_by_keyset = validate_mint_response_signatures(
            &wallet,
            &[signature(other, Amount::from(2))],
            [&output],
            SignatureAmountValidation::AllowZeroAmountPlaceholder,
            SignatureKeysetValidation::AllowSubstitution,
            Default::default(),
        )
        .await
        .expect("a known same-unit keyset is a valid substitute");

        assert!(
            keys_by_keyset.contains_key(&other),
            "the caller needs the substitute's keys to unblind with"
        );
    }

    /// Substitution still has to name a keyset the wallet can resolve, or there
    /// are no keys to check the signature against.
    #[tokio::test]
    async fn substitution_rejects_an_unknown_keyset() {
        let (wallet, requested, _) = wallet_with_two_keysets().await;
        let output = blank_output(requested);
        let unknown = Id::from_str("001711afb1de20cb").expect("valid keyset id");

        let result = validate_mint_response_signatures(
            &wallet,
            &[signature(unknown, Amount::from(2))],
            [&output],
            SignatureAmountValidation::AllowZeroAmountPlaceholder,
            SignatureKeysetValidation::AllowSubstitution,
            Default::default(),
        )
        .await;

        assert!(matches!(result, Err(Error::UnknownKeySet)));
    }
}
