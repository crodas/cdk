//! Choosing the keyset that signs a melt's NUT-08 change.
//!
//! Change outputs are validated against an active keyset when the melt is
//! accepted but signed only once the payment settles, so the keyset can retire,
//! expire, or rotate away in between.

use arc_swap::ArcSwap;
use cdk_common::amount::FeeAndAmounts;
use cdk_common::{CurrencyUnit, Id};
use cdk_signatory::signatory::SignatoryKeySet;

/// Keyset a melt's change outputs are signed under.
pub(super) struct ChangeSigningKeyset {
    pub id: Id,
    pub fee_and_amounts: FeeAndAmounts,
}

impl From<&SignatoryKeySet> for ChangeSigningKeyset {
    fn from(keyset: &SignatoryKeySet) -> Self {
        Self {
            id: keyset.id,
            fee_and_amounts: (keyset.input_fee_ppk, keyset.amounts.clone()).into(),
        }
    }
}

/// The keyset the change outputs were reserved on, whatever state it is in now.
///
/// Matched on id rather than active state for two reasons: the signatory keeps
/// signing a recently retired keyset, and a rotation must not swap in a
/// denomination schedule the reserved keyset has no keys for.
pub(super) fn reserved_change_keyset(
    keysets: &ArcSwap<Vec<SignatoryKeySet>>,
    reserved: Id,
) -> Option<ChangeSigningKeyset> {
    keysets
        .load()
        .iter()
        .find(|keyset| keyset.id == reserved)
        .map(Into::into)
}

/// The keyset to fall back to once the reserved one can no longer sign.
///
/// A blinded secret does not commit to a keyset, so change the reserved keyset
/// can no longer cover is signed under the unit's current active keyset rather
/// than forfeited. Ordering by fee then id mirrors how a wallet picks its own
/// active keyset and keeps concurrent finalizers on the same choice.
pub(super) fn substitute_change_keyset(
    keysets: &ArcSwap<Vec<SignatoryKeySet>>,
    unit: &CurrencyUnit,
    exclude: Id,
) -> Option<ChangeSigningKeyset> {
    keysets
        .load()
        .iter()
        .filter(|keyset| {
            keyset.active && !keyset.is_expired() && &keyset.unit == unit && keyset.id != exclude
        })
        .min_by_key(|keyset| (keyset.input_fee_ppk, keyset.id))
        .map(Into::into)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use cdk_common::nuts::Keys;
    use cdk_common::util::unix_time;

    use super::*;

    fn keyset(id: &str, unit: CurrencyUnit, active: bool, fee: u64) -> SignatoryKeySet {
        SignatoryKeySet {
            id: id.parse().expect("valid keyset id"),
            unit,
            active,
            keys: Keys::new(BTreeMap::new()),
            amounts: (0..8).map(|i| 2u64.pow(i)).collect(),
            input_fee_ppk: fee,
            final_expiry: None,
            issuer_version: None,
            version: 0,
        }
    }

    fn snapshot(keysets: Vec<SignatoryKeySet>) -> ArcSwap<Vec<SignatoryKeySet>> {
        ArcSwap::from_pointee(keysets)
    }

    fn id(value: &str) -> Id {
        value.parse().expect("valid keyset id")
    }

    #[test]
    fn reserved_keyset_resolves_regardless_of_active_state() {
        let retired = keyset("001711afb1de20cb", CurrencyUnit::Sat, false, 100);
        let active = keyset("00ad268c4d1f5826", CurrencyUnit::Sat, true, 0);
        let keysets = snapshot(vec![retired, active]);

        let resolved = reserved_change_keyset(&keysets, id("001711afb1de20cb"))
            .expect("a retired keyset is still resolvable");

        assert_eq!(resolved.id, id("001711afb1de20cb"));
        assert_eq!(resolved.fee_and_amounts.fee(), 100);
    }

    #[test]
    fn reserved_keyset_is_none_when_unknown() {
        let keysets = snapshot(vec![keyset("00ad268c4d1f5826", CurrencyUnit::Sat, true, 0)]);

        assert!(reserved_change_keyset(&keysets, id("001711afb1de20cb")).is_none());
    }

    #[test]
    fn substitute_picks_the_active_keyset_of_the_same_unit() {
        let keysets = snapshot(vec![
            keyset("001711afb1de20cb", CurrencyUnit::Sat, false, 100),
            keyset("00ad268c4d1f5826", CurrencyUnit::Sat, true, 50),
        ]);

        let resolved =
            substitute_change_keyset(&keysets, &CurrencyUnit::Sat, id("001711afb1de20cb"))
                .expect("an active sat keyset exists");

        assert_eq!(resolved.id, id("00ad268c4d1f5826"));
    }

    #[test]
    fn substitute_ignores_other_units() {
        let keysets = snapshot(vec![
            keyset("001711afb1de20cb", CurrencyUnit::Sat, false, 0),
            keyset("00ad268c4d1f5826", CurrencyUnit::Msat, true, 0),
        ]);

        assert!(
            substitute_change_keyset(&keysets, &CurrencyUnit::Sat, id("001711afb1de20cb"))
                .is_none()
        );
    }

    #[test]
    fn substitute_ignores_inactive_and_expired_keysets() {
        let mut expired = keyset("00ad268c4d1f5826", CurrencyUnit::Sat, true, 0);
        expired.final_expiry = Some(unix_time() - 1);

        let keysets = snapshot(vec![
            keyset("001711afb1de20cb", CurrencyUnit::Sat, false, 0),
            expired,
        ]);

        assert!(
            substitute_change_keyset(&keysets, &CurrencyUnit::Sat, id("001711afb1de20cb"))
                .is_none()
        );
    }

    #[test]
    fn substitute_never_returns_the_excluded_keyset() {
        let keysets = snapshot(vec![keyset("001711afb1de20cb", CurrencyUnit::Sat, true, 0)]);

        assert!(
            substitute_change_keyset(&keysets, &CurrencyUnit::Sat, id("001711afb1de20cb"))
                .is_none()
        );
    }

    /// Concurrent finalizers must land on the same substitute, so the order is
    /// cheapest fee first and keyset id as a total-order tiebreak.
    #[test]
    fn substitute_orders_by_fee_then_id() {
        let cheap_high_id = keyset("00ad268c4d1f5826", CurrencyUnit::Sat, true, 0);
        let cheap_low_id = keyset("009a1f293253e41e", CurrencyUnit::Sat, true, 0);
        let expensive = keyset("00cf4b1d1e0f2d0f", CurrencyUnit::Sat, true, 100);
        let reserved = id("001711afb1de20cb");

        let forward = snapshot(vec![
            cheap_high_id.clone(),
            cheap_low_id.clone(),
            expensive.clone(),
        ]);
        let reversed = snapshot(vec![expensive, cheap_low_id, cheap_high_id]);

        let from_forward = substitute_change_keyset(&forward, &CurrencyUnit::Sat, reserved)
            .expect("an active sat keyset exists");
        let from_reversed = substitute_change_keyset(&reversed, &CurrencyUnit::Sat, reserved)
            .expect("an active sat keyset exists");

        assert_eq!(from_forward.id, id("009a1f293253e41e"));
        assert_eq!(from_reversed.id, from_forward.id);
    }
}
