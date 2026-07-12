# Automatic keyset rotation by age

* Status: proposed
* Authors: Cesar Rodas
* Date: 2026-07-11
* Targeted modules: `cdk` (`mint`), `cdk-signatory`, `cdk-mintd`
* Associated tickets/PRs: branch `feature/automatic-key-rotation`

## Context and Problem Statement

A mint keyset should not stay active indefinitely, but retiring one in CDK
requires the operator to bump the derivation path and restart. Nutshell rotates
active keysets automatically once they exceed a configured age. How should CDK
rotate an active keyset once it is older than a configured interval, generating
and activating the next keyset and deactivating the old one, without operator
intervention and without corrupting state under concurrency?

The answer is constrained by CDK's architecture. Unlike Nutshell, where the
`Ledger` owns keyset generation, CDK delegates all key material to a `Signatory`
(`crates/cdk-signatory`); the `Mint` holds only a read-only in-memory cache and
calls `signatory.rotate_keyset(...)`. Two existing properties matter:
`DbSignatory::rotate_keyset` already inserts the new keyset and flips the active
pointer in one database transaction (so there is always exactly one active
keyset per unit), and keyset IDs are deterministic from the derivation path (so
two rotations from the same active keyset produce the same ID, and
`add_keyset_info` is an upsert). These prevent Nutshell's non-atomic-rotation,
timestamp-drift, and `balance`-column bugs by construction.

## Decision Drivers

* Match Nutshell's behavior and defaults (enabled, 90-day interval, grace-period
  preservation) so the port is faithful.
* Keep a single source of truth. The signatory owns the keysets, their
  `valid_from`/`final_expiry`, and the database; the Mint's in-memory cache
  should be a derived view of that, not an independent decision point that can
  drift (including missing a peer replica's rotation).
* Be correct under concurrency (single process, and the common multi-replica
  cases) without a large cross-backend database change.
* Do not break the gRPC signatory protocol.

## Considered Options

### Concurrency

#### Option A: `active_keyset_id` guard only (no table locking)

The caller passes the keyset it intended to rotate. Inside the rotation
transaction, if that keyset is no longer active, another process/task already
rotated it, so skip and return the current active keyset. The next index is the
max `derivation_path_index` across all keysets for the unit, plus one.

**Pros:**

* Covers single-process fully (the periodic task is the only automatic caller,
  so no in-process overlap).
* Covers multi-replica simultaneous rotations via deterministic-ID + upsert
  convergence, and near-sequential ones via the skip guard.
* Mirrors Nutshell's `active_keyset_id` parameter; no new database machinery.

**Cons:**

* Does not strictly serialize two replicas that both read the active keyset in a
  tight window before either commits.

#### Option B: guard plus `SELECT ... FOR UPDATE` table locking

Add row/table locking on the keysets table inside the rotation transaction.

**Pros:**

* Fully serializes multi-replica near-sequential rotations.

**Cons:**

* Touches every SQL backend (`cdk-sql-common` for sqlite and postgres, plus
  redb) for a case the guard already handles in practice.

### Signatory scope

#### Option A: embedded/in-process only

Keep the rotation decision inside the signatory (it already holds `valid_from`
and `final_expiry` on `MintKeySetInfo`), add `RotateKeyArguments.active_keyset_id`
as a plain Rust field, and keep the gRPC protobuf schema unchanged. The remote
gRPC client uses the default trait implementations (rotation disabled, keysets
served as-is).

**Pros:**

* No `.proto` changes; the common (embedded) deployment gets rotation.
* A remote signatory still builds and serves as a refresh source.

**Cons:**

* A remote gRPC signatory does not drive age-based auto-rotation.

#### Option B: wire the fields through protobuf

Extend the `.proto`, conversions, client, and server.

**Pros:**

* Remote-signatory deployments auto-rotate too.

**Cons:**

* Larger surface for a less common deployment.

### Rotation ownership and cache synchronization

The Mint's `ArcSwap` cache must reflect rotations. Who decides, and how does the
cache stay in sync?

#### Option A: Mint decides, refreshes its cache on its own rotation

The Mint holds the interval, evaluates the age predicate against its cache
(which must then carry `valid_from`), calls `rotate_keyset`, and refreshes the
cache when it triggers a rotation.

**Pros:**

* Reuses the Mint's existing task lifecycle.

**Cons:**

* Two sources of truth: the age data is duplicated onto the cache struct, and a
  peer replica's rotation never refreshes this Mint's cache.

#### Option B: signatory runs its own loop; Mint polls the cache (chosen)

The signatory is self-contained: `DbSignatory::new` spawns an internal task that
sleeps on a timer (cadence `min(interval, 60s)`) and rotates any keyset past the
interval, with no external driver. It only **signals** whether it may rotate
(`auto_rotation_config() -> Option<Duration>`). When that signal is set, the Mint
runs one task that fetches `keysets()` every second and mirrors the result into
its cache; the Mint never triggers rotation.

**Pros:**

* The signatory owns rotation end to end; the Mint is a pure reader.
* Single source of truth; the cache is a derived view. Autonomous, manual, and
  peer-replica rotations all surface on the next 1s poll.
* No new gRPC RPC: `keysets()` already exists; the remote client just returns
  `None` from `auto_rotation_config`.

**Cons:**

* The Mint polls `keysets()` once a second while rotation is enabled (a cheap
  in-process read for the embedded signatory; a small RPC for a remote one).

#### Option C: signatory pushes each rotation event

The internal loop notifies the Mint (a `watch`/`Notify`; server streaming for
remote gRPC) on each change, so the Mint refreshes only on an actual rotation
instead of polling.

**Pros:**

* No steady 1s poll; lowest cache-refresh latency.

**Cons:**

* A subscription plumbed through the trait and `embedded::Service` (and a
  streaming RPC for remote); the poll is already cheap and rotations are rare.

## Decision Outcome

Chosen options: **Concurrency = Option A (guard only)**, **Signatory scope =
Option A (embedded only)**, **Ownership/sync = Option B (signatory runs its own
loop; Mint polls the cache)**.

They give a faithful port that is correct for single-process and common
multi-replica cases, keeps rotation self-contained in the signatory with a single
source of truth, and needs no protobuf change and no cross-backend locking work.
The stricter alternatives (including the Option C push notification) are recorded
as deferred follow-ups.

Additional behavior settled by this decision:

* Rotate when a keyset is active, has a non-zero `valid_from`, and
  `now - valid_from >= interval`; Auth keysets excluded. CDK's typed `u64`
  `valid_from` makes Nutshell's "unparseable -> force rotation" branch moot
  (`0` means unknown and does not rotate). This predicate lives in the signatory
  (`db_signatory::should_rotate`), evaluated against `MintKeySetInfo`.
* Grace period: the new keyset's
  `final_expiry = old.final_expiry + (now - old.valid_from)`; if the old keyset
  has none, neither does the new one.
* `DbSignatory::new` returns `Self`; the caller wraps it in an `Arc` and calls
  `spawn_rotation()` once, which starts the signatory's own rotation loop: a task
  holding a `Weak<Self>` (no reference cycle), aborted on `Drop`, that wakes every
  `min(interval, 60s)`, reloads from the DB, and rotates stale keysets via the
  guarded `rotate_keyset`. It is a no-op when rotation is disabled.
* `Mint::start()` reads `signatory.auto_rotation_config()`; when it is `Some`
  it spawns one task that calls `refresh_keysets()` every second (first tick
  immediate), mirroring `signatory.keysets()` into the cache. It never triggers
  rotation. Failures are logged and never crash the task; the handle is joined
  by `stop()`.
* Config on `cdk-mintd`'s `Info`: `keyset_rotation_enabled`
  (`CDK_MINTD_MINT_KEYSET_ROTATION_ENABLED`, default `true`) and
  `keyset_rotation_interval_seconds`
  (`CDK_MINTD_MINT_KEYSET_ROTATION_INTERVAL_SECONDS`, default `7776000`, `> 0`),
  resolved by `MintBuilder::with_automatic_keyset_rotation` to an
  `Option<Duration>` that is passed to `DbSignatory::new` (the signatory owns
  it). A remote gRPC signatory owns its own rotation config instead.

### Positive Consequences

* Age-based rotation works out of the box (enabled, 90 days) with no manual
  derivation-path bumps; the active keyset is recovered from the DB on restart.
* Old keysets stay valid for redeeming existing proofs until their
  `final_expiry`; only the active set offered for new mints changes.
* The signatory rotates autonomously; the Mint cache is a poll-view of it, so
  autonomous, manual, and peer-replica rotations all converge into the cache.
* No protobuf or cross-backend database changes were required.

### Negative Consequences

* While rotation is enabled the Mint polls `keysets()` once a second; a push
  notification (Option C) would remove the steady poll but is deferred.
* A remote gRPC signatory builds but does not auto-rotate until the deferred
  protobuf work lands.
* Strict multi-replica serialization awaits the deferred keyset-table locking.

## Links

* Implemented by: `crates/cdk/src/mint/keysets/mod.rs`,
  `crates/cdk/src/mint/mod.rs`, `crates/cdk-signatory/src/db_signatory.rs`,
  `crates/cdk-signatory/src/signatory.rs`, `crates/cdk-mintd/src/config.rs`.
* Reference behavior: Nutshell `cashu/mint/keysets.py`,
  `tests/mint/test_mint_automatic_rotations.py`.
* Deferred follow-ups: push notification instead of the 1s poll (Option C);
  keyset-table `SELECT ... FOR UPDATE` locking (multi-replica); protobuf wiring
  for remote-signatory auto-rotation.
