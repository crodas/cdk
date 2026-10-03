//! State filter database tests

use cashu::nuts::{FilterElement, State};
use cashu::SecretKey;

use crate::database::mint::{Database, Error, StateFilterConfig};

fn test_config() -> StateFilterConfig {
    StateFilterConfig {
        genesis: 1_700_000_000,
        epoch_seconds: 60,
        p: 20,
        page_size: 8,
    }
}

/// Test that the filter config can only be written once; a second call with
/// different values must be silently ignored and the original values kept.
pub async fn set_state_filter_config_is_write_once<DB>(db: DB)
where
    DB: Database<Error>,
{
    let mut tx = Database::begin_transaction(&db).await.unwrap();
    tx.set_state_filter_config(&test_config()).await.unwrap();
    tx.commit().await.unwrap();

    let conflicting = StateFilterConfig {
        genesis: 0,
        epoch_seconds: 999,
        p: 1,
        page_size: 1,
    };
    let mut tx = Database::begin_transaction(&db).await.unwrap();
    tx.set_state_filter_config(&conflicting).await.unwrap();
    tx.commit().await.unwrap();

    let stored = db.get_state_filter_config().await.unwrap().unwrap();
    assert_eq!(stored, test_config());
}

/// Test that reading the config before it has ever been set returns `None`.
pub async fn get_state_filter_config_when_unset<DB>(db: DB)
where
    DB: Database<Error>,
{
    let stored = db.get_state_filter_config().await.unwrap();
    assert!(stored.is_none());
}

/// Test that elements recorded for an epoch round-trip through
/// `add_filter_elements`/`take_filter_elements`, and that taking is
/// destructive: a second take on the same epoch returns nothing.
pub async fn take_filter_elements_is_destructive<DB>(db: DB)
where
    DB: Database<Error>,
{
    let y_a = SecretKey::generate().public_key();
    let y_b = SecretKey::generate().public_key();
    let element_a = FilterElement::proof_state(&y_a, State::Spent).unwrap();
    let element_b = FilterElement::proof_state(&y_b, State::Spent).unwrap();

    let mut tx = Database::begin_transaction(&db).await.unwrap();
    tx.add_filter_elements(0, &[element_a, element_b])
        .await
        .unwrap();
    tx.commit().await.unwrap();

    let mut tx = Database::begin_transaction(&db).await.unwrap();
    let mut taken = tx.take_filter_elements(0).await.unwrap();
    tx.commit().await.unwrap();
    taken.sort();
    let mut expected = vec![element_a, element_b];
    expected.sort();
    assert_eq!(taken, expected);

    let mut tx = Database::begin_transaction(&db).await.unwrap();
    let taken_again = tx.take_filter_elements(0).await.unwrap();
    tx.commit().await.unwrap();
    assert!(taken_again.is_empty());
}

/// Test that elements added for one epoch never leak into a take on a
/// different epoch.
pub async fn take_filter_elements_is_scoped_to_its_epoch<DB>(db: DB)
where
    DB: Database<Error>,
{
    let y = SecretKey::generate().public_key();
    let element = FilterElement::proof_state(&y, State::Spent).unwrap();

    let mut tx = Database::begin_transaction(&db).await.unwrap();
    tx.add_filter_elements(0, &[element]).await.unwrap();
    tx.commit().await.unwrap();

    let mut tx = Database::begin_transaction(&db).await.unwrap();
    let other_epoch = tx.take_filter_elements(1).await.unwrap();
    tx.commit().await.unwrap();
    assert!(other_epoch.is_empty());

    let mut tx = Database::begin_transaction(&db).await.unwrap();
    let same_epoch = tx.take_filter_elements(0).await.unwrap();
    tx.commit().await.unwrap();
    assert_eq!(same_epoch, vec![element]);
}

/// Test that a built filter is immutable: once stored for an epoch, a second
/// `add_filter` call for that same epoch with different data must be
/// silently ignored rather than overwriting it.
pub async fn add_filter_is_immutable_once_built<DB>(db: DB)
where
    DB: Database<Error>,
{
    let first_data = vec![1u8, 2, 3];
    let second_data = vec![9u8, 9, 9];

    let mut tx = Database::begin_transaction(&db).await.unwrap();
    tx.add_filter(0, 100, 160, &first_data).await.unwrap();
    tx.commit().await.unwrap();

    let mut tx = Database::begin_transaction(&db).await.unwrap();
    tx.add_filter(0, 200, 260, &second_data).await.unwrap();
    tx.commit().await.unwrap();

    let filters = db.get_filters(0, 10).await.unwrap();
    assert_eq!(filters.len(), 1);
    assert_eq!(filters[0].start, 100);
    assert_eq!(filters[0].end, 160);
    assert_eq!(filters[0].data, cashu::util::hex::encode(&first_data));
}

/// Test the epoch boundary on `get_filters`: `first_epoch` is inclusive, and
/// epochs below it are excluded even when they exist.
pub async fn get_filters_first_epoch_is_inclusive<DB>(db: DB)
where
    DB: Database<Error>,
{
    let mut tx = Database::begin_transaction(&db).await.unwrap();
    tx.add_filter(0, 0, 60, &[0u8]).await.unwrap();
    tx.add_filter(1, 60, 120, &[1u8]).await.unwrap();
    tx.add_filter(2, 120, 180, &[2u8]).await.unwrap();
    tx.commit().await.unwrap();

    let filters = db.get_filters(1, 10).await.unwrap();
    assert_eq!(filters.len(), 2);
    assert_eq!(filters[0].start, 60);
    assert_eq!(filters[1].start, 120);
}

/// Test that `get_filters` honors `limit` and returns results in ascending
/// epoch order.
pub async fn get_filters_respects_limit_and_order<DB>(db: DB)
where
    DB: Database<Error>,
{
    let mut tx = Database::begin_transaction(&db).await.unwrap();
    tx.add_filter(2, 120, 180, &[2u8]).await.unwrap();
    tx.add_filter(0, 0, 60, &[0u8]).await.unwrap();
    tx.add_filter(1, 60, 120, &[1u8]).await.unwrap();
    tx.commit().await.unwrap();

    let filters = db.get_filters(0, 2).await.unwrap();
    assert_eq!(filters.len(), 2);
    assert_eq!(filters[0].start, 0);
    assert_eq!(filters[1].start, 60);
}

/// Test that `latest_built_epoch` returns `None` before any filter exists,
/// and the maximum epoch even when filters were inserted out of order.
pub async fn latest_built_epoch_tracks_the_maximum<DB>(db: DB)
where
    DB: Database<Error>,
{
    assert!(db.latest_built_epoch().await.unwrap().is_none());

    let mut tx = Database::begin_transaction(&db).await.unwrap();
    tx.add_filter(0, 0, 60, &[0u8]).await.unwrap();
    tx.add_filter(2, 120, 180, &[2u8]).await.unwrap();
    tx.add_filter(1, 60, 120, &[1u8]).await.unwrap();
    tx.commit().await.unwrap();

    assert_eq!(db.latest_built_epoch().await.unwrap(), Some(2));
}

/// Test that `get_filter_elements` is non-destructive: reading the pending
/// elements for an epoch does not remove them, unlike `take_filter_elements`.
pub async fn get_filter_elements_does_not_remove_them<DB>(db: DB)
where
    DB: Database<Error>,
{
    let y = SecretKey::generate().public_key();
    let element = FilterElement::proof_state(&y, State::Spent).unwrap();

    let mut tx = Database::begin_transaction(&db).await.unwrap();
    tx.add_filter_elements(0, &[element]).await.unwrap();
    tx.commit().await.unwrap();

    let first_read = db.get_filter_elements(0).await.unwrap();
    let second_read = db.get_filter_elements(0).await.unwrap();
    assert_eq!(first_read, vec![element]);
    assert_eq!(second_read, first_read);
}
