//! gRPC round trip for the identity signing key.
//!
//! Exercises both sides of the `Sign` RPC, which also covers the schema-version
//! handshake between client and server.
#![cfg(all(feature = "grpc", feature = "sqlite", not(target_arch = "wasm32")))]

use std::sync::Arc;

use bitcoin::secp256k1::schnorr::Signature;
use cdk_common::{BlindSignature, BlindedMessage, Error, Proof};
use cdk_signatory::db_signatory::DbSignatory;
use cdk_signatory::signatory::{RotateKeyArguments, Signatory, SignatoryKeySet, SignatoryKeysets};
use cdk_signatory::{start_grpc_server_with_incoming, SignatoryRpcClient};
use tokio::net::TcpListener;
use tokio_stream::wrappers::TcpListenerStream;

async fn db_signatory(seed: &[u8]) -> DbSignatory {
    let store = Arc::new(
        cdk_sqlite::mint::memory::empty()
            .await
            .expect("in-memory db"),
    );
    DbSignatory::new(store, seed, Default::default(), Default::default())
        .await
        .expect("DbSignatory::new")
}

async fn serve<S>(signatory: Arc<S>) -> SignatoryRpcClient
where
    S: Signatory + Send + Sync + 'static,
{
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("local addr");
    tokio::spawn(async move {
        if let Err(err) =
            start_grpc_server_with_incoming(signatory, TcpListenerStream::new(listener)).await
        {
            tracing::error!("signatory server stopped: {err}");
        }
    });

    SignatoryRpcClient::new("127.0.0.1", addr.port(), None)
        .await
        .expect("connect to signatory")
}

#[tokio::test]
async fn sign_round_trips_over_grpc() {
    let signatory = Arc::new(db_signatory(b"test-seed-for-grpc-signing").await);
    let expected = signatory.keysets().await.expect("keysets");
    let client = serve(signatory).await;

    let payload = b"an arbitrary stream of bytes".to_vec();
    let signature = client.sign(payload.clone()).await.expect("sign over grpc");

    let keysets = client.keysets().await.expect("keysets over grpc");
    assert_eq!(
        keysets.pubkey, expected.pubkey,
        "the identity pubkey must survive the proto round trip"
    );

    keysets
        .pubkey
        .verify(&payload, &signature)
        .expect("signature from the remote signatory must verify");
    assert!(
        keysets.pubkey.verify(b"tampered", &signature).is_err(),
        "a tampered payload must not verify"
    );
}

#[tokio::test]
async fn empty_payload_signs_and_verifies() {
    let signatory = Arc::new(db_signatory(b"test-seed-for-grpc-empty-payload").await);
    let client = serve(signatory).await;

    let signature = client.sign(Vec::new()).await.expect("sign empty payload");

    client
        .keysets()
        .await
        .expect("keysets over grpc")
        .pubkey
        .verify(b"", &signature)
        .expect("a signature over the empty payload must verify");
}

/// Delegates everything to a real signatory except `sign`, which fails, so the
/// `SignResponse.error` branch of the proto round trip is exercised.
struct FailingSigner {
    inner: DbSignatory,
}

#[async_trait::async_trait]
impl Signatory for FailingSigner {
    fn name(&self) -> String {
        self.inner.name()
    }

    async fn blind_sign(
        &self,
        blinded_messages: Vec<BlindedMessage>,
    ) -> Result<Vec<BlindSignature>, Error> {
        self.inner.blind_sign(blinded_messages).await
    }

    async fn verify_proofs(&self, proofs: Vec<Proof>) -> Result<(), Error> {
        self.inner.verify_proofs(proofs).await
    }

    async fn sign(&self, _payload: Vec<u8>) -> Result<Signature, Error> {
        Err(Error::Custom("identity key unavailable".to_string()))
    }

    async fn keysets(&self) -> Result<SignatoryKeysets, Error> {
        self.inner.keysets().await
    }

    async fn subscribe_keysets(
        &self,
    ) -> Result<tokio::sync::watch::Receiver<SignatoryKeysets>, Error> {
        self.inner.subscribe_keysets().await
    }

    async fn rotate_keyset(&self, args: RotateKeyArguments) -> Result<SignatoryKeySet, Error> {
        self.inner.rotate_keyset(args).await
    }
}

#[tokio::test]
async fn sign_error_crosses_the_proto_boundary() {
    let inner = db_signatory(b"test-seed-for-grpc-sign-error").await;
    let client = serve(Arc::new(FailingSigner { inner })).await;

    let err = client
        .sign(b"an arbitrary stream of bytes".to_vec())
        .await
        .expect_err("the signatory error must reach the client");

    assert!(
        err.to_string().contains("identity key unavailable"),
        "the error must survive the round trip, got: {err}"
    );
}
