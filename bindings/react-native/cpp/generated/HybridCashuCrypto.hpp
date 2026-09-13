// GENERATED FILE.
// DO NOT EDIT.
//
// Produced by uniffi-bindgen-nitro from the UniFFI metadata of `cashu_ffi`.
// Change the Rust `#[uniffi::export]` surface and regenerate instead.

#pragma once

#include "HybridCashuCryptoSpec.hpp"
#include "HybridDeterministicOutputFactorySpec.hpp"
#include "CashuCryptoBridge.hpp"

#include <memory>
#include <optional>
#include <string>
#include <vector>

namespace margelo::nitro::cashucrypto {

/// Root native module. Owns no state; every call goes straight to Rust.
class HybridCashuCrypto final : public HybridCashuCryptoSpec {
public:
  HybridCashuCrypto() : HybridObject(TAG) {
    ::cashucrypto::bridge::assertAbiCompatible();
  }

  BlindPair blindMessage(const std::shared_ptr<ArrayBuffer>& secret, const std::optional<std::shared_ptr<ArrayBuffer>>& blindingFactor) override;
  std::vector<BlindPair> blindMessages(const std::vector<std::shared_ptr<ArrayBuffer>>& secrets) override;
  std::shared_ptr<HybridDeterministicOutputFactorySpec> createDeterministicOutputFactory(const std::shared_ptr<ArrayBuffer>& seed, const std::string& keysetId) override;
  std::vector<BlindedOutput> createDeterministicOutputs(const std::vector<uint64_t>& amounts, const std::shared_ptr<ArrayBuffer>& seed, double counter, const std::string& keysetId) override;
  std::vector<BlindedOutput> createP2pkOutputs(const P2pkOptions& p2pk, const std::vector<uint64_t>& amounts, const std::string& keysetId) override;
  std::vector<BlindedOutput> createRandomOutputs(const std::vector<uint64_t>& amounts, const std::string& keysetId) override;
  std::vector<BlindedOutput> createRestoreOutputs(const std::shared_ptr<ArrayBuffer>& seed, const std::string& keysetId, double startCounter, double endCounter) override;
  BlindedOutput createSingleDeterministicOutput(uint64_t amount, const std::shared_ptr<ArrayBuffer>& seed, double counter, const std::string& keysetId) override;
  BlindedOutput createSingleP2pkOutput(const P2pkOptions& p2pk, uint64_t amount, const std::string& keysetId) override;
  BlindedOutput createSingleRandomOutput(uint64_t amount, const std::string& keysetId) override;
  std::shared_ptr<ArrayBuffer> hashToCurve(const std::shared_ptr<ArrayBuffer>& message) override;
  std::string keysetIdV1(const std::vector<KeyEntry>& keys) override;
  std::shared_ptr<ArrayBuffer> sha256Digest(const std::shared_ptr<ArrayBuffer>& data) override;
  std::vector<uint64_t> splitAmount(uint64_t amount, const std::vector<uint64_t>& denominations, const std::optional<std::vector<uint64_t>>& customSplit) override;
  std::string unblindSignature(const std::string& blindedSignature, const std::string& blindingFactor, const std::string& mintPubkey) override;
  bool verifyProofDleq(const std::string& secret, const std::string& unblindedSignature, const DleqProof& dleq, const std::string& blindingFactor, const std::string& mintPubkey) override;
};

} // namespace margelo::nitro::cashucrypto
