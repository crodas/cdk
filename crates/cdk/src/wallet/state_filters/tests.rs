use std::sync::Arc;

use cdk_common::nuts::state_filters::{
    encode, Filter, FilterElement, FilterKind, GetFiltersInfoResponse, GetFiltersResponse, Settings,
};
use cdk_common::util::hex;
use cdk_common::State;

use crate::wallet::test_utils::{
    create_test_db, create_test_wallet_with_mock, test_keyset_id, test_mint_info, test_mint_url,
    test_proof_info, MockMintConnector,
};

const P: u8 = 10;

fn filter(start: u64, end: u64, elements: &[FilterElement]) -> Filter {
    let data = encode(elements, P).expect("encode");
    Filter {
        start,
        end,
        data: hex::encode(data),
    }
}

fn info(current_page: u64, current_page_count: u64, latest_end: u64) -> GetFiltersInfoResponse {
    GetFiltersInfoResponse {
        p: P,
        epoch: 100,
        kinds: vec![FilterKind::ProofState],
        page_size: 50,
        first_page: 0,
        current_page,
        current_page_count,
        earliest_start: 0,
        latest_end,
        pending: false,
    }
}

/// A sweep re-fetches the still-open page on every call, since the mint keeps
/// filling it. An epoch already tested on a prior sweep must not be reported
/// again just because the page containing it was fetched again, but a new
/// epoch added to that same page since the last sweep must still be caught.
#[tokio::test]
async fn a_second_sweep_skips_already_tested_epochs_on_the_re_fetched_page() {
    let db = create_test_db().await;
    let mint_url = test_mint_url();
    let keyset_id = test_keyset_id();

    let proof_a = test_proof_info(keyset_id, 4, mint_url.clone());
    let proof_b = test_proof_info(keyset_id, 8, mint_url.clone());
    db.update_proofs(vec![proof_a.clone(), proof_b.clone()], vec![])
        .await
        .expect("seed proofs");

    let element_a = FilterElement::proof_state(&proof_a.y, State::Spent).expect("supported");
    let element_b = FilterElement::proof_state(&proof_b.y, State::Spent).expect("supported");

    let mock = Arc::new(MockMintConnector::new());
    // The shared fixture's mint time is stale by now; the wallet refuses
    // to talk to a mint whose clock looks wrong, so clear it for this test.
    let mut mint_info = test_mint_info();
    mint_info.time = None;
    mint_info.nuts.state_filters = Settings::new(vec![FilterKind::ProofState]);
    mock.set_mint_info_response(Ok(mint_info));

    // First sweep: page 0 holds an empty epoch and the one that caught proof_a.
    mock.set_filters_info_response(Ok(info(0, 2, 200)));
    mock.set_filters_response(
        0,
        Ok(GetFiltersResponse {
            page: 0,
            filters: vec![filter(0, 100, &[]), filter(100, 200, &[element_a])],
        }),
    );

    let wallet = create_test_wallet_with_mock(db.clone(), mock.clone()).await;
    let first = wallet.sweep_state_filters().await.expect("first sweep");
    assert_eq!(first.proofs, vec![proof_a.y]);

    // Second sweep: the mint has not rolled to a new page yet, so the whole
    // page is re-fetched, now carrying a third epoch that caught proof_b.
    mock.set_filters_info_response(Ok(info(0, 3, 300)));
    mock.set_filters_response(
        0,
        Ok(GetFiltersResponse {
            page: 0,
            filters: vec![
                filter(0, 100, &[]),
                filter(100, 200, &[element_a]),
                filter(200, 300, &[element_b]),
            ],
        }),
    );

    let second = wallet.sweep_state_filters().await.expect("second sweep");
    assert_eq!(
        second.proofs,
        vec![proof_b.y],
        "a re-fetched page must not repeat an already tested epoch, and must still catch a new one sharing it"
    );
}
