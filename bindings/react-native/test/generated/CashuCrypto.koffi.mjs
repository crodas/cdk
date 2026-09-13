// GENERATED FILE.
// DO NOT EDIT.
//
// Produced by uniffi-bindgen-nitro from the UniFFI metadata of `cashu_ffi`.
// Change the Rust `#[uniffi::export]` surface and regenerate instead.
//
// Test harness only. The shipped React Native path is Nitro over JSI;
// this exists so Node can exercise and benchmark the same Rust build.

import koffi from 'koffi'

const RUST_CALL_SUCCESS = 0
const RUST_CALL_ERROR = 1
const RUST_CALL_UNEXPECTED_ERROR = 2

const RustBufferType = koffi.struct('UniffiRustBuffer', {
  capacity: 'uint64',
  len: 'uint64',
  data: 'void *',
})
const ForeignBytesType = koffi.struct('UniffiForeignBytes', {
  len: 'int32',
  data: 'void *',
})
const RustCallStatusType = koffi.struct('UniffiRustCallStatus', {
  code: 'int8',
  errorBuf: RustBufferType,
})
const StatusOut = koffi.inout(koffi.pointer(RustCallStatusType))

/** Big-endian reader over the UniFFI buffer format. */
class Reader {
  constructor(bytes) {
    this.view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength)
    this.bytes = bytes
    this.position = 0
  }
  readI8() { const v = this.view.getInt8(this.position); this.position += 1; return v }
  readU8() { const v = this.view.getUint8(this.position); this.position += 1; return v }
  readBool() { return this.readI8() !== 0 }
  readI16() { const v = this.view.getInt16(this.position); this.position += 2; return v }
  readU16() { const v = this.view.getUint16(this.position); this.position += 2; return v }
  readI32() { const v = this.view.getInt32(this.position); this.position += 4; return v }
  readU32() { const v = this.view.getUint32(this.position); this.position += 4; return v }
  readI64() { const v = this.view.getBigInt64(this.position); this.position += 8; return v }
  readU64() { const v = this.view.getBigUint64(this.position); this.position += 8; return v }
  readF32() { const v = this.view.getFloat32(this.position); this.position += 4; return v }
  readF64() { const v = this.view.getFloat64(this.position); this.position += 8; return v }
  readLength() {
    const length = this.readI32()
    if (length < 0) throw new Error('uniffi buffer declares a negative length')
    return length
  }
  readBytes() {
    const length = this.readLength()
    const slice = this.bytes.subarray(this.position, this.position + length)
    this.position += length
    return new Uint8Array(slice)
  }
  readString() {
    return new TextDecoder().decode(this.readBytes())
  }
}

/** Big-endian writer producing the UniFFI buffer format. */
class Writer {
  constructor() {
    this.chunks = []
    this.size = 0
  }
  push(bytes) { this.chunks.push(bytes); this.size += bytes.length }
  scalar(width, write) {
    const buffer = new Uint8Array(width)
    write(new DataView(buffer.buffer))
    this.push(buffer)
  }
  writeI8(v) { this.scalar(1, (d) => d.setInt8(0, v)) }
  writeU8(v) { this.scalar(1, (d) => d.setUint8(0, v)) }
  writeBool(v) { this.writeI8(v ? 1 : 0) }
  writeI16(v) { this.scalar(2, (d) => d.setInt16(0, v)) }
  writeU16(v) { this.scalar(2, (d) => d.setUint16(0, v)) }
  writeI32(v) { this.scalar(4, (d) => d.setInt32(0, v)) }
  writeU32(v) { this.scalar(4, (d) => d.setUint32(0, v)) }
  writeI64(v) { this.scalar(8, (d) => d.setBigInt64(0, BigInt(v))) }
  writeU64(v) { this.scalar(8, (d) => d.setBigUint64(0, BigInt(v))) }
  writeF32(v) { this.scalar(4, (d) => d.setFloat32(0, v)) }
  writeF64(v) { this.scalar(8, (d) => d.setFloat64(0, v)) }
  writeLength(n) { this.writeI32(n) }
  writeBytes(bytes) { this.writeLength(bytes.length); this.push(bytes) }
  writeString(text) { this.writeBytes(new TextEncoder().encode(text)) }
  finish() {
    const out = new Uint8Array(this.size)
    let offset = 0
    for (const chunk of this.chunks) {
      out.set(chunk, offset)
      offset += chunk.length
    }
    return out
  }
}

function newStatus() {
  return { code: 0, errorBuf: { capacity: 0, len: 0, data: null } }
}

/** The Rust `CashuFfiError`, rebuilt from the structured payload. */
export class CashuFfiError extends Error {
  constructor(kind, message, fields) {
    super(message)
    this.name = 'CashuFfiError'
    this.kind = kind
    this.fields = fields
  }
}

function readBlindPair(reader) {
  return {
    blindedSecret: readString(reader),
    blindingFactor: readString(reader),
  }
}

function writeBlindPair(writer, value) {
  writeString(writer, value.blindedSecret)
  writeString(writer, value.blindingFactor)

}

function readBlindedOutput(reader) {
  return {
    amount: readU64(reader),
    keysetId: readString(reader),
    blindedSecret: readString(reader),
    blindingFactor: readString(reader),
    secret: readString(reader),
    derivationIndex: readOptU32(reader),
  }
}

function writeBlindedOutput(writer, value) {
  writeU64(writer, value.amount)
  writeString(writer, value.keysetId)
  writeString(writer, value.blindedSecret)
  writeString(writer, value.blindingFactor)
  writeString(writer, value.secret)
  writeOptU32(writer, value.derivationIndex)

}

function readBool(reader) {
  return reader.readBool()
}

function writeBool(writer, value) {
  writer.writeBool(value)
}

function readBytes(reader) {
  return reader.readBytes()
}

function writeBytes(writer, value) {
  writer.writeBytes(value)
}

function readDleqProof(reader) {
  return {
    e: readString(reader),
    s: readString(reader),
  }
}

function writeDleqProof(writer, value) {
  writeString(writer, value.e)
  writeString(writer, value.s)

}

function readKeyEntry(reader) {
  return {
    amount: readU64(reader),
    pubkey: readString(reader),
  }
}

function writeKeyEntry(writer, value) {
  writeU64(writer, value.amount)
  writeString(writer, value.pubkey)

}

function readOptBytes(reader) {
  if (reader.readI8() === 0) return undefined
  return readBytes(reader)
}

function writeOptBytes(writer, value) {
  if (value === undefined || value === null) { writer.writeI8(0); return }
  writer.writeI8(1)
  writeBytes(writer, value)
}

function readOptSeqString(reader) {
  if (reader.readI8() === 0) return undefined
  return readSeqString(reader)
}

function writeOptSeqString(writer, value) {
  if (value === undefined || value === null) { writer.writeI8(0); return }
  writer.writeI8(1)
  writeSeqString(writer, value)
}

function readOptSeqU64(reader) {
  if (reader.readI8() === 0) return undefined
  return readSeqU64(reader)
}

function writeOptSeqU64(writer, value) {
  if (value === undefined || value === null) { writer.writeI8(0); return }
  writer.writeI8(1)
  writeSeqU64(writer, value)
}

function readOptU32(reader) {
  if (reader.readI8() === 0) return undefined
  return readU32(reader)
}

function writeOptU32(writer, value) {
  if (value === undefined || value === null) { writer.writeI8(0); return }
  writer.writeI8(1)
  writeU32(writer, value)
}

function readOptU64(reader) {
  if (reader.readI8() === 0) return undefined
  return readU64(reader)
}

function writeOptU64(writer, value) {
  if (value === undefined || value === null) { writer.writeI8(0); return }
  writer.writeI8(1)
  writeU64(writer, value)
}

function readP2pkOptions(reader) {
  return {
    pubkey: readString(reader),
    additionalPubkeys: readOptSeqString(reader),
    numSigs: readOptU64(reader),
    locktime: readOptU64(reader),
    refundPubkeys: readOptSeqString(reader),
    numSigsRefund: readOptU64(reader),
    sigFlag: readSigFlag(reader),
  }
}

function writeP2pkOptions(writer, value) {
  writeString(writer, value.pubkey)
  writeOptSeqString(writer, value.additionalPubkeys)
  writeOptU64(writer, value.numSigs)
  writeOptU64(writer, value.locktime)
  writeOptSeqString(writer, value.refundPubkeys)
  writeOptU64(writer, value.numSigsRefund)
  writeSigFlag(writer, value.sigFlag)

}

function readSeqBlindPair(reader) {
  const count = reader.readLength()
  const items = new Array(count)
  for (let i = 0; i < count; i++) items[i] = readBlindPair(reader)
  return items
}

function writeSeqBlindPair(writer, value) {
  writer.writeLength(value.length)
  for (const item of value) writeBlindPair(writer, item)
}

function readSeqBlindedOutput(reader) {
  const count = reader.readLength()
  const items = new Array(count)
  for (let i = 0; i < count; i++) items[i] = readBlindedOutput(reader)
  return items
}

function writeSeqBlindedOutput(writer, value) {
  writer.writeLength(value.length)
  for (const item of value) writeBlindedOutput(writer, item)
}

function readSeqBytes(reader) {
  const count = reader.readLength()
  const items = new Array(count)
  for (let i = 0; i < count; i++) items[i] = readBytes(reader)
  return items
}

function writeSeqBytes(writer, value) {
  writer.writeLength(value.length)
  for (const item of value) writeBytes(writer, item)
}

function readSeqKeyEntry(reader) {
  const count = reader.readLength()
  const items = new Array(count)
  for (let i = 0; i < count; i++) items[i] = readKeyEntry(reader)
  return items
}

function writeSeqKeyEntry(writer, value) {
  writer.writeLength(value.length)
  for (const item of value) writeKeyEntry(writer, item)
}

function readSeqString(reader) {
  const count = reader.readLength()
  const items = new Array(count)
  for (let i = 0; i < count; i++) items[i] = readString(reader)
  return items
}

function writeSeqString(writer, value) {
  writer.writeLength(value.length)
  for (const item of value) writeString(writer, item)
}

function readSeqU64(reader) {
  const count = reader.readLength()
  const items = new Array(count)
  for (let i = 0; i < count; i++) items[i] = readU64(reader)
  return items
}

function writeSeqU64(writer, value) {
  writer.writeLength(value.length)
  for (const item of value) writeU64(writer, item)
}

function readSigFlag(reader) {
  const tag = reader.readI32()
  switch (tag) {
    case 1: return 'sigInputs'
    case 2: return 'sigAll'
  }
  throw new Error(`unknown SigFlag variant tag ${tag}`)
}

function writeSigFlag(writer, value) {
  switch (value) {
    case 'sigInputs': writer.writeI32(1); return
    case 'sigAll': writer.writeI32(2); return
  }
  throw new Error(`unknown SigFlag value ${value}`)
}

function readString(reader) {
  return reader.readString()
}

function writeString(writer, value) {
  writer.writeString(value)
}

function readU32(reader) {
  return reader.readU32()
}

function writeU32(writer, value) {
  writer.writeU32(value)
}

function readU64(reader) {
  return reader.readU64()
}

function writeU64(writer, value) {
  writer.writeU64(value)
}

/** Load the Rust library and return the generated API. */
export function load(libraryPath) {
  const lib = koffi.load(libraryPath)
  const fn = {}
  fn['uniffi_cashu_ffi_fn_func_blind_message'] = lib.func('uniffi_cashu_ffi_fn_func_blind_message', RustBufferType, [RustBufferType, RustBufferType, StatusOut])
  fn['uniffi_cashu_ffi_fn_func_blind_messages'] = lib.func('uniffi_cashu_ffi_fn_func_blind_messages', RustBufferType, [RustBufferType, StatusOut])
  fn['uniffi_cashu_ffi_fn_constructor_deterministicoutputfactory_new'] = lib.func('uniffi_cashu_ffi_fn_constructor_deterministicoutputfactory_new', 'uint64', [RustBufferType, RustBufferType, StatusOut])
  fn['uniffi_cashu_ffi_fn_func_create_deterministic_outputs'] = lib.func('uniffi_cashu_ffi_fn_func_create_deterministic_outputs', RustBufferType, [RustBufferType, RustBufferType, 'uint32', RustBufferType, StatusOut])
  fn['uniffi_cashu_ffi_fn_func_create_p2pk_outputs'] = lib.func('uniffi_cashu_ffi_fn_func_create_p2pk_outputs', RustBufferType, [RustBufferType, RustBufferType, RustBufferType, StatusOut])
  fn['uniffi_cashu_ffi_fn_func_create_random_outputs'] = lib.func('uniffi_cashu_ffi_fn_func_create_random_outputs', RustBufferType, [RustBufferType, RustBufferType, StatusOut])
  fn['uniffi_cashu_ffi_fn_func_create_restore_outputs'] = lib.func('uniffi_cashu_ffi_fn_func_create_restore_outputs', RustBufferType, [RustBufferType, RustBufferType, 'uint32', 'uint32', StatusOut])
  fn['uniffi_cashu_ffi_fn_func_create_single_deterministic_output'] = lib.func('uniffi_cashu_ffi_fn_func_create_single_deterministic_output', RustBufferType, ['uint64', RustBufferType, 'uint32', RustBufferType, StatusOut])
  fn['uniffi_cashu_ffi_fn_func_create_single_p2pk_output'] = lib.func('uniffi_cashu_ffi_fn_func_create_single_p2pk_output', RustBufferType, [RustBufferType, 'uint64', RustBufferType, StatusOut])
  fn['uniffi_cashu_ffi_fn_func_create_single_random_output'] = lib.func('uniffi_cashu_ffi_fn_func_create_single_random_output', RustBufferType, ['uint64', RustBufferType, StatusOut])
  fn['uniffi_cashu_ffi_fn_func_hash_to_curve'] = lib.func('uniffi_cashu_ffi_fn_func_hash_to_curve', RustBufferType, [RustBufferType, StatusOut])
  fn['uniffi_cashu_ffi_fn_func_keyset_id_v1'] = lib.func('uniffi_cashu_ffi_fn_func_keyset_id_v1', RustBufferType, [RustBufferType, StatusOut])
  fn['uniffi_cashu_ffi_fn_func_sha256_digest'] = lib.func('uniffi_cashu_ffi_fn_func_sha256_digest', RustBufferType, [RustBufferType, StatusOut])
  fn['uniffi_cashu_ffi_fn_func_split_amount'] = lib.func('uniffi_cashu_ffi_fn_func_split_amount', RustBufferType, ['uint64', RustBufferType, RustBufferType, StatusOut])
  fn['uniffi_cashu_ffi_fn_func_unblind_signature'] = lib.func('uniffi_cashu_ffi_fn_func_unblind_signature', RustBufferType, [RustBufferType, RustBufferType, RustBufferType, StatusOut])
  fn['uniffi_cashu_ffi_fn_func_verify_proof_dleq'] = lib.func('uniffi_cashu_ffi_fn_func_verify_proof_dleq', 'int8', [RustBufferType, RustBufferType, RustBufferType, RustBufferType, RustBufferType, StatusOut])
  fn['uniffi_cashu_ffi_fn_method_deterministicoutputfactory_keyset_id'] = lib.func('uniffi_cashu_ffi_fn_method_deterministicoutputfactory_keyset_id', RustBufferType, ['uint64', StatusOut])
  fn['uniffi_cashu_ffi_fn_method_deterministicoutputfactory_outputs'] = lib.func('uniffi_cashu_ffi_fn_method_deterministicoutputfactory_outputs', RustBufferType, ['uint64', RustBufferType, 'uint32', StatusOut])
  fn['uniffi_cashu_ffi_fn_method_deterministicoutputfactory_restore_batch'] = lib.func('uniffi_cashu_ffi_fn_method_deterministicoutputfactory_restore_batch', RustBufferType, ['uint64', 'uint32', 'uint32', StatusOut])
  fn['uniffi_cashu_ffi_fn_method_deterministicoutputfactory_single_output'] = lib.func('uniffi_cashu_ffi_fn_method_deterministicoutputfactory_single_output', RustBufferType, ['uint64', 'uint64', 'uint32', StatusOut])
  fn['uniffi_cashu_ffi_fn_free_deterministicoutputfactory'] = lib.func('uniffi_cashu_ffi_fn_free_deterministicoutputfactory', 'void', ['uint64', StatusOut])
  fn['uniffi_cashu_ffi_fn_clone_deterministicoutputfactory'] = lib.func('uniffi_cashu_ffi_fn_clone_deterministicoutputfactory', 'uint64', ['uint64', StatusOut])
  fn['ffi_cashu_ffi_rustbuffer_from_bytes'] = lib.func('ffi_cashu_ffi_rustbuffer_from_bytes', RustBufferType, [ForeignBytesType, StatusOut])
  fn['ffi_cashu_ffi_rustbuffer_free'] = lib.func('ffi_cashu_ffi_rustbuffer_free', 'void', [RustBufferType, StatusOut])
  fn['ffi_cashu_ffi_uniffi_contract_version'] = lib.func('ffi_cashu_ffi_uniffi_contract_version', 'uint32', [])
  fn['uniffi_cashu_ffi_checksum_func_blind_message'] = lib.func('uniffi_cashu_ffi_checksum_func_blind_message', 'uint16', [])
  fn['uniffi_cashu_ffi_checksum_func_blind_messages'] = lib.func('uniffi_cashu_ffi_checksum_func_blind_messages', 'uint16', [])
  fn['uniffi_cashu_ffi_checksum_func_create_deterministic_outputs'] = lib.func('uniffi_cashu_ffi_checksum_func_create_deterministic_outputs', 'uint16', [])
  fn['uniffi_cashu_ffi_checksum_func_create_p2pk_outputs'] = lib.func('uniffi_cashu_ffi_checksum_func_create_p2pk_outputs', 'uint16', [])
  fn['uniffi_cashu_ffi_checksum_func_create_random_outputs'] = lib.func('uniffi_cashu_ffi_checksum_func_create_random_outputs', 'uint16', [])
  fn['uniffi_cashu_ffi_checksum_func_create_restore_outputs'] = lib.func('uniffi_cashu_ffi_checksum_func_create_restore_outputs', 'uint16', [])
  fn['uniffi_cashu_ffi_checksum_func_create_single_deterministic_output'] = lib.func('uniffi_cashu_ffi_checksum_func_create_single_deterministic_output', 'uint16', [])
  fn['uniffi_cashu_ffi_checksum_func_create_single_p2pk_output'] = lib.func('uniffi_cashu_ffi_checksum_func_create_single_p2pk_output', 'uint16', [])
  fn['uniffi_cashu_ffi_checksum_func_create_single_random_output'] = lib.func('uniffi_cashu_ffi_checksum_func_create_single_random_output', 'uint16', [])
  fn['uniffi_cashu_ffi_checksum_func_hash_to_curve'] = lib.func('uniffi_cashu_ffi_checksum_func_hash_to_curve', 'uint16', [])
  fn['uniffi_cashu_ffi_checksum_func_keyset_id_v1'] = lib.func('uniffi_cashu_ffi_checksum_func_keyset_id_v1', 'uint16', [])
  fn['uniffi_cashu_ffi_checksum_func_sha256_digest'] = lib.func('uniffi_cashu_ffi_checksum_func_sha256_digest', 'uint16', [])
  fn['uniffi_cashu_ffi_checksum_func_split_amount'] = lib.func('uniffi_cashu_ffi_checksum_func_split_amount', 'uint16', [])
  fn['uniffi_cashu_ffi_checksum_func_unblind_signature'] = lib.func('uniffi_cashu_ffi_checksum_func_unblind_signature', 'uint16', [])
  fn['uniffi_cashu_ffi_checksum_func_verify_proof_dleq'] = lib.func('uniffi_cashu_ffi_checksum_func_verify_proof_dleq', 'uint16', [])
  fn['uniffi_cashu_ffi_checksum_method_deterministicoutputfactory_keyset_id'] = lib.func('uniffi_cashu_ffi_checksum_method_deterministicoutputfactory_keyset_id', 'uint16', [])
  fn['uniffi_cashu_ffi_checksum_method_deterministicoutputfactory_outputs'] = lib.func('uniffi_cashu_ffi_checksum_method_deterministicoutputfactory_outputs', 'uint16', [])
  fn['uniffi_cashu_ffi_checksum_method_deterministicoutputfactory_restore_batch'] = lib.func('uniffi_cashu_ffi_checksum_method_deterministicoutputfactory_restore_batch', 'uint16', [])
  fn['uniffi_cashu_ffi_checksum_method_deterministicoutputfactory_single_output'] = lib.func('uniffi_cashu_ffi_checksum_method_deterministicoutputfactory_single_output', 'uint16', [])
  fn['uniffi_cashu_ffi_checksum_constructor_deterministicoutputfactory_new'] = lib.func('uniffi_cashu_ffi_checksum_constructor_deterministicoutputfactory_new', 'uint16', [])

  if (fn['ffi_cashu_ffi_uniffi_contract_version']() !== 30) throw new Error('uniffi ABI mismatch: rebuild the bindings')
  if (fn['uniffi_cashu_ffi_checksum_func_blind_message']() !== 14388) throw new Error('uniffi checksum mismatch for uniffi_cashu_ffi_checksum_func_blind_message: rebuild the bindings')
  if (fn['uniffi_cashu_ffi_checksum_func_blind_messages']() !== 1071) throw new Error('uniffi checksum mismatch for uniffi_cashu_ffi_checksum_func_blind_messages: rebuild the bindings')
  if (fn['uniffi_cashu_ffi_checksum_func_create_deterministic_outputs']() !== 48451) throw new Error('uniffi checksum mismatch for uniffi_cashu_ffi_checksum_func_create_deterministic_outputs: rebuild the bindings')
  if (fn['uniffi_cashu_ffi_checksum_func_create_p2pk_outputs']() !== 47646) throw new Error('uniffi checksum mismatch for uniffi_cashu_ffi_checksum_func_create_p2pk_outputs: rebuild the bindings')
  if (fn['uniffi_cashu_ffi_checksum_func_create_random_outputs']() !== 32634) throw new Error('uniffi checksum mismatch for uniffi_cashu_ffi_checksum_func_create_random_outputs: rebuild the bindings')
  if (fn['uniffi_cashu_ffi_checksum_func_create_restore_outputs']() !== 65503) throw new Error('uniffi checksum mismatch for uniffi_cashu_ffi_checksum_func_create_restore_outputs: rebuild the bindings')
  if (fn['uniffi_cashu_ffi_checksum_func_create_single_deterministic_output']() !== 60767) throw new Error('uniffi checksum mismatch for uniffi_cashu_ffi_checksum_func_create_single_deterministic_output: rebuild the bindings')
  if (fn['uniffi_cashu_ffi_checksum_func_create_single_p2pk_output']() !== 41235) throw new Error('uniffi checksum mismatch for uniffi_cashu_ffi_checksum_func_create_single_p2pk_output: rebuild the bindings')
  if (fn['uniffi_cashu_ffi_checksum_func_create_single_random_output']() !== 53694) throw new Error('uniffi checksum mismatch for uniffi_cashu_ffi_checksum_func_create_single_random_output: rebuild the bindings')
  if (fn['uniffi_cashu_ffi_checksum_func_hash_to_curve']() !== 62146) throw new Error('uniffi checksum mismatch for uniffi_cashu_ffi_checksum_func_hash_to_curve: rebuild the bindings')
  if (fn['uniffi_cashu_ffi_checksum_func_keyset_id_v1']() !== 63806) throw new Error('uniffi checksum mismatch for uniffi_cashu_ffi_checksum_func_keyset_id_v1: rebuild the bindings')
  if (fn['uniffi_cashu_ffi_checksum_func_sha256_digest']() !== 34561) throw new Error('uniffi checksum mismatch for uniffi_cashu_ffi_checksum_func_sha256_digest: rebuild the bindings')
  if (fn['uniffi_cashu_ffi_checksum_func_split_amount']() !== 65348) throw new Error('uniffi checksum mismatch for uniffi_cashu_ffi_checksum_func_split_amount: rebuild the bindings')
  if (fn['uniffi_cashu_ffi_checksum_func_unblind_signature']() !== 55990) throw new Error('uniffi checksum mismatch for uniffi_cashu_ffi_checksum_func_unblind_signature: rebuild the bindings')
  if (fn['uniffi_cashu_ffi_checksum_func_verify_proof_dleq']() !== 47466) throw new Error('uniffi checksum mismatch for uniffi_cashu_ffi_checksum_func_verify_proof_dleq: rebuild the bindings')
  if (fn['uniffi_cashu_ffi_checksum_method_deterministicoutputfactory_keyset_id']() !== 6974) throw new Error('uniffi checksum mismatch for uniffi_cashu_ffi_checksum_method_deterministicoutputfactory_keyset_id: rebuild the bindings')
  if (fn['uniffi_cashu_ffi_checksum_method_deterministicoutputfactory_outputs']() !== 34621) throw new Error('uniffi checksum mismatch for uniffi_cashu_ffi_checksum_method_deterministicoutputfactory_outputs: rebuild the bindings')
  if (fn['uniffi_cashu_ffi_checksum_method_deterministicoutputfactory_restore_batch']() !== 19625) throw new Error('uniffi checksum mismatch for uniffi_cashu_ffi_checksum_method_deterministicoutputfactory_restore_batch: rebuild the bindings')
  if (fn['uniffi_cashu_ffi_checksum_method_deterministicoutputfactory_single_output']() !== 24845) throw new Error('uniffi checksum mismatch for uniffi_cashu_ffi_checksum_method_deterministicoutputfactory_single_output: rebuild the bindings')
  if (fn['uniffi_cashu_ffi_checksum_constructor_deterministicoutputfactory_new']() !== 31949) throw new Error('uniffi checksum mismatch for uniffi_cashu_ffi_checksum_constructor_deterministicoutputfactory_new: rebuild the bindings')

  function freeBuffer(buffer) {
    if (buffer.data === null) return
    const status = newStatus()
    fn['ffi_cashu_ffi_rustbuffer_free'](buffer, status)
  }

  function consumeBuffer(buffer) {
    const length = Number(buffer.len)
    const bytes = length > 0 ? new Uint8Array(koffi.view(buffer.data, length).slice(0)) : new Uint8Array(0)
    freeBuffer(buffer)
    return bytes
  }

  function toBuffer(bytes) {
    const status = newStatus()
    const buffer = fn['ffi_cashu_ffi_rustbuffer_from_bytes']({ len: bytes.length, data: bytes }, status)
    if (status.code !== RUST_CALL_SUCCESS) throw new Error('uniffi could not allocate a buffer')
    return buffer
  }

  function lower(write, value) {
    const writer = new Writer()
    write(writer, value)
    return toBuffer(writer.finish())
  }

  function lift(read, buffer) {
    return read(new Reader(consumeBuffer(buffer)))
  }

  function stringToBuffer(text) {
    return toBuffer(new TextEncoder().encode(text))
  }

  function bufferToString(buffer) {
    return new TextDecoder().decode(consumeBuffer(buffer))
  }

  function checkUnexpected(status) {
    if (status.code === RUST_CALL_UNEXPECTED_ERROR) {
      throw new Error('rust panicked: ' + bufferToString(status.errorBuf))
    }
  }

  function throwCashuFfiError(buffer) {
    const reader = new Reader(consumeBuffer(buffer))
    const tag = reader.readI32()
    switch (tag) {
      case 1: {
        const field = readString(reader)
        const reason = readString(reader)
        throw new CashuFfiError('InvalidHex', `InvalidHex(${['field=' + field, 'reason=' + reason].join(', ')})`, { field: String(field), reason: String(reason) })
      }
      case 2: {
        const field = readString(reader)
        const reason = readString(reader)
        throw new CashuFfiError('InvalidPublicKey', `InvalidPublicKey(${['field=' + field, 'reason=' + reason].join(', ')})`, { field: String(field), reason: String(reason) })
      }
      case 3: {
        const field = readString(reader)
        const reason = readString(reader)
        throw new CashuFfiError('InvalidSecretKey', `InvalidSecretKey(${['field=' + field, 'reason=' + reason].join(', ')})`, { field: String(field), reason: String(reason) })
      }
      case 4: {
        const id = readString(reader)
        const reason = readString(reader)
        throw new CashuFfiError('InvalidKeysetId', `InvalidKeysetId(${['id=' + id, 'reason=' + reason].join(', ')})`, { id: String(id), reason: String(reason) })
      }
      case 5: {
        const length = readU64(reader)
        throw new CashuFfiError('InvalidSeedLength', `InvalidSeedLength(${['length=' + length].join(', ')})`, { length: String(length) })
      }
      case 6: {
        const reason = readString(reader)
        throw new CashuFfiError('Split', `Split(${['reason=' + reason].join(', ')})`, { reason: String(reason) })
      }
      case 7: {
        const reason = readString(reader)
        throw new CashuFfiError('Dhke', `Dhke(${['reason=' + reason].join(', ')})`, { reason: String(reason) })
      }
      case 8: {
        const reason = readString(reader)
        throw new CashuFfiError('SpendingConditions', `SpendingConditions(${['reason=' + reason].join(', ')})`, { reason: String(reason) })
      }
      case 9: {
        const reason = readString(reader)
        throw new CashuFfiError('Dleq', `Dleq(${['reason=' + reason].join(', ')})`, { reason: String(reason) })
      }
      case 10: {
        const counter = readU32(reader)
        const reason = readString(reader)
        throw new CashuFfiError('Derivation', `Derivation(${['counter=' + counter, 'reason=' + reason].join(', ')})`, { counter: String(counter), reason: String(reason) })
      }
    }
    throw new CashuFfiError('Unknown', 'unknown variant', {})
  }

  class DeterministicOutputFactory {
    constructor(handle) { this.handle = handle }

    cloneHandle() {
      if (this.handle === 0) throw new Error('use of a native object after close()')
      const status = newStatus()
      const cloned = fn['uniffi_cashu_ffi_fn_clone_deterministicoutputfactory'](this.handle, status)
      checkUnexpected(status)
      return cloned
    }

    close() {
      if (this.handle === 0) return
      const status = newStatus()
      fn['uniffi_cashu_ffi_fn_free_deterministicoutputfactory'](this.handle, status)
      this.handle = 0
    }

    keysetId() {
      const status = newStatus()
      const raw = fn['uniffi_cashu_ffi_fn_method_deterministicoutputfactory_keyset_id'](this.cloneHandle(), status)
      checkUnexpected(status)
      return bufferToString(raw)
    }

    outputs(amounts, counter) {
      const lowered_amounts = lower(writeSeqU64, amounts)
      const lowered_counter = counter
      const status = newStatus()
      const raw = fn['uniffi_cashu_ffi_fn_method_deterministicoutputfactory_outputs'](this.cloneHandle(), lowered_amounts, lowered_counter, status)
      if (status.code === RUST_CALL_ERROR) throwCashuFfiError(status.errorBuf)
      checkUnexpected(status)
      return lift(readSeqBlindedOutput, raw)
    }

    restoreBatch(startCounter, endCounter) {
      const lowered_startCounter = startCounter
      const lowered_endCounter = endCounter
      const status = newStatus()
      const raw = fn['uniffi_cashu_ffi_fn_method_deterministicoutputfactory_restore_batch'](this.cloneHandle(), lowered_startCounter, lowered_endCounter, status)
      if (status.code === RUST_CALL_ERROR) throwCashuFfiError(status.errorBuf)
      checkUnexpected(status)
      return lift(readSeqBlindedOutput, raw)
    }

    singleOutput(amount, counter) {
      const lowered_amount = BigInt(amount)
      const lowered_counter = counter
      const status = newStatus()
      const raw = fn['uniffi_cashu_ffi_fn_method_deterministicoutputfactory_single_output'](this.cloneHandle(), lowered_amount, lowered_counter, status)
      if (status.code === RUST_CALL_ERROR) throwCashuFfiError(status.errorBuf)
      checkUnexpected(status)
      return lift(readBlindedOutput, raw)
    }
  }

  return {
    blindMessage: (secret, blindingFactor) => {
      const lowered_secret = lower(writeBytes, secret)
      const lowered_blindingFactor = lower(writeOptBytes, blindingFactor)
      const status = newStatus()
      const raw = fn['uniffi_cashu_ffi_fn_func_blind_message'](lowered_secret, lowered_blindingFactor, status)
      if (status.code === RUST_CALL_ERROR) throwCashuFfiError(status.errorBuf)
      checkUnexpected(status)
      return lift(readBlindPair, raw)
    },
    blindMessages: (secrets) => {
      const lowered_secrets = lower(writeSeqBytes, secrets)
      const status = newStatus()
      const raw = fn['uniffi_cashu_ffi_fn_func_blind_messages'](lowered_secrets, status)
      if (status.code === RUST_CALL_ERROR) throwCashuFfiError(status.errorBuf)
      checkUnexpected(status)
      return lift(readSeqBlindPair, raw)
    },
    createDeterministicOutputFactory: (seed, keysetId) => {
      const lowered_seed = lower(writeBytes, seed)
      const lowered_keysetId = stringToBuffer(keysetId)
      const status = newStatus()
      const raw = fn['uniffi_cashu_ffi_fn_constructor_deterministicoutputfactory_new'](lowered_seed, lowered_keysetId, status)
      if (status.code === RUST_CALL_ERROR) throwCashuFfiError(status.errorBuf)
      checkUnexpected(status)
      return new DeterministicOutputFactory(raw)
    },
    createDeterministicOutputs: (amounts, seed, counter, keysetId) => {
      const lowered_amounts = lower(writeSeqU64, amounts)
      const lowered_seed = lower(writeBytes, seed)
      const lowered_counter = counter
      const lowered_keysetId = stringToBuffer(keysetId)
      const status = newStatus()
      const raw = fn['uniffi_cashu_ffi_fn_func_create_deterministic_outputs'](lowered_amounts, lowered_seed, lowered_counter, lowered_keysetId, status)
      if (status.code === RUST_CALL_ERROR) throwCashuFfiError(status.errorBuf)
      checkUnexpected(status)
      return lift(readSeqBlindedOutput, raw)
    },
    createP2pkOutputs: (p2pk, amounts, keysetId) => {
      const lowered_p2pk = lower(writeP2pkOptions, p2pk)
      const lowered_amounts = lower(writeSeqU64, amounts)
      const lowered_keysetId = stringToBuffer(keysetId)
      const status = newStatus()
      const raw = fn['uniffi_cashu_ffi_fn_func_create_p2pk_outputs'](lowered_p2pk, lowered_amounts, lowered_keysetId, status)
      if (status.code === RUST_CALL_ERROR) throwCashuFfiError(status.errorBuf)
      checkUnexpected(status)
      return lift(readSeqBlindedOutput, raw)
    },
    createRandomOutputs: (amounts, keysetId) => {
      const lowered_amounts = lower(writeSeqU64, amounts)
      const lowered_keysetId = stringToBuffer(keysetId)
      const status = newStatus()
      const raw = fn['uniffi_cashu_ffi_fn_func_create_random_outputs'](lowered_amounts, lowered_keysetId, status)
      if (status.code === RUST_CALL_ERROR) throwCashuFfiError(status.errorBuf)
      checkUnexpected(status)
      return lift(readSeqBlindedOutput, raw)
    },
    createRestoreOutputs: (seed, keysetId, startCounter, endCounter) => {
      const lowered_seed = lower(writeBytes, seed)
      const lowered_keysetId = stringToBuffer(keysetId)
      const lowered_startCounter = startCounter
      const lowered_endCounter = endCounter
      const status = newStatus()
      const raw = fn['uniffi_cashu_ffi_fn_func_create_restore_outputs'](lowered_seed, lowered_keysetId, lowered_startCounter, lowered_endCounter, status)
      if (status.code === RUST_CALL_ERROR) throwCashuFfiError(status.errorBuf)
      checkUnexpected(status)
      return lift(readSeqBlindedOutput, raw)
    },
    createSingleDeterministicOutput: (amount, seed, counter, keysetId) => {
      const lowered_amount = BigInt(amount)
      const lowered_seed = lower(writeBytes, seed)
      const lowered_counter = counter
      const lowered_keysetId = stringToBuffer(keysetId)
      const status = newStatus()
      const raw = fn['uniffi_cashu_ffi_fn_func_create_single_deterministic_output'](lowered_amount, lowered_seed, lowered_counter, lowered_keysetId, status)
      if (status.code === RUST_CALL_ERROR) throwCashuFfiError(status.errorBuf)
      checkUnexpected(status)
      return lift(readBlindedOutput, raw)
    },
    createSingleP2pkOutput: (p2pk, amount, keysetId) => {
      const lowered_p2pk = lower(writeP2pkOptions, p2pk)
      const lowered_amount = BigInt(amount)
      const lowered_keysetId = stringToBuffer(keysetId)
      const status = newStatus()
      const raw = fn['uniffi_cashu_ffi_fn_func_create_single_p2pk_output'](lowered_p2pk, lowered_amount, lowered_keysetId, status)
      if (status.code === RUST_CALL_ERROR) throwCashuFfiError(status.errorBuf)
      checkUnexpected(status)
      return lift(readBlindedOutput, raw)
    },
    createSingleRandomOutput: (amount, keysetId) => {
      const lowered_amount = BigInt(amount)
      const lowered_keysetId = stringToBuffer(keysetId)
      const status = newStatus()
      const raw = fn['uniffi_cashu_ffi_fn_func_create_single_random_output'](lowered_amount, lowered_keysetId, status)
      if (status.code === RUST_CALL_ERROR) throwCashuFfiError(status.errorBuf)
      checkUnexpected(status)
      return lift(readBlindedOutput, raw)
    },
    hashToCurve: (message) => {
      const lowered_message = lower(writeBytes, message)
      const status = newStatus()
      const raw = fn['uniffi_cashu_ffi_fn_func_hash_to_curve'](lowered_message, status)
      if (status.code === RUST_CALL_ERROR) throwCashuFfiError(status.errorBuf)
      checkUnexpected(status)
      return lift(readBytes, raw)
    },
    keysetIdV1: (keys) => {
      const lowered_keys = lower(writeSeqKeyEntry, keys)
      const status = newStatus()
      const raw = fn['uniffi_cashu_ffi_fn_func_keyset_id_v1'](lowered_keys, status)
      if (status.code === RUST_CALL_ERROR) throwCashuFfiError(status.errorBuf)
      checkUnexpected(status)
      return bufferToString(raw)
    },
    sha256Digest: (data) => {
      const lowered_data = lower(writeBytes, data)
      const status = newStatus()
      const raw = fn['uniffi_cashu_ffi_fn_func_sha256_digest'](lowered_data, status)
      checkUnexpected(status)
      return lift(readBytes, raw)
    },
    splitAmount: (amount, denominations, customSplit) => {
      const lowered_amount = BigInt(amount)
      const lowered_denominations = lower(writeSeqU64, denominations)
      const lowered_customSplit = lower(writeOptSeqU64, customSplit)
      const status = newStatus()
      const raw = fn['uniffi_cashu_ffi_fn_func_split_amount'](lowered_amount, lowered_denominations, lowered_customSplit, status)
      if (status.code === RUST_CALL_ERROR) throwCashuFfiError(status.errorBuf)
      checkUnexpected(status)
      return lift(readSeqU64, raw)
    },
    unblindSignature: (blindedSignature, blindingFactor, mintPubkey) => {
      const lowered_blindedSignature = stringToBuffer(blindedSignature)
      const lowered_blindingFactor = stringToBuffer(blindingFactor)
      const lowered_mintPubkey = stringToBuffer(mintPubkey)
      const status = newStatus()
      const raw = fn['uniffi_cashu_ffi_fn_func_unblind_signature'](lowered_blindedSignature, lowered_blindingFactor, lowered_mintPubkey, status)
      if (status.code === RUST_CALL_ERROR) throwCashuFfiError(status.errorBuf)
      checkUnexpected(status)
      return bufferToString(raw)
    },
    verifyProofDleq: (secret, unblindedSignature, dleq, blindingFactor, mintPubkey) => {
      const lowered_secret = stringToBuffer(secret)
      const lowered_unblindedSignature = stringToBuffer(unblindedSignature)
      const lowered_dleq = lower(writeDleqProof, dleq)
      const lowered_blindingFactor = stringToBuffer(blindingFactor)
      const lowered_mintPubkey = stringToBuffer(mintPubkey)
      const status = newStatus()
      const raw = fn['uniffi_cashu_ffi_fn_func_verify_proof_dleq'](lowered_secret, lowered_unblindedSignature, lowered_dleq, lowered_blindingFactor, lowered_mintPubkey, status)
      if (status.code === RUST_CALL_ERROR) throwCashuFfiError(status.errorBuf)
      checkUnexpected(status)
      return (raw !== 0)
    },
  }
}
