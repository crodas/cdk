# Signatory keyset subscription and push injection

* Status: accepted
* Authors: Cesar Rodas
* Date: 2026-07-14
* Targeted modules: cdk-signatory, cdk (mint)
* Associated tickets/PRs: n/a

## Context and Problem Statement

ADR 0001 leaves the mint pulling keysets: it loads them once at construction
and re-reads them only when it is the one that called `rotate_keyset`. A
keyset rotated on the signatory side, out of band from a given mint instance,
is never seen, and `SignatoryRpcClient` has no reconnect that would re-sync
after a dropped connection. How does the signatory push keyset changes to the
mint, re-inject the current set after a gRPC reconnect, and let the mint notify
wallets, without breaking the key-segregation boundary from ADR 0001?

## Decision Drivers

* The `Signatory` trait is the only boundary the mint may use to reach key
  material; any push path must go through it, not around it.
* The mint already stores keysets in `Arc<ArcSwap<Vec<SignatoryKeySet>>>` and
  reads them lock-free; the push path should feed that store, not replace it.
* Keysets are a current-set, not an event log. The consumer always wants the
  latest full snapshot; a backlog of stale snapshots is wrong.
* A dropped gRPC connection must re-sync keysets on reconnect.
* Reuse existing patterns: the `PubSubManager`
  (`crates/cdk/src/mint/subscription.rs`) and the reconnecting subscription in
  `crates/cdk-common/src/pub_sub/`.

## Considered Options

#### Shared `Arc<ArcSwap>` between client and mint

The signatory client and the mint share one `ArcSwap`; the client's reconnect
loop writes, the mint reads.

**Pros:**

* Good, because it is the least code: no drain task on the mint side.

**Cons:**

* Bad, because it swaps keysets silently, so the mint cannot emit a wallet
  event or refresh derived state.
* Bad, because it leaks the mint's storage type across the trait boundary.

#### Raw `mpsc` of keyset updates

The trait exposes an `mpsc::Receiver<SignatoryKeysets>`; the mint drains it.

**Pros:**

* Good, because the mint is woken on change and can react.

**Cons:**

* Bad, because an `mpsc` can back up with stale full snapshots under a slow
  consumer, which contradicts the current-set semantics.

#### `watch` receiver drained into the mint's `ArcSwap`

The trait exposes a `watch::Receiver<SignatoryKeysets>`; the mint drains it
into its existing `ArcSwap` and notifies wallets on each change.

**Pros:**

* Good, because `watch` retains only the latest value (like `ArcSwap`) so a
  slow consumer never accumulates stale versions.
* Good, because it wakes the consumer on change (like `mpsc`) so the mint can
  react: notify wallets, log, refresh derived state.
* Good, because the mint keeps its storage type private to itself.

**Cons:**

* Bad, because it costs one small drain task on the mint side.

## Decision Outcome

Chosen option: "`watch` receiver drained into the mint's `ArcSwap`", because it
is the only option that keeps the latest-snapshot semantics, lets the mint
react to changes, and preserves the ADR 0001 boundary.

The trait gains one method:

```rust
/// Latest-snapshot stream of the signatory's keysets. Yields the full
/// current set on subscribe and again on every rotation.
async fn subscribe_keysets(&self)
    -> Result<tokio::sync::watch::Receiver<SignatoryKeysets>, Error>;
```

The existing `keysets()` method stays; the mint uses it for the synchronous
bootstrap at construction, and `subscribe_keysets()` handles every update
after that.

**gRPC contract.** One server-streaming RPC, reusing existing messages:

```proto
// Sends the current KeysResponse immediately, then again on every rotation.
rpc SubscribeKeysets(EmptyRequest) returns (stream KeysResponse);
```

The protocol rule that makes reconnect correct: on every connect the server
sends the current full snapshot first, then one snapshot per rotation.

**Signatory-side push.** `DbSignatory` holds a `watch::Sender` and publishes a
fresh snapshot from `reload_keys_from_db`, the single place its in-memory
keysets change (initial load and after every rotation). Its `subscribe_keysets`
hands out `watch::Sender::subscribe()`. The streaming server handler wraps that
receiver in a `WatchStream`, which yields the current value on connect and each
replacement after, giving "snapshot first, then one per rotation" for free.

**gRPC client reconnect.** `SignatoryRpcClient` owns a
`watch::Sender<SignatoryKeysets>` and spawns one background task:

```text
loop {
    match client.subscribe_keysets(Empty).await {
        Ok(stream) => for msg in stream { tx.send_replace(msg) }
        Err(_)     => {}
    }
    backoff.sleep().await   // exponential, mirrors pub_sub reconnect
}
```

Because the server sends the current snapshot first on every connect, a
dropped-then-restored stream re-injects the latest keysets automatically, with
no client-side cache or diff.

**Mint integration.** Bootstrap is unchanged (`keysets().await` at
construction). The mint then subscribes and spawns a drain task under the
existing supervisor:

```rust
let mut rx = signatory.subscribe_keysets().await?;
tokio::spawn(async move {
    while rx.changed().await.is_ok() {
        let ks = rx.borrow_and_update().clone();
        keysets.store(Arc::new(ks.keysets));   // same store() rotate uses today
        pubsub_manager.notify_keysets_changed();
    }
});
```

**Keyset-change notification.** The mint drain task fires
`PubSubManager::notify_keysets_changed()` after each swap. This is backed by an
in-process `tokio::sync::broadcast` channel on the `PubSubManager`, exposed to
the rest of CDK through `Mint::subscribe_keyset_changes()`. The payload is a
bare signal: consumers re-read the keysets rather than trust a message body.

The over-the-wire NUT-17 notification to wallets is deferred. NUT-17 today
carries only quote and proof-state kinds, and a `KeysetsChanged` kind would
have to be added to the shared `cashu` protocol crate (`NotificationPayload`,
`Kind`, and the WS layer), which is a protocol change worth proposing upstream
on its own. The in-process broadcast is the seam a future WS bridge would hang
off, so adding the wire event later does not change the mint-side plumbing.

### Positive Consequences

* A signatory-side rotation reaches the mint without a restart or a
  mint-initiated rotate.
* gRPC reconnect re-syncs keysets by construction, closing the no-reconnect
  gap from ADR 0001.
* In-process consumers are notified of a keyset change instead of polling, and
  the seam for a future wallet-facing WS notification is in place.
* The private-key boundary from ADR 0001 is preserved: only public
  `SignatoryKeysets` cross the trait, and the mint's storage type stays
  private.

### Negative Consequences

* A new streaming RPC and a schema-version bump on the proto. Older clients
  keep working through the unary `Keysets` path; the subscription is additive.
* One background task per mint (drain) and one per gRPC client (reconnect),
  plus a watch channel in `DbSignatory`.
* Wallets are not yet notified over the wire: the keyset-change notification is
  in-process only until a NUT-17 keyset kind is added to the `cashu` crate.
* Every `Signatory` implementation must implement `subscribe_keysets`,
  including the embedded wrapper.

## Links

* Refines [ADR-0001](0001-signatory-mint-key-segregation.md)
* Reuses the reconnect and latest-on-subscribe patterns in
  `crates/cdk-common/src/pub_sub/`
