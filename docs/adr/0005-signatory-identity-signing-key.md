# Mint identity key and signing of arbitrary payloads

* Status: accepted
* Authors: Cesar Rodas
* Date: 2026-08-17
* Targeted modules: cdk-signatory, cdk-mintd
* Associated tickets/PRs: https://github.com/cashubtc/nuts/pull/416

## Context and Problem Statement

The `Signatory` trait is the only seam through which the mint reaches
private-key material (ADR-0001), and every method on it is keyset-shaped:
`blind_sign`, `verify_proofs`, `keysets`, `subscribe_keysets`,
`rotate_keyset`. NUT-06 is being extended to require the mint to sign its own
info response, so the mint needs a signature over bytes that are not ecash.
Producing it outside the signatory would put a private key back inside the mint
process, which is exactly what ADR-0001 removed.

The mint already advertises a `pubkey`. In this codebase it came from
`Xpriv::new_master(seed)`, the BIP32 master key, and its private half signed
nothing. Two questions follow: which key should the mint's identity be, and
what shape should the signing call take?

## Decision Drivers

* The signing key must stay inside the signatory, like every other key.
* The identity is the pubkey the mint already advertises; NUT-06 makes that
  field mandatory, so there must not be a second one.
* The derivation must be reproducible from the seed alone, so instances sharing
  a seed agree and a restart is transparent (ADR-0004).
* The signing call must not constrain what the caller signs. Canonicalization
  is a protocol concern that belongs above the signatory.

## Considered Options

### Which key is the mint identity

#### Keep the BIP32 master key

Use `xpriv.private_key`, the key whose pubkey is already published.

**Pros:**

* Good, because existing deployments keep their advertised pubkey.

**Cons:**

* Bad, because it does not match NUT-06, which derives the identity key from
  the seed directly rather than through BIP32. Wallets cannot check the
  derivation, but that makes it the mint's obligation rather than a free
  choice: a mint carrying its seed to another implementation would land on a
  different identity.
* Bad, because that key is the root of the keyset derivation tree. This one is
  weaker than it looks: the keyset paths are fully hardened, so a child private
  key plus the parent public key leaks nothing about the parent, and BIP340 is
  EUF-CMA, so a signing oracle on the root recovers neither the root nor any
  child. What is left is defense in depth and more operations on the root key.

#### Derive with HMAC-SHA256, NUT-12 style

`HMAC_SHA256(key = seed, msg = purpose || counter)`, retrying with the next
counter if the digest falls outside `1..n-1`. This is the shape of NUT-12's
deterministic DLEQ nonce, which is the construction the NUT-06 review asked
for.

**Pros:**

* Good, because it is reproducible by any implementation holding the seed, so
  the mint's identity survives a move between implementations.
* Good, because it reuses a KDF shape already specified in Cashu rather than
  inventing one.
* Good, because the counter retry keeps the derivation total: every
  implementation lands on the same key without reducing modulo `n`, which
  `SHA256(seed)` alone leaves undefined.
* Good, because the identity key becomes a sibling of the keyset tree rather
  than its root.

**Cons:**

* Bad, because it changes the advertised pubkey of any mint that already has
  one. See the migration note below.
* Bad, because the NUT-06 proposal currently specifies `SHA256(seed)` with no
  retry. The HMAC form comes from the review thread, not the merged text.

### What the signing call takes

#### A 32-byte digest

**Pros:**

* Good, because the caller controls hashing entirely.

**Cons:**

* Bad, because it invites the caller to pre-hash and then be hashed again by
  the BIP340 helper, silently producing a signature nobody can verify.

#### The message bytes

**Pros:**

* Good, because `SecretKey::sign` computes `schnorr(SHA256(payload))`, which is
  exactly NUT-06's "SHA-256 hash of the canonical bytes, signed with BIP-340".
  The caller passes canonical bytes and the digest falls out correctly.
* Good, because there is one hashing step and one place it happens.

**Cons:**

* Bad, because a caller wanting to sign a digest it already holds cannot.

## Decision Outcome

Chosen options: "Derive with HMAC-SHA256, NUT-12 style" and "The message
bytes".

The derivation lives in `crates/cdk-signatory/src/identity.rs`:

```
digest = HMAC_SHA256(key = seed, data = b"Cashu_MintIdentity_v1" || ctr)
sk     = SecretKey::from_slice(digest)   // retry next ctr if outside 1..n-1
```

`ctr` is a single counter byte starting at `0x00`, so the byte range is also the
retry bound, exactly as in NUT-12. The NUT-13 tag `Cashu_KDF_HMAC_SHA256` is
deliberately not reused: it belongs to the secret and blinding-factor KDF, and
putting two purposes under one tag with the same HMAC key would leave the
separation resting on message length rather than content.

Test vector, generated outside this codebase and cross-checked against it:

```
seed   = "test-seed-for-identity"
sk     = 6f8874fe245baf7977e12b4ae0340af9c3f1d4fafe9ad9a8ab7d2ae5ee6359ad
pubkey = 0288398305e49ccf96eb12b60b8e45482baff87ca3fe9cf7ca176b7770e62c63a0
```

`SignatoryKeysets::pubkey` is this key's public half, and the mint continues to
persist it into `MintInfo.pubkey`. There is no second pubkey.

The trait gains `sign(payload) -> Signature`, BIP340 Schnorr over
`SHA256(payload)`, verified with `PublicKey::verify` against that pubkey. The
signatory applies no domain tag and no canonicalization: it signs what it is
given. NUT-06's JCS canonicalization and the `signature` and `time` fields are
a separate change, above this layer.

Because the signatory tags nothing, domain separation is the caller's
obligation. Every payload handed to `sign` must carry a purpose marker of its
own. A second untagged consumer of the identity key would make its signatures
interchangeable with NUT-06's, so there must not be one. A tag cannot be added
inside `sign` later without breaking NUT-06 verification.

On the wire this is a new `Sign` RPC. `CONSTANTS_SCHEMA_VERSION` moves from 2
to 3; the version interceptor rejects a mismatch, so a mint and a signatory
across the bump refuse to connect.

`cdk-mintd`'s `root_pubkey` (`config_service.rs`) recomputes the identity
pubkey locally to detect a changed signer. It now calls
`identity::derive_identity_key` rather than repeating a BIP32 derivation, so
the two cannot drift.

### Positive Consequences

* The mint can obtain a signature over arbitrary bytes without ever holding a
  private key.
* The identity key sits outside the keyset derivation tree, so using it as a
  signing oracle does not reach the keysets.
* One identity pubkey, derived one way, computed in one function.

### Negative Consequences

* **The advertised pubkey changes.** Every mint that already persisted a
  `MintInfo.pubkey` gets a new one. `Mint::new_internal` now rewrites a stored
  pubkey that is not the signatory's and logs both values, so an upgrade is
  transparent rather than leaving the mint advertising a key it cannot sign
  with. A config with an explicit `mint_info.pubkey` still fails startup, now
  with an error naming the derived key so the operator can fix the file.
* A mint and a signatory must be upgraded together: the schema version bump
  makes a mixed pair refuse to connect.
* The derivation follows the review discussion on the NUT-06 PR rather than its
  current text, which specifies `SHA256(seed)` with no retry. The exact HMAC
  layout is ours, so it has to be proposed on the PR with its vector before
  another implementation can match it. If the proposal lands differently, this
  derivation and the resulting pubkey change again.
* Two spec questions stay open and affect interoperability: NUT-06 says to
  encode the seed as UTF-8, which does not say what a mnemonic deployment
  holding a 64-byte BIP-39 seed should hash; and it mandates a 33-byte
  compressed pubkey while BIP340 verification is x-only, so `02||X` and `03||X`
  accept the same signatures.
* Nothing consumes `sign` yet. There is no `Mint::sign` pass-through; the first
  caller adds it.

## Links

* Refines [ADR-0001](0001-signatory-mint-key-segregation.md)
* Implements part of https://github.com/cashubtc/nuts/pull/416
