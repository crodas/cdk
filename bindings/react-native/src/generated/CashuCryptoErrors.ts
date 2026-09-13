// GENERATED FILE.
// DO NOT EDIT.
//
// Produced by uniffi-bindgen-nitro from the UniFFI metadata of `cashu_ffi`.
// Change the Rust `#[uniffi::export]` surface and regenerate instead.

/** Discriminant of CashuFfiError, matching the Rust variant names. */
export type CashuFfiErrorKind =
  | 'InvalidHex'
  | 'InvalidPublicKey'
  | 'InvalidSecretKey'
  | 'InvalidKeysetId'
  | 'InvalidSeedLength'
  | 'Split'
  | 'Dhke'
  | 'SpendingConditions'
  | 'Dleq'
  | 'Derivation';

/** A `CashuFfiError` raised by the native module. */
export class CashuFfiError extends Error {
  readonly kind: CashuFfiErrorKind
  readonly fields: Readonly<Record<string, unknown>>

  constructor(kind: CashuFfiErrorKind, message: string, fields: Record<string, unknown>) {
    super(message)
    this.name = 'CashuFfiError'
    this.kind = kind
    this.fields = Object.freeze(fields)
  }

  /** Narrow to the `InvalidHex` variant. */
  isInvalidHex(): this is CashuFfiError & { fields: { field: string; reason: string } } {
    return this.kind === 'InvalidHex'
  }

  /** Narrow to the `InvalidPublicKey` variant. */
  isInvalidPublicKey(): this is CashuFfiError & { fields: { field: string; reason: string } } {
    return this.kind === 'InvalidPublicKey'
  }

  /** Narrow to the `InvalidSecretKey` variant. */
  isInvalidSecretKey(): this is CashuFfiError & { fields: { field: string; reason: string } } {
    return this.kind === 'InvalidSecretKey'
  }

  /** Narrow to the `InvalidKeysetId` variant. */
  isInvalidKeysetId(): this is CashuFfiError & { fields: { id: string; reason: string } } {
    return this.kind === 'InvalidKeysetId'
  }

  /** Narrow to the `InvalidSeedLength` variant. */
  isInvalidSeedLength(): this is CashuFfiError & { fields: { length: string } } {
    return this.kind === 'InvalidSeedLength'
  }

  /** Narrow to the `Split` variant. */
  isSplit(): this is CashuFfiError & { fields: { reason: string } } {
    return this.kind === 'Split'
  }

  /** Narrow to the `Dhke` variant. */
  isDhke(): this is CashuFfiError & { fields: { reason: string } } {
    return this.kind === 'Dhke'
  }

  /** Narrow to the `SpendingConditions` variant. */
  isSpendingConditions(): this is CashuFfiError & { fields: { reason: string } } {
    return this.kind === 'SpendingConditions'
  }

  /** Narrow to the `Dleq` variant. */
  isDleq(): this is CashuFfiError & { fields: { reason: string } } {
    return this.kind === 'Dleq'
  }

  /** Narrow to the `Derivation` variant. */
  isDerivation(): this is CashuFfiError & { fields: { counter: number; reason: string } } {
    return this.kind === 'Derivation'
  }
}

/** Marker the native adapter prefixes structured errors with. */
const NATIVE_ERROR_PREFIX = 'uniffi-nitro-error:'

/**
 * Rebuild a typed error from whatever the native module threw.
 *
 * Anything that is not a structured native error is returned unchanged, so a
 * genuine JavaScript failure is never disguised as a protocol error.
 */
export function toNativeError(thrown: unknown): unknown {
  const message = thrown instanceof Error ? thrown.message : String(thrown)
  const start = message.indexOf(NATIVE_ERROR_PREFIX)
  if (start < 0) return thrown
  let payload: { type?: string; kind?: string; message?: string; fields?: Record<string, unknown> }
  try {
    payload = JSON.parse(message.slice(start + NATIVE_ERROR_PREFIX.length))
  } catch {
    return thrown
  }
  switch (payload.type) {
    case 'CashuFfiError':
      return new CashuFfiError(payload.kind as CashuFfiErrorKind, payload.message ?? message, payload.fields ?? {})
    default:
      return thrown
  }
}

/** Run `call`, converting any structured native error it throws. */
export function withNativeErrors<T>(call: () => T): T {
  try {
    return call()
  } catch (thrown) {
    throw toNativeError(thrown)
  }
}
