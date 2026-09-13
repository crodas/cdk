// GENERATED FILE.
// DO NOT EDIT.
//
// Produced by uniffi-bindgen-nitro from the UniFFI metadata of `cashu_ffi`.
// Change the Rust `#[uniffi::export]` surface and regenerate instead.

#pragma once

#include <cstdint>

namespace cashucrypto::ffi {

extern "C" {

/// Mirrors `uniffi_core::RustBuffer`: a Vec<u8> handed over to the caller.
struct RustBuffer {
  uint64_t capacity;
  uint64_t len;
  uint8_t* data;
};

/// Mirrors `uniffi_core::ForeignBytes`: bytes the caller keeps alive.
struct ForeignBytes {
  int32_t len;
  const uint8_t* data;
};

/// Mirrors `uniffi_core::RustCallStatus`.
struct RustCallStatus {
  int8_t code;
  RustBuffer errorBuf;
};

/// `RustCallStatusCode` values.
enum RustCallStatusCode : int8_t {
  RUST_CALL_SUCCESS = 0,
  RUST_CALL_ERROR = 1,
  RUST_CALL_UNEXPECTED_ERROR = 2,
  RUST_CALL_CANCELLED = 3,
};

uint64_t uniffi_cashu_ffi_fn_clone_deterministicoutputfactory(uint64_t handle, RustCallStatus* uniffiOutStatus);
void uniffi_cashu_ffi_fn_free_deterministicoutputfactory(uint64_t handle, RustCallStatus* uniffiOutStatus);
uint64_t uniffi_cashu_ffi_fn_constructor_deterministicoutputfactory_new(RustBuffer seed, RustBuffer keyset_id, RustCallStatus* uniffiOutStatus);
RustBuffer uniffi_cashu_ffi_fn_method_deterministicoutputfactory_keyset_id(uint64_t ptr, RustCallStatus* uniffiOutStatus);
RustBuffer uniffi_cashu_ffi_fn_method_deterministicoutputfactory_outputs(uint64_t ptr, RustBuffer amounts, uint32_t counter, RustCallStatus* uniffiOutStatus);
RustBuffer uniffi_cashu_ffi_fn_method_deterministicoutputfactory_restore_batch(uint64_t ptr, uint32_t start_counter, uint32_t end_counter, RustCallStatus* uniffiOutStatus);
RustBuffer uniffi_cashu_ffi_fn_method_deterministicoutputfactory_single_output(uint64_t ptr, uint64_t amount, uint32_t counter, RustCallStatus* uniffiOutStatus);
RustBuffer uniffi_cashu_ffi_fn_func_blind_message(RustBuffer secret, RustBuffer blinding_factor, RustCallStatus* uniffiOutStatus);
RustBuffer uniffi_cashu_ffi_fn_func_blind_messages(RustBuffer secrets, RustCallStatus* uniffiOutStatus);
RustBuffer uniffi_cashu_ffi_fn_func_create_deterministic_outputs(RustBuffer amounts, RustBuffer seed, uint32_t counter, RustBuffer keyset_id, RustCallStatus* uniffiOutStatus);
RustBuffer uniffi_cashu_ffi_fn_func_create_p2pk_outputs(RustBuffer p2pk, RustBuffer amounts, RustBuffer keyset_id, RustCallStatus* uniffiOutStatus);
RustBuffer uniffi_cashu_ffi_fn_func_create_random_outputs(RustBuffer amounts, RustBuffer keyset_id, RustCallStatus* uniffiOutStatus);
RustBuffer uniffi_cashu_ffi_fn_func_create_restore_outputs(RustBuffer seed, RustBuffer keyset_id, uint32_t start_counter, uint32_t end_counter, RustCallStatus* uniffiOutStatus);
RustBuffer uniffi_cashu_ffi_fn_func_create_single_deterministic_output(uint64_t amount, RustBuffer seed, uint32_t counter, RustBuffer keyset_id, RustCallStatus* uniffiOutStatus);
RustBuffer uniffi_cashu_ffi_fn_func_create_single_p2pk_output(RustBuffer p2pk, uint64_t amount, RustBuffer keyset_id, RustCallStatus* uniffiOutStatus);
RustBuffer uniffi_cashu_ffi_fn_func_create_single_random_output(uint64_t amount, RustBuffer keyset_id, RustCallStatus* uniffiOutStatus);
RustBuffer uniffi_cashu_ffi_fn_func_hash_to_curve(RustBuffer message, RustCallStatus* uniffiOutStatus);
RustBuffer uniffi_cashu_ffi_fn_func_keyset_id_v1(RustBuffer keys, RustCallStatus* uniffiOutStatus);
RustBuffer uniffi_cashu_ffi_fn_func_sha256_digest(RustBuffer data, RustCallStatus* uniffiOutStatus);
RustBuffer uniffi_cashu_ffi_fn_func_split_amount(uint64_t amount, RustBuffer denominations, RustBuffer custom_split, RustCallStatus* uniffiOutStatus);
RustBuffer uniffi_cashu_ffi_fn_func_unblind_signature(RustBuffer blinded_signature, RustBuffer blinding_factor, RustBuffer mint_pubkey, RustCallStatus* uniffiOutStatus);
int8_t uniffi_cashu_ffi_fn_func_verify_proof_dleq(RustBuffer secret, RustBuffer unblinded_signature, RustBuffer dleq, RustBuffer blinding_factor, RustBuffer mint_pubkey, RustCallStatus* uniffiOutStatus);
RustBuffer ffi_cashu_ffi_rustbuffer_alloc(uint64_t size, RustCallStatus* uniffiOutStatus);
RustBuffer ffi_cashu_ffi_rustbuffer_from_bytes(ForeignBytes bytes, RustCallStatus* uniffiOutStatus);
void ffi_cashu_ffi_rustbuffer_free(RustBuffer buf, RustCallStatus* uniffiOutStatus);
uint16_t uniffi_cashu_ffi_checksum_func_blind_message(void);
uint16_t uniffi_cashu_ffi_checksum_func_blind_messages(void);
uint16_t uniffi_cashu_ffi_checksum_func_create_deterministic_outputs(void);
uint16_t uniffi_cashu_ffi_checksum_func_create_p2pk_outputs(void);
uint16_t uniffi_cashu_ffi_checksum_func_create_random_outputs(void);
uint16_t uniffi_cashu_ffi_checksum_func_create_restore_outputs(void);
uint16_t uniffi_cashu_ffi_checksum_func_create_single_deterministic_output(void);
uint16_t uniffi_cashu_ffi_checksum_func_create_single_p2pk_output(void);
uint16_t uniffi_cashu_ffi_checksum_func_create_single_random_output(void);
uint16_t uniffi_cashu_ffi_checksum_func_hash_to_curve(void);
uint16_t uniffi_cashu_ffi_checksum_func_keyset_id_v1(void);
uint16_t uniffi_cashu_ffi_checksum_func_sha256_digest(void);
uint16_t uniffi_cashu_ffi_checksum_func_split_amount(void);
uint16_t uniffi_cashu_ffi_checksum_func_unblind_signature(void);
uint16_t uniffi_cashu_ffi_checksum_func_verify_proof_dleq(void);
uint16_t uniffi_cashu_ffi_checksum_method_deterministicoutputfactory_keyset_id(void);
uint16_t uniffi_cashu_ffi_checksum_method_deterministicoutputfactory_outputs(void);
uint16_t uniffi_cashu_ffi_checksum_method_deterministicoutputfactory_restore_batch(void);
uint16_t uniffi_cashu_ffi_checksum_method_deterministicoutputfactory_single_output(void);
uint16_t uniffi_cashu_ffi_checksum_constructor_deterministicoutputfactory_new(void);
uint32_t ffi_cashu_ffi_uniffi_contract_version(void);

} // extern "C"

} // namespace cashucrypto::ffi
