//! SQLite Wallet Database

use std::collections::HashMap;
use std::fmt::Debug;
use std::str::{self, FromStr};
use std::sync::Arc;

use async_trait::async_trait;
use bitcoin::bip32::DerivationPath;
use cdk_common::database::{ConversionError, Error, WalletDatabase};
use cdk_common::mint_url::MintUrl;
use cdk_common::nuts::{MeltQuoteState, MintQuoteState};
use cdk_common::secret::Secret;
use cdk_common::util::unix_time;
use cdk_common::wallet::{
    self, MintId, MintIdentity, MintIdentityClaim, MintIdentityClaimKind, MintQuote, ProofInfo,
    Transaction, TransactionDirection, TransactionId,
};
use cdk_common::{
    database, Amount, CurrencyUnit, Id, KeySet, KeySetInfo, Keys, MintInfo, PaymentMethod, Proof,
    ProofDleq, PublicKey, SecretKey, SpendingConditions, State,
};
use tracing::instrument;
use uuid::Uuid;

use crate::common::migrate;
use crate::database::{ConnectionWithTransaction, DatabaseExecutor};
use crate::pool::{DatabasePool, Pool, PooledResource};
use crate::stmt::{query, Column};
use crate::{
    column_as_binary, column_as_nullable_binary, column_as_nullable_number,
    column_as_nullable_string, column_as_number, column_as_string, unpack_into,
};

#[rustfmt::skip]
mod migrations {
    include!(concat!(env!("OUT_DIR"), "/migrations_wallet.rs"));
}

/// Tables carrying a `mint_url` that references a mint, excluding `mint` itself.
///
/// A mint's identity has to be rewritten across all of them together; missing one
/// strands its rows under a mint the wallet no longer knows about.
const MINT_URL_TABLES: [&str; 6] = [
    "keyset",
    "mint_quote",
    "melt_quote",
    "proof",
    "transactions",
    "wallet_sagas",
];

/// The URLs a mint identity is stored under.
///
/// A URL-identified mint is its own single locator, resolved without a query so
/// that rows referencing a URL with no `mint` row still match, as they did when
/// every lookup was by URL.
async fn mint_urls_for<T>(conn: &T, mint: &MintId) -> Result<Vec<String>, Error>
where
    T: DatabaseExecutor,
{
    let pubkey = match mint {
        MintId::Url(mint_url) => return Ok(vec![mint_url.to_string()]),
        MintId::Pubkey(pubkey) => pubkey,
    };

    query(r#"SELECT mint_url FROM mint_locator WHERE pubkey = :pubkey"#)?
        .bind("pubkey", hex_pubkey(pubkey))
        .fetch_all(conn)
        .await?
        .into_iter()
        .map(|mut row| {
            Ok(column_as_string!(row
                .pop()
                .ok_or(ConversionError::MissingColumn(0, 1))?))
        })
        .collect()
}

/// Lowercase hex, the representation the pubkey-keyed tables use.
fn hex_pubkey(pubkey: &PublicKey) -> String {
    pubkey.to_hex()
}

/// Resolve a URL-shaped id to the identity that URL was promoted to.
///
/// A URL keeps reaching its mint after the mint moves onto the pubkey-keyed
/// tables, so a caller holding only a URL still gets the whole identity, which
/// after a merge is more than the rows that arrived through that one URL.
async fn canonical_mint<T>(conn: &T, mint: &MintId) -> Result<MintId, Error>
where
    T: DatabaseExecutor,
{
    let MintId::Url(mint_url) = mint else {
        return Ok(mint.clone());
    };

    match promoted_pubkey(conn, mint_url).await? {
        Some(pubkey) => Ok(MintId::Pubkey(
            PublicKey::from_hex(&pubkey).map_err(|e| Error::Internal(e.to_string()))?,
        )),
        None => Ok(mint.clone()),
    }
}

/// The `WHERE` fragment selecting the rows that belong to a mint.
///
/// A mint that has moved onto the pubkey-keyed tables carries its pubkey on
/// every row, so its rows stay found regardless of which of its URLs they were
/// written under. One that has not is still selected by URL.
fn mint_rows_clause(mint: &MintId) -> &'static str {
    match mint {
        MintId::Pubkey(_) => "mint_pubkey = :mint_pubkey",
        MintId::Url(_) => "mint_url = :mint_url",
    }
}

/// Bind whichever parameter [`mint_rows_clause`] referenced.
fn bind_mint_rows(stmt: crate::stmt::Statement, mint: &MintId) -> crate::stmt::Statement {
    match mint {
        MintId::Pubkey(pubkey) => stmt.bind("mint_pubkey", hex_pubkey(pubkey)),
        MintId::Url(mint_url) => stmt.bind("mint_url", mint_url.to_string()),
    }
}

/// The pubkey a mint URL has been promoted to, if any.
///
/// Rows written for a promoted mint carry this so they stay attached to the
/// identity rather than to the URL they happened to arrive through.
async fn promoted_pubkey<T>(conn: &T, mint_url: &MintUrl) -> Result<Option<String>, Error>
where
    T: DatabaseExecutor,
{
    query(r#"SELECT pubkey FROM mint_locator WHERE mint_url = :mint_url"#)?
        .bind("mint_url", mint_url.to_string())
        .pluck(conn)
        .await?
        .map(|pubkey| Ok(column_as_string!(pubkey)))
        .transpose()
}

/// Whether a mint was moved onto the pubkey-keyed tables.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Promotion {
    /// The mint is now identified by the pubkey it claimed.
    Applied,
    /// The claim was recorded instead of applied; the mint is unchanged.
    Deferred(MintIdentityClaimKind),
}

/// Attach every row of `mint_url` to `pubkey`.
///
/// Runs in the promoting transaction, so a mint never ends up half moved: for a
/// given mint either all its rows carry the identity or none do, which is what
/// lets a read pick one predicate rather than testing both.
async fn stamp_rows<T>(tx: &T, mint_url: &MintUrl, pubkey: &str) -> Result<(), Error>
where
    T: DatabaseExecutor,
{
    for table in MINT_URL_TABLES {
        query(&format!(
            "UPDATE {table} SET mint_pubkey = :mint_pubkey WHERE mint_url = :mint_url"
        ))?
        .bind("mint_pubkey", pubkey.to_string())
        .bind("mint_url", mint_url.to_string())
        .execute(tx)
        .await?;
    }
    Ok(())
}

/// Write the metadata for an identity, creating it if it is new.
async fn upsert_identity<T>(tx: &T, pubkey: &str, info: &MintInfo, now: u64) -> Result<(), Error>
where
    T: DatabaseExecutor,
{
    query(
        r#"
   INSERT INTO mint_identity
   (
       pubkey, name, version, description, description_long, contact, nuts,
       icon_url, urls, motd, mint_time, tos_url, first_seen, last_seen
   )
   VALUES
   (
       :pubkey, :name, :version, :description, :description_long, :contact, :nuts,
       :icon_url, :urls, :motd, :mint_time, :tos_url, :now, :now
   )
   ON CONFLICT(pubkey) DO UPDATE SET
       name = excluded.name,
       version = excluded.version,
       description = excluded.description,
       description_long = excluded.description_long,
       contact = excluded.contact,
       nuts = excluded.nuts,
       icon_url = excluded.icon_url,
       urls = excluded.urls,
       motd = excluded.motd,
       mint_time = excluded.mint_time,
       tos_url = excluded.tos_url,
       last_seen = excluded.last_seen
   ;
        "#,
    )?
    .bind("pubkey", pubkey.to_string())
    .bind("name", info.name.clone())
    .bind(
        "version",
        info.version
            .as_ref()
            .and_then(|v| serde_json::to_string(v).ok()),
    )
    .bind("description", info.description.clone())
    .bind("description_long", info.description_long.clone())
    .bind(
        "contact",
        info.contact
            .as_ref()
            .and_then(|c| serde_json::to_string(c).ok()),
    )
    .bind("nuts", serde_json::to_string(&info.nuts).ok())
    .bind("icon_url", info.icon_url.clone())
    .bind(
        "urls",
        info.urls
            .as_ref()
            .and_then(|u| serde_json::to_string(u).ok()),
    )
    .bind("motd", info.motd.clone())
    .bind("mint_time", info.time.map(|v| v as i64))
    .bind("tos_url", info.tos_url.clone())
    .bind("now", now as i64)
    .execute(tx)
    .await?;

    Ok(())
}

/// Record a URL/pubkey association the wallet saw but would not apply.
async fn record_claim<T>(
    tx: &T,
    mint_url: &MintUrl,
    pubkey: &str,
    kind: MintIdentityClaimKind,
    now: u64,
) -> Result<(), Error>
where
    T: DatabaseExecutor,
{
    let kind = match kind {
        MintIdentityClaimKind::Merge => "merge",
        MintIdentityClaimKind::Rotation => "rotation",
    };

    query(
        r#"
        INSERT INTO mint_identity_claim (mint_url, pubkey, kind, status, first_seen, last_seen)
        VALUES (:mint_url, :pubkey, :kind, 'pending', :now, :now)
        ON CONFLICT(mint_url, pubkey) DO UPDATE SET last_seen = excluded.last_seen
        "#,
    )?
    .bind("mint_url", mint_url.to_string())
    .bind("pubkey", pubkey.to_string())
    .bind("kind", kind.to_string())
    .bind("now", now as i64)
    .execute(tx)
    .await?;

    Ok(())
}

/// Link a URL to an identity and move its rows across.
async fn link_locator<T>(
    tx: &T,
    mint_url: &MintUrl,
    pubkey: &str,
    source: &str,
    now: u64,
) -> Result<(), Error>
where
    T: DatabaseExecutor,
{
    query(
        r#"
        INSERT INTO mint_locator (mint_url, pubkey, source, added_time, last_verified_time)
        VALUES (:mint_url, :pubkey, :source, :now, :now)
        ON CONFLICT(mint_url) DO UPDATE SET
            pubkey = excluded.pubkey,
            last_verified_time = excluded.last_verified_time
        "#,
    )?
    .bind("mint_url", mint_url.to_string())
    .bind("pubkey", pubkey.to_string())
    .bind("source", source.to_string())
    .bind("now", now as i64)
    .execute(tx)
    .await?;

    stamp_rows(tx, mint_url, pubkey).await?;

    // The URL-keyed row is what the mint is moving off, and a mint must appear
    // in exactly one of the two tables or `get_mints` counts it twice.
    query(r#"DELETE FROM mint WHERE mint_url = :mint_url"#)?
        .bind("mint_url", mint_url.to_string())
        .execute(tx)
        .await?;

    Ok(())
}

/// Move a mint onto the pubkey-keyed tables, applying the trust policy.
///
/// The pubkey is an unauthenticated self-assertion, so a claim that would pool
/// two URLs' funds under one identity is recorded rather than applied unless the
/// identity already advertises the newcomer among its own URLs.
async fn promote_mint_identity<T>(
    tx: &T,
    mint_url: &MintUrl,
    pubkey: &PublicKey,
    info: &MintInfo,
) -> Result<Promotion, Error>
where
    T: DatabaseExecutor,
{
    let hex = hex_pubkey(pubkey);
    let now = unix_time();
    let current = promoted_pubkey(tx, mint_url).await?;

    match current {
        // Already this identity: refresh what the mint just told us.
        Some(ref held) if *held == hex => {
            upsert_identity(tx, &hex, info, now).await?;
            link_locator(tx, mint_url, &hex, "contacted", now).await?;
            Ok(Promotion::Applied)
        }
        // This URL now claims a different key than the one it is linked to.
        Some(held) => {
            let others = locator_count(tx, &held).await?;
            if others > 1 {
                // Retagging would split an identity several URLs share, and
                // there is no way to tell which of them the new key belongs to.
                record_claim(tx, mint_url, &hex, MintIdentityClaimKind::Rotation, now).await?;
                return Ok(Promotion::Deferred(MintIdentityClaimKind::Rotation));
            }

            // Sole locator, so nothing is shared and no rows change hands.
            upsert_identity(tx, &hex, info, now).await?;
            link_locator(tx, mint_url, &hex, "contacted", now).await?;
            query(r#"DELETE FROM mint_identity WHERE pubkey = :pubkey"#)?
                .bind("pubkey", held)
                .execute(tx)
                .await?;
            Ok(Promotion::Applied)
        }
        None => {
            let incumbent_urls = identity_urls(tx, &hex).await?;

            match incumbent_urls {
                // First to claim this key.
                None => {
                    upsert_identity(tx, &hex, info, now).await?;
                    link_locator(tx, mint_url, &hex, "contacted", now).await?;
                    Ok(Promotion::Applied)
                }
                // The identity already names this URL as one of its own, which
                // is the only claim that comes from the incumbent rather than
                // from the newcomer.
                Some(urls) if urls.iter().any(|u| u == &mint_url.to_string()) => {
                    upsert_identity(tx, &hex, info, now).await?;
                    link_locator(tx, mint_url, &hex, "corroborated", now).await?;
                    Ok(Promotion::Applied)
                }
                // A new URL claiming a key someone else holds. Applying it would
                // let this mint's proofs count as the incumbent's.
                Some(_) => {
                    record_claim(tx, mint_url, &hex, MintIdentityClaimKind::Merge, now).await?;
                    Ok(Promotion::Deferred(MintIdentityClaimKind::Merge))
                }
            }
        }
    }
}

/// How many URLs resolve to an identity.
async fn locator_count<T>(tx: &T, pubkey: &str) -> Result<i64, Error>
where
    T: DatabaseExecutor,
{
    Ok(
        query(r#"SELECT COUNT(*) FROM mint_locator WHERE pubkey = :pubkey"#)?
            .bind("pubkey", pubkey.to_string())
            .pluck(tx)
            .await?
            .map(|n| Ok::<_, Error>(column_as_number!(n)))
            .transpose()?
            .unwrap_or(0),
    )
}

/// The URLs an identity advertises for itself, if it exists.
///
/// `None` distinguishes "no such identity" from "an identity that advertises
/// nothing", which is what decides whether a claim is a merge.
async fn identity_urls<T>(tx: &T, pubkey: &str) -> Result<Option<Vec<String>>, Error>
where
    T: DatabaseExecutor,
{
    let Some(row) = query(r#"SELECT urls FROM mint_identity WHERE pubkey = :pubkey"#)?
        .bind("pubkey", pubkey.to_string())
        .fetch_one(tx)
        .await?
    else {
        return Ok(None);
    };

    let urls = row
        .into_iter()
        .next()
        .and_then(|urls| match urls {
            Column::Text(text) => serde_json::from_str::<Vec<String>>(&text).ok(),
            _ => None,
        })
        .unwrap_or_default();

    Ok(Some(urls))
}

/// Wallet SQLite Database
#[derive(Debug, Clone)]
pub struct SQLWalletDatabase<RM>
where
    RM: DatabasePool + 'static,
{
    pool: Arc<Pool<RM>>,
}

impl<RM> SQLWalletDatabase<RM>
where
    RM: DatabasePool + 'static,
{
    /// Creates a new instance
    pub async fn new<X>(db: X) -> Result<Self, Error>
    where
        X: Into<RM::Config>,
    {
        let pool = Pool::new(db.into());
        Self::migrate(pool.get().await.map_err(|e| Error::Database(Box::new(e)))?).await?;

        Ok(Self { pool })
    }

    /// Migrate [`WalletSqliteDatabase`]
    async fn migrate(conn: PooledResource<RM>) -> Result<(), Error> {
        let tx = ConnectionWithTransaction::new(conn).await?;
        migrate(&tx, RM::Connection::name(), migrations::MIGRATIONS).await?;
        // Update any existing keys with missing keyset_u32 values
        Self::add_keyset_u32(&tx).await?;
        tx.commit().await?;

        Ok(())
    }

    async fn add_keyset_u32<T>(conn: &T) -> Result<(), Error>
    where
        T: DatabaseExecutor,
    {
        // First get the keysets where keyset_u32 on key is null
        let keys_without_u32: Vec<Vec<Column>> = query(
            r#"
            SELECT
                id
            FROM key
            WHERE keyset_u32 IS NULL
            "#,
        )?
        .fetch_all(conn)
        .await?;

        for row in keys_without_u32 {
            unpack_into!(let (id) = row);
            let id = column_as_string!(id);

            if let Ok(id) = Id::from_str(&id) {
                query(
                    r#"
            UPDATE
                key
            SET keyset_u32 = :u32_keyset
            WHERE id = :keyset_id
            "#,
                )?
                .bind("u32_keyset", u32::from(id))
                .bind("keyset_id", id.to_string())
                .execute(conn)
                .await?;
            }
        }

        // Also update keysets where keyset_u32 is null
        let keysets_without_u32: Vec<Vec<Column>> = query(
            r#"
            SELECT
                id
            FROM keyset
            WHERE keyset_u32 IS NULL
            "#,
        )?
        .fetch_all(conn)
        .await?;

        for row in keysets_without_u32 {
            unpack_into!(let (id) = row);
            let id = column_as_string!(id);

            if let Ok(id) = Id::from_str(&id) {
                query(
                    r#"
            UPDATE
                keyset
            SET keyset_u32 = :u32_keyset
            WHERE id = :keyset_id
            "#,
                )?
                .bind("u32_keyset", u32::from(id))
                .bind("keyset_id", id.to_string())
                .execute(conn)
                .await?;
            }
        }

        Ok(())
    }
}

#[async_trait]
impl<RM> WalletDatabase<database::Error> for SQLWalletDatabase<RM>
where
    RM: DatabasePool + 'static,
{
    #[instrument(skip(self))]
    async fn get_melt_quotes(&self) -> Result<Vec<wallet::MeltQuote>, database::Error> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| Error::Database(Box::new(e)))?;

        Ok(query(
            r#"
              SELECT
                  id,
                  unit,
                  amount,
                  request,
                  fee_reserve,
                  state,
                  expiry,
                  payment_proof,
                  payment_method,
                  estimated_blocks,
                  fee_index,
                  used_by_operation,
                  version,
                  mint_url
              FROM
                  melt_quote
              "#,
        )?
        .fetch_all(&*conn)
        .await?
        .into_iter()
        .map(sql_row_to_melt_quote)
        .collect::<Result<_, _>>()?)
    }

    #[instrument(skip(self))]
    async fn resolve_mint(&self, mint_url: &MintUrl) -> Result<Option<MintId>, database::Error> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| Error::Database(Box::new(e)))?;

        // A mint that has been promoted is identified by the pubkey it was
        // promoted to. One that has not, whether because it publishes none or
        // because the wallet has not spoken to it since, keeps its URL.
        if let Some(pubkey) = promoted_pubkey(&*conn, mint_url).await? {
            return Ok(Some(MintId::Pubkey(
                PublicKey::from_hex(&pubkey).map_err(|e| Error::Internal(e.to_string()))?,
            )));
        }

        let known = query(r#"SELECT 1 FROM mint WHERE mint_url = :mint_url"#)?
            .bind("mint_url", mint_url.to_string())
            .pluck(&*conn)
            .await?
            .is_some();

        Ok(known.then(|| MintId::Url(mint_url.clone())))
    }

    #[instrument(skip(self))]
    async fn mint_urls(&self, mint: &MintId) -> Result<Vec<MintUrl>, database::Error> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| Error::Database(Box::new(e)))?;

        Ok(mint_urls_for(&*conn, mint)
            .await?
            .into_iter()
            .filter_map(|url| MintUrl::from_str(&url).ok())
            .collect())
    }

    #[instrument(skip(self))]
    async fn get_mint(&self, mint: &MintId) -> Result<Option<MintInfo>, database::Error> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| Error::Database(Box::new(e)))?;

        let mint = &canonical_mint(&*conn, mint).await?;

        if let MintId::Pubkey(pubkey) = mint {
            return query(
                r#"
                SELECT
                    name,
                    pubkey,
                    version,
                    description,
                    description_long,
                    contact,
                    nuts,
                    icon_url,
                    motd,
                    urls,
                    mint_time,
                    tos_url
                FROM
                    mint_identity
                WHERE pubkey = :pubkey
                "#,
            )?
            .bind("pubkey", hex_pubkey(pubkey))
            .fetch_one(&*conn)
            .await?
            .map(sql_row_to_mint_info)
            .transpose();
        }

        let mint_urls = mint_urls_for(&*conn, mint).await?;
        let Some(mint_url) = mint_urls.first() else {
            return Ok(None);
        };

        Ok(query(
            r#"
            SELECT
                name,
                pubkey,
                version,
                description,
                description_long,
                contact,
                nuts,
                icon_url,
                motd,
                urls,
                mint_time,
                tos_url
            FROM
                mint
            WHERE mint_url = :mint_url
            "#,
        )?
        .bind("mint_url", mint_url.clone())
        .fetch_one(&*conn)
        .await?
        .map(sql_row_to_mint_info)
        .transpose()?)
    }

    #[instrument(skip(self))]
    async fn get_mints(&self) -> Result<HashMap<MintId, Option<MintInfo>>, database::Error> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| Error::Database(Box::new(e)))?;
        let mut mints = query(
            r#"
                SELECT
                    name,
                    pubkey,
                    version,
                    description,
                    description_long,
                    contact,
                    nuts,
                    icon_url,
                    motd,
                    urls,
                    mint_time,
                    tos_url,
                    mint_url
                FROM
                    mint
                "#,
        )?
        .fetch_all(&*conn)
        .await?
        .into_iter()
        .map(|mut row| {
            let url = column_as_string!(
                row.pop().ok_or(ConversionError::MissingColumn(0, 1))?,
                MintUrl::from_str
            );

            Ok((MintId::Url(url), sql_row_to_mint_info(row).ok()))
        })
        .collect::<Result<HashMap<_, _>, Error>>()?;

        // Promoted mints are keyed by pubkey, so a mint reachable at several
        // URLs appears once rather than once per URL.
        for mut row in query(
            r#"
                SELECT
                    name,
                    pubkey,
                    version,
                    description,
                    description_long,
                    contact,
                    nuts,
                    icon_url,
                    motd,
                    urls,
                    mint_time,
                    tos_url,
                    pubkey
                FROM
                    mint_identity
                "#,
        )?
        .fetch_all(&*conn)
        .await?
        {
            let pubkey = column_as_string!(
                row.pop().ok_or(ConversionError::MissingColumn(0, 1))?,
                PublicKey::from_hex,
                PublicKey::from_slice
            );

            mints.insert(MintId::Pubkey(pubkey), sql_row_to_mint_info(row).ok());
        }

        Ok(mints)
    }

    #[instrument(skip(self))]
    async fn get_mint_keysets(
        &self,
        mint: &MintId,
    ) -> Result<Option<Vec<KeySetInfo>>, database::Error> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| Error::Database(Box::new(e)))?;

        let mint = &canonical_mint(&*conn, mint).await?;

        let keysets = bind_mint_rows(
            query(&format!(
                r#"
            SELECT
                id,
                unit,
                active,
                input_fee_ppk,
                final_expiry
            FROM
                keyset
            WHERE {}
            "#,
                mint_rows_clause(mint)
            ))?,
            mint,
        )
        .fetch_all(&*conn)
        .await?
        .into_iter()
        .map(sql_row_to_keyset)
        .collect::<Result<Vec<_>, Error>>()?;

        match keysets.is_empty() {
            false => Ok(Some(keysets)),
            true => Ok(None),
        }
    }

    #[instrument(skip(self), fields(keyset_id = %keyset_id))]
    async fn get_keyset_by_id(
        &self,
        keyset_id: &Id,
    ) -> Result<Option<KeySetInfo>, database::Error> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| Error::Database(Box::new(e)))?;
        query(
            r#"
            SELECT
                id,
                unit,
                active,
                input_fee_ppk,
                final_expiry
            FROM
                keyset
            WHERE id = :id
            "#,
        )?
        .bind("id", keyset_id.to_string())
        .fetch_one(&*conn)
        .await?
        .map(sql_row_to_keyset)
        .transpose()
    }

    #[instrument(skip(self))]
    async fn get_mint_quote(&self, quote_id: &str) -> Result<Option<MintQuote>, database::Error> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| Error::Database(Box::new(e)))?;
        query(
            r#"
            SELECT
                id,
                mint_url,
                amount,
                unit,
                request,
                state,
                expiry,
                secret_key,
                payment_method,
                amount_issued,
                amount_paid,
                updated_at,
                estimated_blocks,
                used_by_operation,
                version
            FROM
                mint_quote
            WHERE
                id = :id
            "#,
        )?
        .bind("id", quote_id.to_string())
        .fetch_one(&*conn)
        .await?
        .map(sql_row_to_mint_quote)
        .transpose()
    }

    #[instrument(skip(self))]
    async fn get_mint_quotes(&self) -> Result<Vec<MintQuote>, database::Error> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| Error::Database(Box::new(e)))?;
        Ok(query(
            r#"
            SELECT
                id,
                mint_url,
                amount,
                unit,
                request,
                state,
                expiry,
                secret_key,
                payment_method,
                amount_issued,
                amount_paid,
                updated_at,
                estimated_blocks,
                used_by_operation,
                version
            FROM
                mint_quote
            "#,
        )?
        .fetch_all(&*conn)
        .await?
        .into_iter()
        .map(sql_row_to_mint_quote)
        .collect::<Result<_, _>>()?)
    }

    #[instrument(skip(self))]
    async fn get_unissued_mint_quotes(&self) -> Result<Vec<MintQuote>, database::Error> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| Error::Database(Box::new(e)))?;
        Ok(query(
            r#"
            SELECT
                id,
                mint_url,
                amount,
                unit,
                request,
                state,
                expiry,
                secret_key,
                payment_method,
                amount_issued,
                amount_paid,
                updated_at,
                estimated_blocks,
                used_by_operation,
                version
            FROM
                mint_quote
            WHERE
                amount_issued = 0
                OR
                payment_method = 'bolt12'
            "#,
        )?
        .fetch_all(&*conn)
        .await?
        .into_iter()
        .map(sql_row_to_mint_quote)
        .collect::<Result<_, _>>()?)
    }

    #[instrument(skip(self))]
    async fn get_melt_quote(
        &self,
        quote_id: &str,
    ) -> Result<Option<wallet::MeltQuote>, database::Error> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| Error::Database(Box::new(e)))?;
        query(
            r#"
            SELECT
                id,
                unit,
                amount,
                request,
                fee_reserve,
                state,
                expiry,
                payment_proof,
                payment_method,
                estimated_blocks,
                fee_index,
                used_by_operation,
                version,
                mint_url
            FROM
                melt_quote
            WHERE
                id=:id
            "#,
        )?
        .bind("id", quote_id.to_owned())
        .fetch_one(&*conn)
        .await?
        .map(sql_row_to_melt_quote)
        .transpose()
    }

    #[instrument(skip(self), fields(keyset_id = %keyset_id))]
    async fn get_keys(&self, keyset_id: &Id) -> Result<Option<Keys>, database::Error> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| Error::Database(Box::new(e)))?;
        query(
            r#"
            SELECT
                keys
            FROM key
            WHERE id = :id
            "#,
        )?
        .bind("id", keyset_id.to_string())
        .pluck(&*conn)
        .await?
        .map(|keys| {
            let keys = column_as_string!(keys);
            serde_json::from_str(&keys).map_err(Error::from)
        })
        .transpose()
    }

    #[instrument(skip(self, state, spending_conditions))]
    async fn get_proofs(
        &self,
        mint: Option<&MintId>,
        unit: Option<CurrencyUnit>,
        state: Option<Vec<State>>,
        spending_conditions: Option<Vec<SpendingConditions>>,
    ) -> Result<Vec<ProofInfo>, database::Error> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| Error::Database(Box::new(e)))?;

        let mint = match mint {
            Some(mint) => Some(canonical_mint(&*conn, mint).await?),
            None => None,
        };
        let mint = mint.as_ref();

        let mut sql = r#"
            SELECT
                amount,
                unit,
                keyset_id,
                secret,
                c,
                witness,
                dleq_e,
                dleq_s,
                dleq_r,
                y,
                mint_url,
                state,
                spending_condition,
                used_by_operation,
                created_by_operation,
                p2pk_e
            FROM proof
            "#
        .to_string();

        if let Some(mint) = mint {
            sql.push_str(" WHERE ");
            sql.push_str(mint_rows_clause(mint));
        }

        let mut stmt = query(&sql)?;
        if let Some(mint) = mint {
            stmt = bind_mint_rows(stmt, mint);
        }

        Ok(stmt
            .fetch_all(&*conn)
            .await?
            .into_iter()
            .filter_map(|row| {
                let row = sql_row_to_proof_info(row).ok()?;
                // The mint is already filtered in SQL, so it is not passed here.
                row.matches_conditions(&None, &unit, &state, &spending_conditions)
                    .then_some(row)
            })
            .collect::<Vec<_>>())
    }

    #[instrument(skip(self, ys))]
    async fn get_proofs_by_ys(
        &self,
        ys: Vec<PublicKey>,
    ) -> Result<Vec<ProofInfo>, database::Error> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| Error::Database(Box::new(e)))?;
        Ok(query(
            r#"
            SELECT
                amount,
                unit,
                keyset_id,
                secret,
                c,
                witness,
                dleq_e,
                dleq_s,
                dleq_r,
                y,
                mint_url,
                state,
                spending_condition,
                used_by_operation,
                created_by_operation,
                p2pk_e
            FROM proof
            WHERE y IN (:ys)
        "#,
        )?
        .bind_vec("ys", ys.iter().map(|y| y.to_bytes().to_vec()).collect())?
        .fetch_all(&*conn)
        .await?
        .into_iter()
        .filter_map(|row| sql_row_to_proof_info(row).ok())
        .collect::<Vec<_>>())
    }

    async fn get_balance(
        &self,
        mint: Option<&MintId>,
        unit: Option<CurrencyUnit>,
        states: Option<Vec<State>>,
    ) -> Result<u64, database::Error> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| Error::Database(Box::new(e)))?;

        let mint = match mint {
            Some(mint) => Some(canonical_mint(&*conn, mint).await?),
            None => None,
        };
        let mint = mint.as_ref();

        let mut query_str = "SELECT COALESCE(SUM(amount), 0) as total FROM proof".to_string();
        let mut where_clauses = Vec::new();
        let states = states
            .unwrap_or_default()
            .into_iter()
            .map(|x| x.to_string())
            .collect::<Vec<_>>();

        if let Some(mint) = mint {
            where_clauses.push(mint_rows_clause(mint));
        }
        if unit.is_some() {
            where_clauses.push("unit = :unit");
        }
        if !states.is_empty() {
            where_clauses.push("state IN (:states)");
        }

        if !where_clauses.is_empty() {
            query_str.push_str(" WHERE ");
            query_str.push_str(&where_clauses.join(" AND "));
        }

        let mut q = query(&query_str)?;

        if let Some(mint) = mint {
            q = bind_mint_rows(q, mint);
        }
        if let Some(ref unit) = unit {
            q = q.bind("unit", unit.to_string());
        }

        if !states.is_empty() {
            q = q.bind_vec("states", states)?;
        }

        let balance = q
            .pluck(&*conn)
            .await?
            .map(|n| {
                // SQLite SUM returns INTEGER which we need to convert to u64
                match n {
                    crate::stmt::Column::Integer(i) => Ok(i as u64),
                    crate::stmt::Column::Real(f) => Ok(f as u64),
                    _ => Err(Error::Database(Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "Invalid balance type",
                    )))),
                }
            })
            .transpose()?
            .unwrap_or(0);

        Ok(balance)
    }

    #[instrument(skip(self))]
    async fn get_transaction(
        &self,
        transaction_id: TransactionId,
    ) -> Result<Option<Transaction>, database::Error> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| Error::Database(Box::new(e)))?;
        Ok(query(
            r#"
            SELECT
                mint_url,
                direction,
                unit,
                amount,
                fee,
                ys,
                timestamp,
                memo,
                metadata,
                quote_id,
                payment_request,
                payment_proof,
                payment_method,
                saga_id
            FROM
                transactions
            WHERE
                id = :id
            "#,
        )?
        .bind("id", transaction_id.as_slice().to_vec())
        .fetch_one(&*conn)
        .await?
        .map(sql_row_to_transaction)
        .transpose()?)
    }

    #[instrument(skip(self))]
    async fn list_transactions(
        &self,
        mint: Option<&MintId>,
        direction: Option<TransactionDirection>,
        unit: Option<CurrencyUnit>,
    ) -> Result<Vec<Transaction>, database::Error> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| Error::Database(Box::new(e)))?;

        let mint = match mint {
            Some(mint) => Some(canonical_mint(&*conn, mint).await?),
            None => None,
        };
        let mint = mint.as_ref();

        let mut sql = r#"
            SELECT
                mint_url,
                direction,
                unit,
                amount,
                fee,
                ys,
                timestamp,
                memo,
                metadata,
                quote_id,
                payment_request,
                payment_proof,
                payment_method,
                saga_id
            FROM
                transactions
            "#
        .to_string();

        if let Some(mint) = mint {
            sql.push_str(" WHERE ");
            sql.push_str(mint_rows_clause(mint));
        }

        let mut stmt = query(&sql)?;
        if let Some(mint) = mint {
            stmt = bind_mint_rows(stmt, mint);
        }

        Ok(stmt
            .fetch_all(&*conn)
            .await?
            .into_iter()
            .filter_map(|row| {
                // TODO: Avoid a table scan by passing the heavy lifting of checking to the DB engine
                let transaction = sql_row_to_transaction(row).ok()?;
                // The mint is already filtered in SQL, so it is not passed here.
                transaction
                    .matches_conditions(&None, &direction, &unit)
                    .then_some(transaction)
            })
            .collect::<Vec<_>>())
    }

    async fn update_proofs(
        &self,
        added: Vec<ProofInfo>,
        removed_ys: Vec<PublicKey>,
    ) -> Result<(), database::Error> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| Error::Database(Box::new(e)))?;
        let tx = ConnectionWithTransaction::new(conn).await?;

        for proof in added {
            query(
                r#"
    INSERT INTO proof
    (y, mint_url, mint_pubkey, state, spending_condition, unit, amount, keyset_id, secret, c, witness, dleq_e, dleq_s, dleq_r, used_by_operation, created_by_operation, p2pk_e)
    VALUES
    (:y, :mint_url, :mint_pubkey, :state, :spending_condition, :unit, :amount, :keyset_id, :secret, :c, :witness, :dleq_e, :dleq_s, :dleq_r, :used_by_operation, :created_by_operation, :p2pk_e)
    ON CONFLICT(y) DO UPDATE SET
        mint_url = excluded.mint_url,
        mint_pubkey = excluded.mint_pubkey,
        state = excluded.state,
        spending_condition = excluded.spending_condition,
        unit = excluded.unit,
        amount = excluded.amount,
        keyset_id = excluded.keyset_id,
        secret = excluded.secret,
        c = excluded.c,
        witness = excluded.witness,
        dleq_e = excluded.dleq_e,
        dleq_s = excluded.dleq_s,
        dleq_r = excluded.dleq_r,
        used_by_operation = excluded.used_by_operation,
        created_by_operation = excluded.created_by_operation,
        p2pk_e = excluded.p2pk_e
    ;
            "#,
            )?
            .bind("y", proof.y.to_bytes().to_vec())
            .bind("mint_url", proof.mint_url.to_string())
            .bind("mint_pubkey", promoted_pubkey(&tx, &proof.mint_url).await?)
            .bind("state", proof.state.to_string())
            .bind(
                "spending_condition",
                proof
                    .spending_condition
                    .map(|s| serde_json::to_string(&s).ok()),
            )
            .bind("unit", proof.unit.to_string())
            .bind("amount", u64::from(proof.proof.amount) as i64)
            .bind("keyset_id", proof.proof.keyset_id.to_string())
            .bind("secret", proof.proof.secret.to_string())
            .bind("c", proof.proof.c.to_bytes().to_vec())
            .bind(
                "witness",
                proof
                    .proof
                    .witness
                    .and_then(|w| serde_json::to_string(&w).ok()),
            )
            .bind(
                "dleq_e",
                proof.proof.dleq.as_ref().map(|dleq| dleq.e.to_secret_bytes().to_vec()),
            )
            .bind(
                "dleq_s",
                proof.proof.dleq.as_ref().map(|dleq| dleq.s.to_secret_bytes().to_vec()),
            )
            .bind(
                "dleq_r",
                proof.proof.dleq.as_ref().map(|dleq| dleq.r.to_secret_bytes().to_vec()),
            )
            .bind("used_by_operation", proof.used_by_operation.map(|id| id.to_string()))
            .bind("created_by_operation", proof.created_by_operation.map(|id| id.to_string()))
            .bind(
                "p2pk_e",
                proof
                    .proof
                    .p2pk_e
                    .as_ref()
                    .map(|pk| pk.to_bytes().to_vec()),
            )
            .execute(&tx)
            .await?;
        }

        if !removed_ys.is_empty() {
            query(r#"DELETE FROM proof WHERE y IN (:ys)"#)?
                .bind_vec(
                    "ys",
                    removed_ys.iter().map(|y| y.to_bytes().to_vec()).collect(),
                )?
                .execute(&tx)
                .await?;
        }

        tx.commit().await?;

        Ok(())
    }

    #[instrument(skip(self))]
    async fn update_proofs_state(
        &self,
        ys: Vec<PublicKey>,
        state: State,
    ) -> Result<(), database::Error> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| Error::Database(Box::new(e)))?;

        query("UPDATE proof SET state = :state WHERE y IN (:ys)")?
            .bind_vec("ys", ys.iter().map(|y| y.to_bytes().to_vec()).collect())?
            .bind("state", state.to_string())
            .execute(&*conn)
            .await?;

        Ok(())
    }

    #[instrument(skip(self))]
    async fn add_transaction(&self, transaction: Transaction) -> Result<(), database::Error> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| Error::Database(Box::new(e)))?;

        let mint_pubkey = promoted_pubkey(&*conn, &transaction.mint_url).await?;
        let mint_url = transaction.mint_url.to_string();
        let direction = transaction.direction.to_string();
        let unit = transaction.unit.to_string();
        let amount = u64::from(transaction.amount) as i64;
        let fee = u64::from(transaction.fee) as i64;
        let ys = transaction
            .ys
            .iter()
            .flat_map(|y| y.to_bytes().to_vec())
            .collect::<Vec<_>>();

        let id = transaction.id();

        query(
               r#"
   INSERT INTO transactions
   (id, mint_url, mint_pubkey, direction, unit, amount, fee, ys, timestamp, memo, metadata, quote_id, payment_request, payment_proof, payment_method, saga_id)
   VALUES
   (:id, :mint_url, :mint_pubkey, :direction, :unit, :amount, :fee, :ys, :timestamp, :memo, :metadata, :quote_id, :payment_request, :payment_proof, :payment_method, :saga_id)
   ON CONFLICT(id) DO UPDATE SET
       mint_url = excluded.mint_url,
       mint_pubkey = excluded.mint_pubkey,
       direction = excluded.direction,
       unit = excluded.unit,
       amount = excluded.amount,
       fee = excluded.fee,
       timestamp = excluded.timestamp,
       memo = excluded.memo,
       metadata = excluded.metadata,
       quote_id = excluded.quote_id,
       payment_request = excluded.payment_request,
       payment_proof = excluded.payment_proof,
       payment_method = excluded.payment_method,
       saga_id = excluded.saga_id
   ;
           "#,
           )?
           .bind("id", id.as_slice().to_vec())
           .bind("mint_url", mint_url)
           .bind("mint_pubkey", mint_pubkey)
           .bind("direction", direction)
           .bind("unit", unit)
           .bind("amount", amount)
           .bind("fee", fee)
           .bind("ys", ys)
           .bind("timestamp", transaction.timestamp as i64)
           .bind("memo", transaction.memo)
           .bind(
               "metadata",
               serde_json::to_string(&transaction.metadata).map_err(Error::from)?,
           )
           .bind("quote_id", transaction.quote_id)
           .bind("payment_request", transaction.payment_request)
           .bind("payment_proof", transaction.payment_proof)
           .bind("payment_method", transaction.payment_method.map(|pm| pm.to_string()))
           .bind("saga_id", transaction.saga_id.map(|id| id.to_string()))
           .execute(&*conn)
           .await?;

        Ok(())
    }

    #[instrument(skip(self))]
    async fn update_mint_url(
        &self,
        old_mint_url: MintUrl,
        new_mint_url: MintUrl,
    ) -> Result<(), database::Error> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| Error::Database(Box::new(e)))?;
        let tx = ConnectionWithTransaction::new(conn).await?;

        // The mint row must move too, otherwise the mint keeps answering under
        // its old URL and every dependent row is orphaned. A promoted mint has
        // no such row; it is its locator that moves.
        for table in ["mint", "mint_locator", "mint_identity_claim"] {
            query(&format!(
                "UPDATE {table} SET mint_url = :new_mint_url WHERE mint_url = :old_mint_url"
            ))?
            .bind("new_mint_url", new_mint_url.to_string())
            .bind("old_mint_url", old_mint_url.to_string())
            .execute(&tx)
            .await?;
        }

        for table in MINT_URL_TABLES {
            query(&format!(
                r#"
                UPDATE {table}
                SET mint_url = :new_mint_url
                WHERE mint_url = :old_mint_url
            "#
            ))?
            .bind("new_mint_url", new_mint_url.to_string())
            .bind("old_mint_url", old_mint_url.to_string())
            .execute(&tx)
            .await?;
        }

        tx.commit().await?;

        Ok(())
    }

    #[instrument(skip(self), fields(keyset_id = %keyset_id))]
    async fn increment_keyset_counter(
        &self,
        keyset_id: &Id,
        count: u32,
    ) -> Result<u32, database::Error> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| Error::Database(Box::new(e)))?;

        let new_counter = query(
            r#"
            INSERT INTO keyset_counter (keyset_id, counter)
            VALUES (:keyset_id, :count)
            ON CONFLICT(keyset_id) DO UPDATE SET
                counter = keyset_counter.counter + :count
            RETURNING counter
            "#,
        )?
        .bind("keyset_id", keyset_id.to_string())
        .bind("count", count)
        .pluck(&*conn)
        .await?
        .map(|n| Ok::<_, Error>(column_as_number!(n)))
        .transpose()?
        .ok_or_else(|| Error::Internal("Counter update returned no value".to_owned()))?;

        Ok(new_counter)
    }

    #[instrument(skip(self, mint_info))]
    async fn add_mint(
        &self,
        mint_url: MintUrl,
        mint_info: Option<MintInfo>,
    ) -> Result<(), database::Error> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| Error::Database(Box::new(e)))?;
        let tx = ConnectionWithTransaction::new(conn).await?;

        // A mint that tells us a pubkey moves onto the pubkey-keyed tables, and
        // stays there. Everything else keeps its URL-keyed row.
        if let Some(info) = mint_info.as_ref() {
            if let Some(pubkey) = info.pubkey {
                if promote_mint_identity(&tx, &mint_url, &pubkey, info).await? == Promotion::Applied
                {
                    tx.commit().await?;
                    return Ok(());
                }

                tracing::warn!(
                    "{mint_url} claims pubkey {pubkey}, which is held elsewhere; recorded the \
                     claim rather than merging the two"
                );
            }
        }

        // Never resurrect a URL-keyed row for a mint that has already moved:
        // a mint must appear in exactly one of the two tables.
        if promoted_pubkey(&tx, &mint_url).await?.is_some() {
            if let Some(info) = mint_info.as_ref() {
                if let Some(pubkey) = info.pubkey {
                    upsert_identity(&tx, &hex_pubkey(&pubkey), info, unix_time()).await?;
                }
            }
            tx.commit().await?;
            return Ok(());
        }

        let (
            name,
            pubkey,
            version,
            description,
            description_long,
            contact,
            nuts,
            icon_url,
            urls,
            motd,
            time,
            tos_url,
        ) = match mint_info {
            Some(mint_info) => {
                let MintInfo {
                    name,
                    pubkey,
                    version,
                    description,
                    description_long,
                    contact,
                    nuts,
                    icon_url,
                    urls,
                    motd,
                    time,
                    tos_url,
                } = mint_info;

                (
                    name,
                    pubkey.map(|p| p.to_bytes().to_vec()),
                    version.map(|v| serde_json::to_string(&v).ok()),
                    description,
                    description_long,
                    contact.map(|c| serde_json::to_string(&c).ok()),
                    serde_json::to_string(&nuts).ok(),
                    icon_url,
                    urls.map(|c| serde_json::to_string(&c).ok()),
                    motd,
                    time,
                    tos_url,
                )
            }
            None => (
                None, None, None, None, None, None, None, None, None, None, None, None,
            ),
        };

        query(
            r#"
   INSERT INTO mint
   (
       mint_url, name, pubkey, version, description, description_long,
       contact, nuts, icon_url, urls, motd, mint_time, tos_url
   )
   VALUES
   (
       :mint_url, :name, :pubkey, :version, :description, :description_long,
       :contact, :nuts, :icon_url, :urls, :motd, :mint_time, :tos_url
   )
   ON CONFLICT(mint_url) DO UPDATE SET
       name = excluded.name,
       pubkey = excluded.pubkey,
       version = excluded.version,
       description = excluded.description,
       description_long = excluded.description_long,
       contact = excluded.contact,
       nuts = excluded.nuts,
       icon_url = excluded.icon_url,
       urls = excluded.urls,
       motd = excluded.motd,
       mint_time = excluded.mint_time,
       tos_url = excluded.tos_url
   ;
           "#,
        )?
        .bind("mint_url", mint_url.to_string())
        .bind("name", name)
        .bind("pubkey", pubkey)
        .bind("version", version)
        .bind("description", description)
        .bind("description_long", description_long)
        .bind("contact", contact)
        .bind("nuts", nuts)
        .bind("icon_url", icon_url)
        .bind("urls", urls)
        .bind("motd", motd)
        .bind("mint_time", time.map(|v| v as i64))
        .bind("tos_url", tos_url)
        .execute(&tx)
        .await?;

        tx.commit().await?;

        Ok(())
    }

    #[instrument(skip(self))]
    async fn remove_mint(&self, mint: &MintId) -> Result<(), database::Error> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| Error::Database(Box::new(e)))?;

        let mint_urls = mint_urls_for(&*conn, mint).await?;
        if mint_urls.is_empty() {
            return Ok(());
        }

        let tx = ConnectionWithTransaction::new(conn).await?;

        // Deleted explicitly rather than by cascade: the mint_url foreign key was
        // removed so that a mint can be identified by pubkey instead.
        query(r#"DELETE FROM keyset WHERE mint_url IN (:mint_urls)"#)?
            .bind_vec("mint_urls", mint_urls.clone())?
            .execute(&tx)
            .await?;

        query(r#"DELETE FROM mint WHERE mint_url IN (:mint_urls)"#)?
            .bind_vec("mint_urls", mint_urls.clone())?
            .execute(&tx)
            .await?;

        query(r#"DELETE FROM mint_locator WHERE mint_url IN (:mint_urls)"#)?
            .bind_vec("mint_urls", mint_urls.clone())?
            .execute(&tx)
            .await?;

        query(r#"DELETE FROM mint_identity_claim WHERE mint_url IN (:mint_urls)"#)?
            .bind_vec("mint_urls", mint_urls)?
            .execute(&tx)
            .await?;

        // An identity nothing reaches any more is not a mint the wallet knows.
        query(
            r#"DELETE FROM mint_identity
               WHERE pubkey NOT IN (SELECT pubkey FROM mint_locator)"#,
        )?
        .execute(&tx)
        .await?;

        tx.commit().await?;

        Ok(())
    }

    #[instrument(skip(self, keysets))]
    async fn add_mint_keysets(
        &self,
        mint: &MintId,
        keysets: Vec<KeySetInfo>,
    ) -> Result<(), database::Error> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| Error::Database(Box::new(e)))?;

        // Keysets carry the identity, but also a URL, since a mint that has not
        // been promoted has nothing else to be found by. A pubkey-identified
        // mint records them under the first URL it answers on.
        let mint_url = mint_urls_for(&*conn, mint)
            .await?
            .into_iter()
            .next()
            .ok_or_else(|| Error::Internal(format!("unknown mint {mint}")))?;
        let mint_pubkey = mint.pubkey().map(hex_pubkey);

        let tx = ConnectionWithTransaction::new(conn).await?;

        for keyset in keysets {
            query(
                r#"
        INSERT INTO keyset
        (mint_url, mint_pubkey, id, unit, active, input_fee_ppk, final_expiry, keyset_u32)
        VALUES
        (:mint_url, :mint_pubkey, :id, :unit, :active, :input_fee_ppk, :final_expiry, :keyset_u32)
        ON CONFLICT(id) DO UPDATE SET
            mint_url = excluded.mint_url,
            mint_pubkey = excluded.mint_pubkey,
            active = excluded.active,
            input_fee_ppk = excluded.input_fee_ppk
        "#,
            )?
            .bind("mint_url", mint_url.clone())
            .bind("mint_pubkey", mint_pubkey.clone())
            .bind("id", keyset.id.to_string())
            .bind("unit", keyset.unit.to_string())
            .bind("active", keyset.active)
            .bind("input_fee_ppk", keyset.input_fee_ppk as i64)
            .bind("final_expiry", keyset.final_expiry.map(|v| v as i64))
            .bind("keyset_u32", u32::from(keyset.id))
            .execute(&tx)
            .await?;
        }

        tx.commit().await?;

        Ok(())
    }

    #[instrument(skip_all)]
    async fn add_mint_quote(&self, quote: MintQuote) -> Result<(), database::Error> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| Error::Database(Box::new(e)))?;

        let expected_version = quote.version;
        let new_version = expected_version.wrapping_add(1);

        let rows_affected = query(
                r#"
    INSERT INTO mint_quote
    (id, mint_url, mint_pubkey, amount, unit, request, state, expiry, secret_key, payment_method, amount_issued, amount_paid, updated_at, estimated_blocks, version, used_by_operation)
    VALUES
    (:id, :mint_url, :mint_pubkey, :amount, :unit, :request, :state, :expiry, :secret_key, :payment_method, :amount_issued, :amount_paid, :updated_at, :estimated_blocks, :version, :used_by_operation)
    ON CONFLICT(id) DO UPDATE SET
        mint_url = excluded.mint_url,
        mint_pubkey = excluded.mint_pubkey,
        amount = excluded.amount,
        unit = excluded.unit,
        request = excluded.request,
        state = excluded.state,
        expiry = excluded.expiry,
        secret_key = excluded.secret_key,
        payment_method = excluded.payment_method,
        amount_issued = excluded.amount_issued,
        amount_paid = excluded.amount_paid,
        updated_at = excluded.updated_at,
        estimated_blocks = excluded.estimated_blocks,
        version = :new_version,
        used_by_operation = excluded.used_by_operation
    WHERE mint_quote.version = :expected_version
    ;
            "#,
            )?
            .bind("id", quote.id.to_string())
            .bind("mint_url", quote.mint_url.to_string())
            .bind("mint_pubkey", promoted_pubkey(&*conn, &quote.mint_url).await?)
            .bind("amount", quote.amount.map(|a| a.to_i64()))
            .bind("unit", quote.unit.to_string())
            .bind("request", quote.request)
            .bind("state", quote.state.to_string())
            .bind("expiry", quote.expiry as i64)
            .bind("secret_key", quote.secret_key.map(|p| p.to_string()))
            .bind("payment_method", quote.payment_method.to_string())
            .bind("amount_issued", quote.amount_issued.to_i64())
            .bind("amount_paid", quote.amount_paid.to_i64())
            .bind("updated_at", quote.updated_at as i64)
            .bind("estimated_blocks", quote.estimated_blocks.map(i64::from))
            .bind("version", quote.version as i64)
            .bind("new_version", new_version as i64)
            .bind("expected_version", expected_version as i64)
            .bind("used_by_operation", quote.used_by_operation)
            .execute(&*conn).await?;

        if rows_affected == 0 {
            return Err(database::Error::ConcurrentUpdate);
        }

        Ok(())
    }

    #[instrument(skip(self))]
    async fn remove_mint_quote(&self, quote_id: &str) -> Result<(), database::Error> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| Error::Database(Box::new(e)))?;

        query(r#"DELETE FROM mint_quote WHERE id=:id"#)?
            .bind("id", quote_id.to_string())
            .execute(&*conn)
            .await?;

        Ok(())
    }

    #[instrument(skip_all)]
    async fn add_melt_quote(&self, quote: wallet::MeltQuote) -> Result<(), database::Error> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| Error::Database(Box::new(e)))?;

        let expected_version = quote.version;
        let new_version = expected_version.wrapping_add(1);

        let mint_pubkey = match quote.mint_url.as_ref() {
            Some(mint_url) => promoted_pubkey(&*conn, mint_url).await?,
            None => None,
        };

        let rows_affected = query(
            r#"
 INSERT INTO melt_quote
 (id, unit, amount, request, fee_reserve, state, expiry, payment_proof, payment_method, estimated_blocks, fee_index, version, mint_url, mint_pubkey, used_by_operation)
 VALUES
 (:id, :unit, :amount, :request, :fee_reserve, :state, :expiry, :payment_proof, :payment_method, :estimated_blocks, :fee_index, :version, :mint_url, :mint_pubkey, :used_by_operation)
 ON CONFLICT(id) DO UPDATE SET
     unit = excluded.unit,
     amount = excluded.amount,
     request = excluded.request,
     fee_reserve = excluded.fee_reserve,
     state = excluded.state,
     expiry = excluded.expiry,
     payment_proof = COALESCE(excluded.payment_proof, melt_quote.payment_proof),
     payment_method = excluded.payment_method,
     estimated_blocks = excluded.estimated_blocks,
     fee_index = excluded.fee_index,
     version = :new_version,
     mint_url = excluded.mint_url,
     mint_pubkey = excluded.mint_pubkey,
     used_by_operation = excluded.used_by_operation
 WHERE melt_quote.version = :expected_version
 ;
         "#,
        )?
        .bind("id", quote.id.to_string())
        .bind("unit", quote.unit.to_string())
        .bind("amount", u64::from(quote.amount) as i64)
        .bind("request", quote.request)
        .bind("fee_reserve", u64::from(quote.fee_reserve) as i64)
        .bind("state", quote.state.to_string())
        .bind("expiry", quote.expiry as i64)
        .bind("payment_proof", quote.payment_proof)
        .bind("payment_method", quote.payment_method.to_string())
        .bind("estimated_blocks", quote.estimated_blocks.map(i64::from))
        .bind("fee_index", quote.fee_index.map(i64::from))
        .bind("version", quote.version as i64)
        .bind("new_version", new_version as i64)
        .bind("expected_version", expected_version as i64)
        .bind("mint_pubkey", mint_pubkey)
        .bind("mint_url", quote.mint_url.map(|m| m.to_string()))
        .bind("used_by_operation", quote.used_by_operation)
        .execute(&*conn)
        .await?;

        if rows_affected == 0 {
            return Err(database::Error::ConcurrentUpdate);
        }

        Ok(())
    }

    #[instrument(skip(self))]
    async fn remove_melt_quote(&self, quote_id: &str) -> Result<(), database::Error> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| Error::Database(Box::new(e)))?;

        query(r#"DELETE FROM melt_quote WHERE id=:id"#)?
            .bind("id", quote_id.to_owned())
            .execute(&*conn)
            .await?;

        Ok(())
    }

    #[instrument(skip_all)]
    async fn add_keys(&self, keyset: KeySet) -> Result<(), database::Error> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| Error::Database(Box::new(e)))?;

        keyset.verify_id()?;

        query(
            r#"
                INSERT INTO key
                (id, keys, keyset_u32)
                VALUES
                (:id, :keys, :keyset_u32)
            "#,
        )?
        .bind("id", keyset.id.to_string())
        .bind(
            "keys",
            serde_json::to_string(&keyset.keys).map_err(Error::from)?,
        )
        .bind("keyset_u32", u32::from(keyset.id))
        .execute(&*conn)
        .await?;

        Ok(())
    }

    #[instrument(skip(self))]
    async fn remove_keys(&self, id: &Id) -> Result<(), database::Error> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| Error::Database(Box::new(e)))?;

        query(r#"DELETE FROM key WHERE id = :id"#)?
            .bind("id", id.to_string())
            .execute(&*conn)
            .await?;

        Ok(())
    }

    #[instrument(skip(self))]
    async fn remove_transaction(
        &self,
        transaction_id: TransactionId,
    ) -> Result<(), database::Error> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| Error::Database(Box::new(e)))?;

        query(r#"DELETE FROM transactions WHERE id=:id"#)?
            .bind("id", transaction_id.as_slice().to_vec())
            .execute(&*conn)
            .await?;

        Ok(())
    }

    #[instrument(skip(self))]
    async fn add_saga(&self, saga: wallet::WalletSaga) -> Result<(), database::Error> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| Error::Database(Box::new(e)))?;

        let state_json = serde_json::to_string(&saga.state).map_err(|e| {
            Error::Database(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Failed to serialize saga state: {}", e),
            )))
        })?;

        let data_json = serde_json::to_string(&saga.data).map_err(|e| {
            Error::Database(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Failed to serialize saga data: {}", e),
            )))
        })?;

        query(
            r#"
            INSERT INTO wallet_sagas
            (id, kind, state, amount, mint_url, mint_pubkey, unit, quote_id, created_at, updated_at, data, version)
            VALUES
            (:id, :kind, :state, :amount, :mint_url, :mint_pubkey, :unit, :quote_id, :created_at, :updated_at, :data, :version)
            "#,
        )?
        .bind("id", saga.id.to_string())
        .bind("kind", saga.kind.to_string())
        .bind("state", state_json)
        .bind("amount", u64::from(saga.amount) as i64)
        .bind("mint_url", saga.mint_url.to_string())
        .bind("mint_pubkey", promoted_pubkey(&*conn, &saga.mint_url).await?)
        .bind("unit", saga.unit.to_string())
        .bind("quote_id", saga.quote_id)
        .bind("created_at", saga.created_at as i64)
        .bind("updated_at", saga.updated_at as i64)
        .bind("data", data_json)
        .bind("version", saga.version as i64)
        .execute(&*conn)
        .await?;

        Ok(())
    }

    #[instrument(skip(self))]
    async fn get_saga(
        &self,
        id: &uuid::Uuid,
    ) -> Result<Option<wallet::WalletSaga>, database::Error> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| Error::Database(Box::new(e)))?;

        let rows = query(
            r#"
            SELECT id, kind, state, amount, mint_url, unit, quote_id, created_at, updated_at, data, version
            FROM wallet_sagas
            WHERE id = :id
            "#,
        )?
        .bind("id", id.to_string())
        .fetch_all(&*conn)
        .await?;

        match rows.into_iter().next() {
            Some(row) => Ok(Some(sql_row_to_wallet_saga(row)?)),
            None => Ok(None),
        }
    }

    #[instrument(skip(self))]
    async fn update_saga(&self, saga: wallet::WalletSaga) -> Result<bool, database::Error> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| Error::Database(Box::new(e)))?;

        let state_json = serde_json::to_string(&saga.state).map_err(|e| {
            Error::Database(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Failed to serialize saga state: {}", e),
            )))
        })?;

        let data_json = serde_json::to_string(&saga.data).map_err(|e| {
            Error::Database(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Failed to serialize saga data: {}", e),
            )))
        })?;

        // Optimistic locking: only update if the version matches the expected value.
        // The saga.version has already been incremented by the caller, so we check
        // for (saga.version - 1) in the WHERE clause.
        let expected_version = saga.version.saturating_sub(1);

        let rows_affected = query(
            r#"
            UPDATE wallet_sagas
            SET kind = :kind, state = :state, amount = :amount, mint_url = :mint_url,
                unit = :unit, quote_id = :quote_id, updated_at = :updated_at, data = :data,
                version = :new_version
            WHERE id = :id AND version = :expected_version
            "#,
        )?
        .bind("id", saga.id.to_string())
        .bind("kind", saga.kind.to_string())
        .bind("state", state_json)
        .bind("amount", u64::from(saga.amount) as i64)
        .bind("mint_url", saga.mint_url.to_string())
        .bind("unit", saga.unit.to_string())
        .bind("quote_id", saga.quote_id)
        .bind("updated_at", saga.updated_at as i64)
        .bind("data", data_json)
        .bind("new_version", saga.version as i64)
        .bind("expected_version", expected_version as i64)
        .execute(&*conn)
        .await?;

        // Return true if the update succeeded (version matched), false if version mismatch
        Ok(rows_affected > 0)
    }

    #[instrument(skip(self))]
    async fn delete_saga(&self, id: &uuid::Uuid) -> Result<(), database::Error> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| Error::Database(Box::new(e)))?;

        query(r#"DELETE FROM wallet_sagas WHERE id = :id"#)?
            .bind("id", id.to_string())
            .execute(&*conn)
            .await?;

        Ok(())
    }

    #[instrument(skip(self))]
    async fn get_incomplete_sagas(&self) -> Result<Vec<wallet::WalletSaga>, database::Error> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| Error::Database(Box::new(e)))?;

        let rows = query(
            r#"
            SELECT id, kind, state, amount, mint_url, unit, quote_id, created_at, updated_at, data, version
            FROM wallet_sagas
            ORDER BY created_at ASC
            "#,
        )?
        .fetch_all(&*conn)
        .await?;

        rows.into_iter().map(sql_row_to_wallet_saga).collect()
    }

    #[instrument(skip(self))]
    async fn reserve_proofs(
        &self,
        ys: Vec<PublicKey>,
        operation_id: &uuid::Uuid,
    ) -> Result<(), database::Error> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| Error::Database(Box::new(e)))?;

        if ys.is_empty() {
            return Ok(());
        }

        let expected = ys.len();
        let tx = ConnectionWithTransaction::new(conn).await?;

        let rows_affected = query(
            r#"
            UPDATE proof
            SET state = 'RESERVED', used_by_operation = :operation_id
            WHERE y IN (:ys) AND state = 'UNSPENT'
            "#,
        )?
        .bind_vec("ys", ys.iter().map(|y| y.to_bytes().to_vec()).collect())?
        .bind("operation_id", operation_id.to_string())
        .execute(&tx)
        .await?;

        // Reserving is all-or-nothing: a partial match must not leave some of
        // the proofs reserved.
        if rows_affected != expected {
            tx.rollback().await?;
            return Err(database::Error::ProofNotUnspent);
        }

        tx.commit().await?;

        Ok(())
    }

    #[instrument(skip(self))]
    async fn release_proofs(&self, operation_id: &uuid::Uuid) -> Result<(), database::Error> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| Error::Database(Box::new(e)))?;

        query(
            r#"
            UPDATE proof
            SET state = 'UNSPENT', used_by_operation = NULL
            WHERE used_by_operation = :operation_id
            "#,
        )?
        .bind("operation_id", operation_id.to_string())
        .execute(&*conn)
        .await?;

        Ok(())
    }

    #[instrument(skip(self))]
    async fn get_reserved_proofs(
        &self,
        operation_id: &uuid::Uuid,
    ) -> Result<Vec<ProofInfo>, database::Error> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| Error::Database(Box::new(e)))?;

        let rows = query(
            r#"
            SELECT
                amount,
                unit,
                keyset_id,
                secret,
                c,
                witness,
                dleq_e,
                dleq_s,
                dleq_r,
                y,
                mint_url,
                state,
                spending_condition,
                used_by_operation,
                created_by_operation,
                p2pk_e
            FROM proof
            WHERE used_by_operation = :operation_id
            "#,
        )?
        .bind("operation_id", operation_id.to_string())
        .fetch_all(&*conn)
        .await?;

        rows.into_iter().map(sql_row_to_proof_info).collect()
    }

    #[instrument(skip(self))]
    async fn reserve_melt_quote(
        &self,
        quote_id: &str,
        operation_id: &uuid::Uuid,
    ) -> Result<(), database::Error> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| Error::Database(Box::new(e)))?;

        let rows_affected = query(
            r#"
            UPDATE melt_quote
            SET used_by_operation = :operation_id
            WHERE id = :quote_id AND used_by_operation IS NULL
            "#,
        )?
        .bind("operation_id", operation_id.to_string())
        .bind("quote_id", quote_id)
        .execute(&*conn)
        .await?;

        if rows_affected == 0 {
            // Check if the quote exists
            let exists = query(
                r#"
                SELECT 1 FROM melt_quote WHERE id = :quote_id
                "#,
            )?
            .bind("quote_id", quote_id)
            .fetch_one(&*conn)
            .await?;

            if exists.is_none() {
                return Err(database::Error::UnknownQuote);
            }
            return Err(database::Error::QuoteAlreadyInUse);
        }

        Ok(())
    }

    #[instrument(skip(self))]
    async fn release_melt_quote(&self, operation_id: &uuid::Uuid) -> Result<(), database::Error> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| Error::Database(Box::new(e)))?;

        query(
            r#"
            UPDATE melt_quote
            SET used_by_operation = NULL
            WHERE used_by_operation = :operation_id
            "#,
        )?
        .bind("operation_id", operation_id.to_string())
        .execute(&*conn)
        .await?;

        Ok(())
    }

    #[instrument(skip(self))]
    async fn reserve_mint_quote(
        &self,
        quote_id: &str,
        operation_id: &uuid::Uuid,
    ) -> Result<(), database::Error> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| Error::Database(Box::new(e)))?;

        let rows_affected = query(
            r#"
            UPDATE mint_quote
            SET used_by_operation = :operation_id
            WHERE id = :quote_id AND used_by_operation IS NULL
            "#,
        )?
        .bind("operation_id", operation_id.to_string())
        .bind("quote_id", quote_id)
        .execute(&*conn)
        .await?;

        if rows_affected == 0 {
            // Check if the quote exists
            let exists = query(
                r#"
                SELECT 1 FROM mint_quote WHERE id = :quote_id
                "#,
            )?
            .bind("quote_id", quote_id)
            .fetch_one(&*conn)
            .await?;

            if exists.is_none() {
                return Err(database::Error::UnknownQuote);
            }
            return Err(database::Error::QuoteAlreadyInUse);
        }

        Ok(())
    }

    #[instrument(skip(self))]
    async fn release_mint_quote(&self, operation_id: &uuid::Uuid) -> Result<(), database::Error> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| Error::Database(Box::new(e)))?;

        query(
            r#"
            UPDATE mint_quote
            SET used_by_operation = NULL
            WHERE used_by_operation = :operation_id
            "#,
        )?
        .bind("operation_id", operation_id.to_string())
        .execute(&*conn)
        .await?;

        Ok(())
    }

    async fn kv_read(
        &self,
        primary_namespace: &str,
        secondary_namespace: &str,
        key: &str,
    ) -> Result<Option<Vec<u8>>, database::Error> {
        crate::keyvalue::kv_read(&self.pool, primary_namespace, secondary_namespace, key).await
    }

    async fn kv_list(
        &self,
        primary_namespace: &str,
        secondary_namespace: &str,
    ) -> Result<Vec<String>, database::Error> {
        crate::keyvalue::kv_list(&self.pool, primary_namespace, secondary_namespace).await
    }

    async fn kv_write(
        &self,
        primary_namespace: &str,
        secondary_namespace: &str,
        key: &str,
        value: &[u8],
    ) -> Result<(), database::Error> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| Error::Database(Box::new(e)))?;
        crate::keyvalue::kv_write_standalone(
            &*conn,
            primary_namespace,
            secondary_namespace,
            key,
            value,
        )
        .await?;
        Ok(())
    }

    async fn kv_remove(
        &self,
        primary_namespace: &str,
        secondary_namespace: &str,
        key: &str,
    ) -> Result<(), database::Error> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| Error::Database(Box::new(e)))?;
        crate::keyvalue::kv_remove_standalone(&*conn, primary_namespace, secondary_namespace, key)
            .await?;
        Ok(())
    }

    // P2PK methods

    #[instrument(skip(self))]
    async fn add_p2pk_key(
        &self,
        pubkey: &PublicKey,
        derivation_path: DerivationPath,
        derivation_index: u32,
    ) -> Result<(), Error> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| Error::Database(Box::new(e)))?;
        let query_str = r#"
        INSERT INTO p2pk_signing_key (pubkey, derivation_index, derivation_path, created_time)
        VALUES (:pubkey, :derivation_index, :derivation_path, :created_time)
        "#
        .to_string();

        query(&query_str)?
            .bind("pubkey", pubkey.to_bytes().to_vec())
            .bind("derivation_index", derivation_index)
            .bind("derivation_path", derivation_path.to_string())
            .bind("created_time", unix_time() as i64)
            .execute(&*conn)
            .await?;

        Ok(())
    }

    #[instrument(skip(self))]
    async fn get_p2pk_key(
        &self,
        pubkey: &PublicKey,
    ) -> Result<Option<wallet::P2PKSigningKey>, Error> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| Error::Database(Box::new(e)))?;
        let query_str = r#"SELECT pubkey, derivation_index, derivation_path, created_time FROM p2pk_signing_key WHERE pubkey = :pubkey"#.to_string();

        query(&query_str)?
            .bind("pubkey", pubkey.to_bytes().to_vec())
            .fetch_one(&*conn)
            .await?
            .map(sql_row_to_p2pk_signing_key)
            .transpose()
    }

    #[instrument(skip(self))]
    async fn list_p2pk_keys(&self) -> Result<Vec<wallet::P2PKSigningKey>, Error> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| Error::Database(Box::new(e)))?;
        let query_str = r#"
        SELECT pubkey, derivation_index, derivation_path, created_time FROM p2pk_signing_key ORDER BY derivation_index DESC
        "#.to_string();

        Ok(query(&query_str)?
            .fetch_all(&*conn)
            .await?
            .into_iter()
            .filter_map(|row| {
                let row = sql_row_to_p2pk_signing_key(row).ok()?;

                Some(row)
            })
            .collect::<Vec<wallet::P2PKSigningKey>>())
    }

    #[instrument(skip(self))]
    #[instrument(skip(self))]
    async fn list_mint_identities(&self) -> Result<Vec<MintIdentity>, database::Error> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| Error::Database(Box::new(e)))?;

        let mut identities = Vec::new();

        for mut row in query(
            r#"
            SELECT
                name, version, description, description_long, contact, nuts,
                icon_url, motd, urls, mint_time, tos_url,
                pubkey, first_seen, last_seen
            FROM mint_identity
            "#,
        )?
        .fetch_all(&*conn)
        .await?
        {
            let last_seen =
                column_as_number!(row.pop().ok_or(ConversionError::MissingColumn(0, 1))?);
            let first_seen =
                column_as_number!(row.pop().ok_or(ConversionError::MissingColumn(0, 1))?);
            let pubkey = column_as_string!(
                row.pop().ok_or(ConversionError::MissingColumn(0, 1))?,
                PublicKey::from_hex,
                PublicKey::from_slice
            );

            // `sql_row_to_mint_info` expects a `pubkey` column between `name`
            // and `version`; the identity table keys on it instead.
            row.insert(1, Column::Text(pubkey.to_hex()));

            identities.push(MintIdentity {
                pubkey,
                urls: mint_urls_for(&*conn, &MintId::Pubkey(pubkey))
                    .await?
                    .into_iter()
                    .filter_map(|url| MintUrl::from_str(&url).ok())
                    .collect(),
                info: sql_row_to_mint_info(row)?,
                first_seen,
                last_seen,
            });
        }

        Ok(identities)
    }

    #[instrument(skip(self))]
    async fn list_mint_identity_claims(&self) -> Result<Vec<MintIdentityClaim>, database::Error> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| Error::Database(Box::new(e)))?;

        Ok(query(
            r#"
            SELECT mint_url, pubkey, kind, first_seen, last_seen
            FROM mint_identity_claim
            WHERE status = 'pending'
            "#,
        )?
        .fetch_all(&*conn)
        .await?
        .into_iter()
        .map(|row| {
            unpack_into!(let (mint_url, pubkey, kind, first_seen, last_seen) = row);

            Ok(MintIdentityClaim {
                mint_url: column_as_string!(mint_url, MintUrl::from_str),
                pubkey: column_as_string!(pubkey, PublicKey::from_hex, PublicKey::from_slice),
                kind: match column_as_string!(kind).as_str() {
                    "rotation" => MintIdentityClaimKind::Rotation,
                    _ => MintIdentityClaimKind::Merge,
                },
                first_seen: column_as_number!(first_seen),
                last_seen: column_as_number!(last_seen),
            })
        })
        .collect::<Result<Vec<_>, Error>>()?)
    }

    #[instrument(skip(self))]
    async fn resolve_mint_identity_claim(
        &self,
        mint_url: MintUrl,
        pubkey: PublicKey,
        accept: bool,
    ) -> Result<(), database::Error> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| Error::Database(Box::new(e)))?;
        let tx = ConnectionWithTransaction::new(conn).await?;
        let hex = hex_pubkey(&pubkey);

        if accept {
            // The user has decided these are the same mint, which is the only
            // thing that can make it so: nothing signs the pubkey.
            link_locator(&tx, &mint_url, &hex, "accepted", unix_time()).await?;
            query(
                r#"DELETE FROM mint_identity_claim
                   WHERE mint_url = :mint_url AND pubkey = :pubkey"#,
            )?
            .bind("mint_url", mint_url.to_string())
            .bind("pubkey", hex)
            .execute(&tx)
            .await?;
        } else {
            // Kept as rejected rather than deleted, so the same claim is not
            // raised again every time the mint is contacted.
            query(
                r#"UPDATE mint_identity_claim SET status = 'rejected'
                   WHERE mint_url = :mint_url AND pubkey = :pubkey"#,
            )?
            .bind("mint_url", mint_url.to_string())
            .bind("pubkey", hex)
            .execute(&tx)
            .await?;
        }

        tx.commit().await?;

        Ok(())
    }

    async fn latest_p2pk(&self) -> Result<Option<wallet::P2PKSigningKey>, Error> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| Error::Database(Box::new(e)))?;
        let query_str = r#"
        SELECT pubkey, derivation_index, derivation_path, created_time FROM p2pk_signing_key ORDER BY derivation_index DESC LIMIT 1
        "#.to_string();

        query(&query_str)?
            .fetch_one(&*conn)
            .await?
            .map(sql_row_to_p2pk_signing_key)
            .transpose()
    }
}

fn sql_row_to_mint_info(row: Vec<Column>) -> Result<MintInfo, Error> {
    unpack_into!(
        let (
            name,
            pubkey,
            version,
            description,
            description_long,
            contact,
            nuts,
            icon_url,
            motd,
            urls,
            mint_time,
            tos_url
        ) = row
    );

    Ok(MintInfo {
        name: column_as_nullable_string!(&name),
        // A pubkey is stored bare, not as JSON, so serde is the wrong reader for
        // either representation: `PublicKey` deserializes from a *quoted* JSON
        // string, which neither raw bytes nor bare hex are.
        pubkey: column_as_nullable_string!(
            &pubkey,
            |v| PublicKey::from_hex(v).ok(),
            // `add_mint` has written 33 raw bytes since the first migration; the
            // utf8 fallback covers rows carried over from a hex-encoding backend.
            |v| PublicKey::from_slice(v).ok().or_else(|| str::from_utf8(v)
                .ok()
                .and_then(|s| PublicKey::from_hex(s).ok()))
        ),
        version: column_as_nullable_string!(&version).and_then(|v| serde_json::from_str(&v).ok()),
        description: column_as_nullable_string!(description),
        description_long: column_as_nullable_string!(description_long),
        contact: column_as_nullable_string!(contact, |v| serde_json::from_str(&v).ok()),
        nuts: column_as_nullable_string!(nuts, |v| serde_json::from_str(&v).ok())
            .unwrap_or_default(),
        urls: column_as_nullable_string!(urls, |v| serde_json::from_str(&v).ok()),
        icon_url: column_as_nullable_string!(icon_url),
        motd: column_as_nullable_string!(motd),
        time: column_as_nullable_number!(mint_time).map(|t| t),
        tos_url: column_as_nullable_string!(tos_url),
    })
}

#[instrument(skip_all)]
fn sql_row_to_keyset(row: Vec<Column>) -> Result<KeySetInfo, Error> {
    unpack_into!(
        let (
            id,
            unit,
            active,
            input_fee_ppk,
            final_expiry
        ) = row
    );

    Ok(KeySetInfo {
        id: column_as_string!(id, Id::from_str, Id::from_bytes),
        unit: column_as_string!(unit, CurrencyUnit::from_str),
        active: matches!(active, Column::Integer(1)),
        input_fee_ppk: column_as_nullable_number!(input_fee_ppk).unwrap_or(0),
        final_expiry: column_as_nullable_number!(final_expiry),
    })
}

fn sql_row_to_mint_quote(row: Vec<Column>) -> Result<MintQuote, Error> {
    unpack_into!(
        let (
            id,
            mint_url,
            amount,
            unit,
            request,
            state,
            expiry,
            secret_key,
            row_method,
            row_amount_minted,
            row_amount_paid,
            updated_at,
            estimated_blocks,
            used_by_operation,
            version
        ) = row
    );

    let amount: Option<i64> = column_as_nullable_number!(amount);

    let amount_paid: u64 = column_as_number!(row_amount_paid);
    let amount_minted: u64 = column_as_number!(row_amount_minted);
    let expiry_val: u64 = column_as_number!(expiry);
    let updated_at: u64 = column_as_number!(updated_at);
    let version_val: u32 = column_as_number!(version);
    let payment_method =
        PaymentMethod::from_str(&column_as_string!(row_method)).map_err(Error::from)?;

    Ok(MintQuote {
        id: column_as_string!(id),
        mint_url: column_as_string!(mint_url, MintUrl::from_str),
        amount: amount.and_then(Amount::from_i64),
        unit: column_as_string!(unit, CurrencyUnit::from_str),
        request: column_as_string!(request),
        state: column_as_string!(state, MintQuoteState::from_str),
        expiry: expiry_val,
        secret_key: column_as_nullable_string!(secret_key, |s| SecretKey::from_str(&s).ok()),
        payment_method,
        amount_issued: Amount::from(amount_minted),
        amount_paid: Amount::from(amount_paid),
        updated_at,
        estimated_blocks: column_as_nullable_number!(estimated_blocks),
        used_by_operation: column_as_nullable_string!(used_by_operation),
        version: version_val,
    })
}

fn sql_row_to_melt_quote(row: Vec<Column>) -> Result<wallet::MeltQuote, Error> {
    unpack_into!(
        let (
            id,
            unit,
            amount,
            request,
            fee_reserve,
            state,
            expiry,
            payment_proof,
            row_method,
            estimated_blocks,
            fee_index,
            used_by_operation,
            version,
            mint_url
        ) = row
    );

    let payment_method =
        PaymentMethod::from_str(&column_as_string!(row_method)).map_err(Error::from)?;

    let amount_val: u64 = column_as_number!(amount);
    let fee_reserve_val: u64 = column_as_number!(fee_reserve);
    let expiry_val: u64 = column_as_number!(expiry);
    let version_val: u32 = column_as_number!(version);

    Ok(wallet::MeltQuote {
        id: column_as_string!(id),
        mint_url: column_as_nullable_string!(mint_url, |s| MintUrl::from_str(&s).ok()),
        unit: column_as_string!(unit, CurrencyUnit::from_str),
        amount: Amount::from(amount_val),
        request: column_as_string!(request),
        fee_reserve: Amount::from(fee_reserve_val),
        state: column_as_string!(state, MeltQuoteState::from_str),
        expiry: expiry_val,
        payment_proof: column_as_nullable_string!(payment_proof),
        estimated_blocks: column_as_nullable_number!(estimated_blocks),
        fee_index: column_as_nullable_number!(fee_index),
        payment_method,
        used_by_operation: column_as_nullable_string!(used_by_operation),
        version: version_val,
    })
}

fn sql_row_to_proof_info(row: Vec<Column>) -> Result<ProofInfo, Error> {
    unpack_into!(
        let (
            amount,
            unit,
            keyset_id,
            secret,
            c,
            witness,
            dleq_e,
            dleq_s,
            dleq_r,
            y,
            mint_url,
            state,
            spending_condition,
            used_by_operation,
            created_by_operation,
            p2pk_e
        ) = row
    );

    let dleq = match (
        column_as_nullable_binary!(dleq_e),
        column_as_nullable_binary!(dleq_s),
        column_as_nullable_binary!(dleq_r),
    ) {
        (Some(e), Some(s), Some(r)) => {
            let e_key = SecretKey::from_slice(&e)?;
            let s_key = SecretKey::from_slice(&s)?;
            let r_key = SecretKey::from_slice(&r)?;

            Some(ProofDleq::new(e_key, s_key, r_key))
        }
        _ => None,
    };

    let amount: u64 = column_as_number!(amount);
    let proof = Proof {
        amount: Amount::from(amount),
        keyset_id: column_as_string!(keyset_id, Id::from_str),
        secret: column_as_string!(secret, Secret::from_str),
        witness: column_as_nullable_string!(witness, |v| { serde_json::from_str(&v).ok() }, |v| {
            serde_json::from_slice(&v).ok()
        }),
        c: column_as_string!(c, PublicKey::from_str, PublicKey::from_slice),
        dleq,
        p2pk_e: column_as_nullable_binary!(p2pk_e)
            .map(|bytes| PublicKey::from_slice(&bytes))
            .transpose()?,
    };

    let used_by_operation =
        column_as_nullable_string!(used_by_operation).and_then(|id| Uuid::from_str(&id).ok());
    let created_by_operation =
        column_as_nullable_string!(created_by_operation).and_then(|id| Uuid::from_str(&id).ok());

    Ok(ProofInfo {
        proof,
        y: column_as_string!(y, PublicKey::from_str, PublicKey::from_slice),
        mint_url: column_as_string!(mint_url, MintUrl::from_str),
        state: column_as_string!(state, State::from_str),
        spending_condition: column_as_nullable_string!(
            spending_condition,
            |r| { serde_json::from_str(&r).ok() },
            |r| { serde_json::from_slice(&r).ok() }
        ),
        unit: column_as_string!(unit, CurrencyUnit::from_str),
        used_by_operation,
        created_by_operation,
    })
}

fn sql_row_to_wallet_saga(row: Vec<Column>) -> Result<wallet::WalletSaga, Error> {
    unpack_into!(
        let (
            id,
            kind,
            state,
            amount,
            mint_url,
            unit,
            quote_id,
            created_at,
            updated_at,
            data,
            version
        ) = row
    );

    let id_str: String = column_as_string!(id);
    let id = uuid::Uuid::parse_str(&id_str).map_err(|e| {
        Error::Database(Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Invalid UUID: {}", e),
        )))
    })?;
    let kind_str: String = column_as_string!(kind);
    let state_json: String = column_as_string!(state);
    let amount: u64 = column_as_number!(amount);
    let mint_url: MintUrl = column_as_string!(mint_url, MintUrl::from_str);
    let unit: CurrencyUnit = column_as_string!(unit, CurrencyUnit::from_str);
    let quote_id: Option<String> = column_as_nullable_string!(quote_id);
    let created_at: u64 = column_as_number!(created_at);
    let updated_at: u64 = column_as_number!(updated_at);
    let data_json: String = column_as_string!(data);
    let version: u32 = column_as_number!(version);

    let kind = wallet::OperationKind::from_str(&kind_str).map_err(|_| {
        Error::Database(Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Invalid operation kind: {}", kind_str),
        )))
    })?;
    let state: wallet::WalletSagaState = serde_json::from_str(&state_json).map_err(|e| {
        Error::Database(Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Failed to deserialize saga state: {}", e),
        )))
    })?;
    let data: wallet::OperationData = serde_json::from_str(&data_json).map_err(|e| {
        Error::Database(Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Failed to deserialize saga data: {}", e),
        )))
    })?;

    Ok(wallet::WalletSaga {
        id,
        kind,
        state,
        amount: Amount::from(amount),
        mint_url,
        unit,
        quote_id,
        created_at,
        updated_at,
        data,
        version,
    })
}

fn sql_row_to_transaction(row: Vec<Column>) -> Result<Transaction, Error> {
    unpack_into!(
        let (
            mint_url,
            direction,
            unit,
            amount,
            fee,
            ys,
            timestamp,
            memo,
            metadata,
            quote_id,
            payment_request,
            payment_proof,
            payment_method,
            saga_id
        ) = row
    );

    let amount: u64 = column_as_number!(amount);
    let fee: u64 = column_as_number!(fee);

    let saga_id: Option<Uuid> = column_as_nullable_string!(saga_id)
        .map(|id| Uuid::from_str(&id).ok())
        .flatten();

    Ok(Transaction {
        mint_url: column_as_string!(mint_url, MintUrl::from_str),
        direction: column_as_string!(direction, TransactionDirection::from_str),
        unit: column_as_string!(unit, CurrencyUnit::from_str),
        amount: Amount::from(amount),
        fee: Amount::from(fee),
        ys: column_as_binary!(ys)
            .chunks(33)
            .map(PublicKey::from_slice)
            .collect::<Result<Vec<_>, _>>()?,
        timestamp: column_as_number!(timestamp),
        memo: column_as_nullable_string!(memo),
        metadata: column_as_nullable_string!(metadata, |v| serde_json::from_str(&v).ok(), |v| {
            serde_json::from_slice(&v).ok()
        })
        .unwrap_or_default(),
        quote_id: column_as_nullable_string!(quote_id),
        payment_request: column_as_nullable_string!(payment_request),
        payment_proof: column_as_nullable_string!(payment_proof),
        payment_method: column_as_nullable_string!(payment_method)
            .map(|v| PaymentMethod::from_str(&v))
            .transpose()
            .map_err(Error::from)?,
        saga_id,
    })
}

fn sql_row_to_p2pk_signing_key(row: Vec<Column>) -> Result<wallet::P2PKSigningKey, Error> {
    unpack_into!(
        let (
            pubkey,
            derivation_index,
            derivation_path,
            created_time
        ) = row
    );

    Ok(wallet::P2PKSigningKey {
        pubkey: column_as_string!(pubkey, PublicKey::from_str, PublicKey::from_slice),
        derivation_index: column_as_number!(derivation_index),
        derivation_path: column_as_string!(derivation_path, DerivationPath::from_str),
        created_time: column_as_number!(created_time),
    })
}
