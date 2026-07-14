use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::Duration;

use cdk_signatory::signatory::RotateKeyArguments;
use tokio::sync::Notify;
use tracing::instrument;

use super::{
    CurrencyUnit, Id, KeySet, KeySetInfo, KeysResponse, KeysetResponse, Mint, MintKeySetInfo,
};
use crate::Error;

mod auth;

/// How often the mint pings the signatory for its keyset ids.
const KEYSET_POLL_INTERVAL: Duration = Duration::from_secs(60);

impl Mint {
    /// Retrieve the public keys of the active keyset for distribution to wallet
    /// clients
    #[instrument(skip(self))]
    pub fn keyset_pubkeys(&self, keyset_id: &Id) -> Result<KeysResponse, Error> {
        self.keysets
            .load()
            .iter()
            .find(|keyset| &keyset.id == keyset_id)
            .ok_or(Error::UnknownKeySet)
            .map(|key| KeysResponse {
                keysets: vec![key.into()],
            })
    }

    /// Retrieve the public keys of the active keyset for distribution to wallet
    /// clients
    #[instrument(skip_all)]
    pub fn pubkeys(&self) -> KeysResponse {
        KeysResponse {
            keysets: self
                .keysets
                .load()
                .iter()
                .filter(|keyset| keyset.active && keyset.unit != CurrencyUnit::Auth)
                .map(|key| key.into())
                .collect::<Vec<_>>(),
        }
    }

    /// Return a list of all supported keysets
    #[instrument(skip_all)]
    pub fn keysets(&self) -> KeysetResponse {
        KeysetResponse {
            keysets: self
                .keysets
                .load()
                .iter()
                .filter(|k| k.unit != CurrencyUnit::Auth)
                .map(|k| KeySetInfo {
                    id: k.id,
                    unit: k.unit.clone(),
                    active: k.active,
                    input_fee_ppk: k.input_fee_ppk,
                    final_expiry: k.final_expiry,
                })
                .collect(),
        }
    }

    /// Get keysets
    #[instrument(skip(self))]
    pub fn keyset(&self, id: &Id) -> Option<KeySet> {
        self.keysets
            .load()
            .iter()
            .find(|key| &key.id == id)
            .map(|x| x.into())
    }

    /// Add current keyset to inactive keysets
    /// Generate new keyset
    #[instrument(skip(self))]
    pub async fn rotate_keyset(
        &self,
        unit: CurrencyUnit,
        amounts: Vec<u64>,
        input_fee_ppk: u64,
        use_keyset_v2: bool,
        final_expiry: Option<u64>,
    ) -> Result<MintKeySetInfo, Error> {
        let result = self
            .signatory
            .rotate_keyset(RotateKeyArguments {
                unit,
                amounts,
                input_fee_ppk,
                keyset_id_type: if use_keyset_v2 {
                    cdk_common::nut02::KeySetVersion::Version01
                } else {
                    cdk_common::nut02::KeySetVersion::Version00
                },
                final_expiry,
            })
            .await?;

        let new_keyset = self.signatory.keysets().await?;
        self.keysets.store(new_keyset.keysets.into());

        Ok(result.into())
    }

    /// Refresh the in-memory keyset cache if the signatory reports a change.
    ///
    /// Cheap on the common path: a single ping for the signatory's keyset ids,
    /// compared as a set against the ids the mint already has cached. Only when
    /// the sets differ does it pull the full keyset set and swap the cache.
    /// Returns whether a refresh happened.
    #[instrument(skip_all)]
    pub async fn refresh_keysets_if_changed(&self) -> Result<bool, Error> {
        let remote: BTreeSet<Id> = self.signatory.keyset_ids().await?.into_iter().collect();
        let local: BTreeSet<Id> = self.keysets.load().iter().map(|k| k.id).collect();
        if remote == local {
            return Ok(false);
        }

        let keysets = self.signatory.keysets().await?;
        self.keysets.store(keysets.keysets.into());

        Ok(true)
    }

    /// Background task: poll the signatory for keyset changes.
    ///
    /// The signatory may rotate keys out of band (a remote signatory, or one
    /// shared by several mint instances). Polling a cheap fingerprint lets the
    /// mint notice and refresh its cache without shipping the full key material
    /// on every tick.
    pub(super) async fn poll_keyset_changes(mint: Arc<Self>, shutdown: Arc<Notify>) {
        let mut ticker = tokio::time::interval(KEYSET_POLL_INTERVAL);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        // The first tick fires immediately; skip it, the cache was just loaded.
        ticker.tick().await;

        loop {
            tokio::select! {
                _ = shutdown.notified() => break,
                _ = ticker.tick() => match mint.refresh_keysets_if_changed().await {
                    Ok(true) => {
                        tracing::info!("Signatory keysets changed; refreshed local cache")
                    }
                    Ok(false) => {}
                    Err(e) => tracing::warn!("Keyset fingerprint poll failed: {}", e),
                },
            }
        }

        tracing::debug!("Keyset poll task stopped");
    }
}
