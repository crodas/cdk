// GENERATED FILE.
// DO NOT EDIT.
//
// Produced by uniffi-bindgen-nitro from the UniFFI metadata of `cashu_ffi`.
// Change the Rust `#[uniffi::export]` surface and regenerate instead.

#pragma once

#include <cstdint>
#include <memory>
#include <optional>
#include <stdexcept>
#include <string>
#include <unordered_map>
#include <vector>

namespace cashucrypto::bridge {

/// ABI contract version the Rust library was generated against.
inline constexpr uint32_t kUniffiContractVersion = 30;

/// Aborts at startup if the loaded library is not the one this was generated from.
void assertAbiCompatible();

/// Errors returned by every fallible export in this crate.
/// 
/// Variants keep their fields so foreign callers can branch on the cause instead
/// of matching on a formatted string.
enum class CashuFfiErrorKind : int32_t {
  InvalidHex = 1,
  InvalidPublicKey = 2,
  InvalidSecretKey = 3,
  InvalidKeysetId = 4,
  InvalidSeedLength = 5,
  Split = 6,
  Dhke = 7,
  SpendingConditions = 8,
  Dleq = 9,
  Derivation = 10,
};

/// The Rust `CashuFfiError` as a C++ exception.
///
/// `what()` carries the whole variant as JSON so the TypeScript layer can
/// rebuild a typed error; `kind()` is there for C++ callers.
class CashuFfiError final : public std::runtime_error {
public:
  CashuFfiError(CashuFfiErrorKind kind, const std::string& payload)
    : std::runtime_error(payload), kind_(kind) {}

  CashuFfiErrorKind kind() const noexcept { return kind_; }

private:
  CashuFfiErrorKind kind_;
};

/// NUT-11 signature flag.
enum class SigFlag : int32_t {
  SigInputs = 1,
  SigAll = 2,
};

/// Result of blinding a single secret.
struct BlindPair final {
  std::string blindedSecret;
  std::string blindingFactor;
};

/// A blinded output plus the secrets needed to later unblind it.
struct BlindedOutput final {
  uint64_t amount;
  std::string keysetId;
  std::string blindedSecret;
  std::string blindingFactor;
  std::string secret;
  std::optional<uint32_t> derivationIndex;
};

/// A NUT-12 DLEQ proof as it appears on a blind signature.
struct DleqProof final {
  std::string e;
  std::string s;
};

/// One `amount -> mint public key` pair of a keyset.
/// 
/// A list is used rather than a map because JavaScript object keys are strings,
/// which would silently narrow the u64 amount.
struct KeyEntry final {
  uint64_t amount;
  std::string pubkey;
};

/// NUT-11 pay-to-public-key locking options.
struct P2pkOptions final {
  std::string pubkey;
  std::optional<std::vector<std::string>> additionalPubkeys;
  std::optional<uint64_t> numSigs;
  std::optional<uint64_t> locktime;
  std::optional<std::vector<std::string>> refundPubkeys;
  std::optional<uint64_t> numSigsRefund;
  SigFlag sigFlag;
};

/// Derives NUT-13 outputs for one keyset from a seed held in Rust.
/// 
/// Keeping the seed here means a wallet copies it across the FFI once instead of
/// on every derivation.
/// Owns the UniFFI handle for the Rust `DeterministicOutputFactory`.
class DeterministicOutputFactory final {
public:
  explicit DeterministicOutputFactory(uint64_t handle) noexcept : handle_(handle) {}
  ~DeterministicOutputFactory() { close(); }
  DeterministicOutputFactory(const DeterministicOutputFactory&) = delete;
  DeterministicOutputFactory& operator=(const DeterministicOutputFactory&) = delete;
  DeterministicOutputFactory(DeterministicOutputFactory&& other) noexcept : handle_(other.handle_) { other.handle_ = 0; }

  /// Drop the Rust reference. Safe to call more than once.
  void close() noexcept;

  /// The raw handle, or zero once closed.
  uint64_t handle() const noexcept { return handle_; }

  /// A fresh owned handle.
  ///
  /// UniFFI consumes the receiver of every method call, so each call
  /// hands it a clone rather than this object's own reference.
  uint64_t cloneHandle() const;

  /// The keyset this factory derives for.
  std::string keysetId() const;
  /// Deterministic outputs, one per denomination, walking `counter` upward.
  std::vector<BlindedOutput> outputs(const std::vector<uint64_t>& amounts, uint32_t counter) const;
  /// NUT-09 restore batch of blank outputs for counters `start..end`.
  std::vector<BlindedOutput> restoreBatch(uint32_t startCounter, uint32_t endCounter) const;
  /// A single deterministic output of exactly `amount` at `counter`.
  BlindedOutput singleOutput(uint64_t amount, uint32_t counter) const;

private:
  uint64_t handle_;
};

// Free functions and object constructors.
/// Blind a secret, optionally with a caller supplied blinding factor.
/// 
/// Omitting `blinding_factor` draws a fresh one from the system RNG.
BlindPair blindMessage(const std::vector<uint8_t>& secret, const std::optional<std::vector<uint8_t>>& blindingFactor);
/// Blind a batch of secrets in one crossing.
/// 
/// Exists because a wallet blinds one secret per output, and the round trip
/// costs more than the blinding for a single one.
std::vector<BlindPair> blindMessages(const std::vector<std::vector<uint8_t>>& secrets);
/// Bind a 64 byte BIP39 seed to one keyset.
std::shared_ptr<DeterministicOutputFactory> createDeterministicOutputFactory(const std::vector<uint8_t>& seed, const std::string& keysetId);
/// NUT-13 deterministic secrets, one per denomination, walking `counter` upward.
std::vector<BlindedOutput> createDeterministicOutputs(const std::vector<uint64_t>& amounts, const std::vector<uint8_t>& seed, uint32_t counter, const std::string& keysetId);
/// One P2PK locked secret per requested denomination.
std::vector<BlindedOutput> createP2pkOutputs(const P2pkOptions& p2pk, const std::vector<uint64_t>& amounts, const std::string& keysetId);
/// One random secret per requested denomination.
std::vector<BlindedOutput> createRandomOutputs(const std::vector<uint64_t>& amounts, const std::string& keysetId);
/// NUT-09 restore batch: zero-amount outputs for counters `start..end`.
std::vector<BlindedOutput> createRestoreOutputs(const std::vector<uint8_t>& seed, const std::string& keysetId, uint32_t startCounter, uint32_t endCounter);
/// A single NUT-13 deterministic output of exactly `amount`.
BlindedOutput createSingleDeterministicOutput(uint64_t amount, const std::vector<uint8_t>& seed, uint32_t counter, const std::string& keysetId);
/// A single P2PK locked output of exactly `amount`.
BlindedOutput createSingleP2pkOutput(const P2pkOptions& p2pk, uint64_t amount, const std::string& keysetId);
/// A single random output of exactly `amount`.
BlindedOutput createSingleRandomOutput(uint64_t amount, const std::string& keysetId);
/// NUT-00 `hash_to_curve`, returning the compressed point.
std::vector<uint8_t> hashToCurve(const std::vector<uint8_t>& message);
/// Compute the NUT-02 v1 keyset id for a set of mint keys.
std::string keysetIdV1(const std::vector<KeyEntry>& keys);
/// SHA-256 of the input.
/// 
/// Exists as the smallest possible end-to-end check of the binding pipeline.
std::vector<uint8_t> sha256Digest(const std::vector<uint8_t>& data);
/// Split an amount over the denominations a keyset can sign.
/// 
/// `custom_split` pins specific denominations; any remainder is split greedily.
std::vector<uint64_t> splitAmount(uint64_t amount, const std::vector<uint64_t>& denominations, const std::optional<std::vector<uint64_t>>& customSplit);
/// NUT-00 unblinding: `C = C_ - r * K`.
std::string unblindSignature(const std::string& blindedSignature, const std::string& blindingFactor, const std::string& mintPubkey);
/// Verify the NUT-12 DLEQ proof carried by an unblinded proof.
/// 
/// A well-formed proof that does not verify returns `false`; only malformed
/// input raises.
bool verifyProofDleq(const std::string& secret, const std::string& unblindedSignature, const DleqProof& dleq, const std::string& blindingFactor, const std::string& mintPubkey);

} // namespace cashucrypto::bridge
