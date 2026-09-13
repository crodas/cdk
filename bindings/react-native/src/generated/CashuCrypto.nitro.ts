// GENERATED FILE.
// DO NOT EDIT.
//
// Produced by uniffi-bindgen-nitro from the UniFFI metadata of `cashu_ffi`.
// Change the Rust `#[uniffi::export]` surface and regenerate instead.

import type { HybridObject, Int64, UInt64 } from 'react-native-nitro-modules'

/**
 * NUT-11 signature flag.
 */
export type SigFlag = 'sigInputs' | 'sigAll'

/**
 * Result of blinding a single secret.
 */
export interface BlindPair {
  blindedSecret: string
  blindingFactor: string
}

/**
 * A blinded output plus the secrets needed to later unblind it.
 */
export interface BlindedOutput {
  amount: UInt64
  keysetId: string
  blindedSecret: string
  blindingFactor: string
  secret: string
  derivationIndex?: number
}

/**
 * A NUT-12 DLEQ proof as it appears on a blind signature.
 */
export interface DleqProof {
  e: string
  s: string
}

/**
 * One `amount -> mint public key` pair of a keyset.
 *
 * A list is used rather than a map because JavaScript object keys are strings,
 * which would silently narrow the u64 amount.
 */
export interface KeyEntry {
  amount: UInt64
  pubkey: string
}

/**
 * NUT-11 pay-to-public-key locking options.
 */
export interface P2pkOptions {
  pubkey: string
  additionalPubkeys?: string[]
  numSigs?: UInt64
  locktime?: UInt64
  refundPubkeys?: string[]
  numSigsRefund?: UInt64
  sigFlag: SigFlag
}

/**
 * Derives NUT-13 outputs for one keyset from a seed held in Rust.
 *
 * Keeping the seed here means a wallet copies it across the FFI once instead of
 * on every derivation.
 */
export interface DeterministicOutputFactory extends HybridObject<{ ios: 'c++'; android: 'c++' }> {
  /**
   * The keyset this factory derives for.
   */
  keysetId(): string
  /**
   * Deterministic outputs, one per denomination, walking `counter` upward.
   */
  outputs(amounts: UInt64[], counter: number): BlindedOutput[]
  /**
   * NUT-09 restore batch of blank outputs for counters `start..end`.
   */
  restoreBatch(startCounter: number, endCounter: number): BlindedOutput[]
  /**
   * A single deterministic output of exactly `amount` at `counter`.
   */
  singleOutput(amount: UInt64, counter: number): BlindedOutput
}

/**
 * Root native module. Every free function of the Rust crate hangs off it.
 */
export interface CashuCrypto extends HybridObject<{ ios: 'c++'; android: 'c++' }> {
  /**
   * Blind a secret, optionally with a caller supplied blinding factor.
   *
   * Omitting `blinding_factor` draws a fresh one from the system RNG.
   */
  blindMessage(secret: ArrayBuffer, blindingFactor?: ArrayBuffer): BlindPair
  /**
   * Blind a batch of secrets in one crossing.
   *
   * Exists because a wallet blinds one secret per output, and the round trip
   * costs more than the blinding for a single one.
   */
  blindMessages(secrets: ArrayBuffer[]): BlindPair[]
  /**
   * Bind a 64 byte BIP39 seed to one keyset.
   */
  createDeterministicOutputFactory(seed: ArrayBuffer, keysetId: string): DeterministicOutputFactory
  /**
   * NUT-13 deterministic secrets, one per denomination, walking `counter` upward.
   */
  createDeterministicOutputs(amounts: UInt64[], seed: ArrayBuffer, counter: number, keysetId: string): BlindedOutput[]
  /**
   * One P2PK locked secret per requested denomination.
   */
  createP2pkOutputs(p2pk: P2pkOptions, amounts: UInt64[], keysetId: string): BlindedOutput[]
  /**
   * One random secret per requested denomination.
   */
  createRandomOutputs(amounts: UInt64[], keysetId: string): BlindedOutput[]
  /**
   * NUT-09 restore batch: zero-amount outputs for counters `start..end`.
   */
  createRestoreOutputs(seed: ArrayBuffer, keysetId: string, startCounter: number, endCounter: number): BlindedOutput[]
  /**
   * A single NUT-13 deterministic output of exactly `amount`.
   */
  createSingleDeterministicOutput(amount: UInt64, seed: ArrayBuffer, counter: number, keysetId: string): BlindedOutput
  /**
   * A single P2PK locked output of exactly `amount`.
   */
  createSingleP2pkOutput(p2pk: P2pkOptions, amount: UInt64, keysetId: string): BlindedOutput
  /**
   * A single random output of exactly `amount`.
   */
  createSingleRandomOutput(amount: UInt64, keysetId: string): BlindedOutput
  /**
   * NUT-00 `hash_to_curve`, returning the compressed point.
   */
  hashToCurve(message: ArrayBuffer): ArrayBuffer
  /**
   * Compute the NUT-02 v1 keyset id for a set of mint keys.
   */
  keysetIdV1(keys: KeyEntry[]): string
  /**
   * SHA-256 of the input.
   *
   * Exists as the smallest possible end-to-end check of the binding pipeline.
   */
  sha256Digest(data: ArrayBuffer): ArrayBuffer
  /**
   * Split an amount over the denominations a keyset can sign.
   *
   * `custom_split` pins specific denominations; any remainder is split greedily.
   */
  splitAmount(amount: UInt64, denominations: UInt64[], customSplit?: UInt64[]): UInt64[]
  /**
   * NUT-00 unblinding: `C = C_ - r * K`.
   */
  unblindSignature(blindedSignature: string, blindingFactor: string, mintPubkey: string): string
  /**
   * Verify the NUT-12 DLEQ proof carried by an unblinded proof.
   *
   * A well-formed proof that does not verify returns `false`; only malformed
   * input raises.
   */
  verifyProofDleq(secret: string, unblindedSignature: string, dleq: DleqProof, blindingFactor: string, mintPubkey: string): boolean
}
