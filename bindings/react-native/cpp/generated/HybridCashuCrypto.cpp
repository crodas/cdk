// GENERATED FILE.
// DO NOT EDIT.
//
// Produced by uniffi-bindgen-nitro from the UniFFI metadata of `cashu_ffi`.
// Change the Rust `#[uniffi::export]` surface and regenerate instead.

#include "HybridCashuCrypto.hpp"
#include "HybridDeterministicOutputFactory.hpp"

#include <NitroModules/ArrayBuffer.hpp>
#include <NitroModules/Promise.hpp>

#include <utility>

namespace margelo::nitro::cashucrypto {

namespace {

::cashucrypto::bridge::SigFlag toBridgeSigFlag(SigFlag value);
SigFlag fromBridgeSigFlag(::cashucrypto::bridge::SigFlag value);

::cashucrypto::bridge::BlindPair toBridgeBlindPair(const BlindPair& value) {
  ::cashucrypto::bridge::BlindPair out;
  out.blindedSecret = value.blindedSecret;
  out.blindingFactor = value.blindingFactor;
  return out;
}

BlindPair fromBridgeBlindPair(const ::cashucrypto::bridge::BlindPair& value) {
  return BlindPair(value.blindedSecret, value.blindingFactor);
}

::cashucrypto::bridge::BlindedOutput toBridgeBlindedOutput(const BlindedOutput& value) {
  ::cashucrypto::bridge::BlindedOutput out;
  out.amount = value.amount;
  out.keysetId = value.keysetId;
  out.blindedSecret = value.blindedSecret;
  out.blindingFactor = value.blindingFactor;
  out.secret = value.secret;
  out.derivationIndex = ([&]{ const auto& src0 = (value.derivationIndex); std::optional<uint32_t> out0; if (src0.has_value()) { out0 = static_cast<uint32_t>((*src0)); } return out0; }());
  return out;
}

BlindedOutput fromBridgeBlindedOutput(const ::cashucrypto::bridge::BlindedOutput& value) {
  return BlindedOutput(value.amount, value.keysetId, value.blindedSecret, value.blindingFactor, value.secret, ([&]{ const auto& src0 = (value.derivationIndex); std::optional<double> out0; if (src0.has_value()) { out0 = static_cast<double>((*src0)); } return out0; }()));
}

::cashucrypto::bridge::DleqProof toBridgeDleqProof(const DleqProof& value) {
  ::cashucrypto::bridge::DleqProof out;
  out.e = value.e;
  out.s = value.s;
  return out;
}

DleqProof fromBridgeDleqProof(const ::cashucrypto::bridge::DleqProof& value) {
  return DleqProof(value.e, value.s);
}

::cashucrypto::bridge::KeyEntry toBridgeKeyEntry(const KeyEntry& value) {
  ::cashucrypto::bridge::KeyEntry out;
  out.amount = value.amount;
  out.pubkey = value.pubkey;
  return out;
}

KeyEntry fromBridgeKeyEntry(const ::cashucrypto::bridge::KeyEntry& value) {
  return KeyEntry(value.amount, value.pubkey);
}

::cashucrypto::bridge::P2pkOptions toBridgeP2pkOptions(const P2pkOptions& value) {
  ::cashucrypto::bridge::P2pkOptions out;
  out.pubkey = value.pubkey;
  out.additionalPubkeys = ([&]{ const auto& src0 = (value.additionalPubkeys); std::optional<std::vector<std::string>> out0; if (src0.has_value()) { out0 = ([&]{ const auto& src1 = ((*src0)); std::vector<std::string> out1; out1.reserve(src1.size()); for (const auto& item1 : src1) { out1.push_back(item1); } return out1; }()); } return out0; }());
  out.numSigs = ([&]{ const auto& src0 = (value.numSigs); std::optional<uint64_t> out0; if (src0.has_value()) { out0 = (*src0); } return out0; }());
  out.locktime = ([&]{ const auto& src0 = (value.locktime); std::optional<uint64_t> out0; if (src0.has_value()) { out0 = (*src0); } return out0; }());
  out.refundPubkeys = ([&]{ const auto& src0 = (value.refundPubkeys); std::optional<std::vector<std::string>> out0; if (src0.has_value()) { out0 = ([&]{ const auto& src1 = ((*src0)); std::vector<std::string> out1; out1.reserve(src1.size()); for (const auto& item1 : src1) { out1.push_back(item1); } return out1; }()); } return out0; }());
  out.numSigsRefund = ([&]{ const auto& src0 = (value.numSigsRefund); std::optional<uint64_t> out0; if (src0.has_value()) { out0 = (*src0); } return out0; }());
  out.sigFlag = toBridgeSigFlag(value.sigFlag);
  return out;
}

P2pkOptions fromBridgeP2pkOptions(const ::cashucrypto::bridge::P2pkOptions& value) {
  return P2pkOptions(value.pubkey, ([&]{ const auto& src0 = (value.additionalPubkeys); std::optional<std::vector<std::string>> out0; if (src0.has_value()) { out0 = ([&]{ const auto& src1 = ((*src0)); std::vector<std::string> out1; out1.reserve(src1.size()); for (const auto& item1 : src1) { out1.push_back(item1); } return out1; }()); } return out0; }()), ([&]{ const auto& src0 = (value.numSigs); std::optional<uint64_t> out0; if (src0.has_value()) { out0 = (*src0); } return out0; }()), ([&]{ const auto& src0 = (value.locktime); std::optional<uint64_t> out0; if (src0.has_value()) { out0 = (*src0); } return out0; }()), ([&]{ const auto& src0 = (value.refundPubkeys); std::optional<std::vector<std::string>> out0; if (src0.has_value()) { out0 = ([&]{ const auto& src1 = ((*src0)); std::vector<std::string> out1; out1.reserve(src1.size()); for (const auto& item1 : src1) { out1.push_back(item1); } return out1; }()); } return out0; }()), ([&]{ const auto& src0 = (value.numSigsRefund); std::optional<uint64_t> out0; if (src0.has_value()) { out0 = (*src0); } return out0; }()), fromBridgeSigFlag(value.sigFlag));
}

::cashucrypto::bridge::SigFlag toBridgeSigFlag(SigFlag value) {
  switch (value) {
    case SigFlag::SIGINPUTS: return ::cashucrypto::bridge::SigFlag::SigInputs;
    case SigFlag::SIGALL: return ::cashucrypto::bridge::SigFlag::SigAll;
  }
  throw std::runtime_error("unknown SigFlag value");
}

SigFlag fromBridgeSigFlag(::cashucrypto::bridge::SigFlag value) {
  switch (value) {
    case ::cashucrypto::bridge::SigFlag::SigInputs: return SigFlag::SIGINPUTS;
    case ::cashucrypto::bridge::SigFlag::SigAll: return SigFlag::SIGALL;
  }
  throw std::runtime_error("unknown SigFlag value");
}

} // namespace

BlindPair HybridCashuCrypto::blindMessage(const std::shared_ptr<ArrayBuffer>& secret, const std::optional<std::shared_ptr<ArrayBuffer>>& blindingFactor) {
  auto bridge_secret = ([&]{ const auto& src0 = (secret); return std::vector<uint8_t>(src0->data(), src0->data() + src0->size()); }());
  auto bridge_blindingFactor = ([&]{ const auto& src0 = (blindingFactor); std::optional<std::vector<uint8_t>> out0; if (src0.has_value()) { out0 = ([&]{ const auto& src1 = ((*src0)); return std::vector<uint8_t>(src1->data(), src1->data() + src1->size()); }()); } return out0; }());
  auto result = ::cashucrypto::bridge::blindMessage(bridge_secret, bridge_blindingFactor);
  return fromBridgeBlindPair(result);
}

std::vector<BlindPair> HybridCashuCrypto::blindMessages(const std::vector<std::shared_ptr<ArrayBuffer>>& secrets) {
  auto bridge_secrets = ([&]{ const auto& src0 = (secrets); std::vector<std::vector<uint8_t>> out0; out0.reserve(src0.size()); for (const auto& item0 : src0) { out0.push_back(([&]{ const auto& src1 = (item0); return std::vector<uint8_t>(src1->data(), src1->data() + src1->size()); }())); } return out0; }());
  auto result = ::cashucrypto::bridge::blindMessages(bridge_secrets);
  return ([&]{ const auto& src0 = (result); std::vector<BlindPair> out0; out0.reserve(src0.size()); for (const auto& item0 : src0) { out0.push_back(fromBridgeBlindPair(item0)); } return out0; }());
}

std::shared_ptr<HybridDeterministicOutputFactorySpec> HybridCashuCrypto::createDeterministicOutputFactory(const std::shared_ptr<ArrayBuffer>& seed, const std::string& keysetId) {
  auto bridge_seed = ([&]{ const auto& src0 = (seed); return std::vector<uint8_t>(src0->data(), src0->data() + src0->size()); }());
  auto bridge_keysetId = keysetId;
  auto result = ::cashucrypto::bridge::createDeterministicOutputFactory(bridge_seed, bridge_keysetId);
  return std::make_shared<HybridDeterministicOutputFactory>(std::move(result));
}

std::vector<BlindedOutput> HybridCashuCrypto::createDeterministicOutputs(const std::vector<uint64_t>& amounts, const std::shared_ptr<ArrayBuffer>& seed, double counter, const std::string& keysetId) {
  auto bridge_amounts = ([&]{ const auto& src0 = (amounts); std::vector<uint64_t> out0; out0.reserve(src0.size()); for (const auto& item0 : src0) { out0.push_back(item0); } return out0; }());
  auto bridge_seed = ([&]{ const auto& src0 = (seed); return std::vector<uint8_t>(src0->data(), src0->data() + src0->size()); }());
  auto bridge_counter = static_cast<uint32_t>(counter);
  auto bridge_keysetId = keysetId;
  auto result = ::cashucrypto::bridge::createDeterministicOutputs(bridge_amounts, bridge_seed, bridge_counter, bridge_keysetId);
  return ([&]{ const auto& src0 = (result); std::vector<BlindedOutput> out0; out0.reserve(src0.size()); for (const auto& item0 : src0) { out0.push_back(fromBridgeBlindedOutput(item0)); } return out0; }());
}

std::vector<BlindedOutput> HybridCashuCrypto::createP2pkOutputs(const P2pkOptions& p2pk, const std::vector<uint64_t>& amounts, const std::string& keysetId) {
  auto bridge_p2pk = toBridgeP2pkOptions(p2pk);
  auto bridge_amounts = ([&]{ const auto& src0 = (amounts); std::vector<uint64_t> out0; out0.reserve(src0.size()); for (const auto& item0 : src0) { out0.push_back(item0); } return out0; }());
  auto bridge_keysetId = keysetId;
  auto result = ::cashucrypto::bridge::createP2pkOutputs(bridge_p2pk, bridge_amounts, bridge_keysetId);
  return ([&]{ const auto& src0 = (result); std::vector<BlindedOutput> out0; out0.reserve(src0.size()); for (const auto& item0 : src0) { out0.push_back(fromBridgeBlindedOutput(item0)); } return out0; }());
}

std::vector<BlindedOutput> HybridCashuCrypto::createRandomOutputs(const std::vector<uint64_t>& amounts, const std::string& keysetId) {
  auto bridge_amounts = ([&]{ const auto& src0 = (amounts); std::vector<uint64_t> out0; out0.reserve(src0.size()); for (const auto& item0 : src0) { out0.push_back(item0); } return out0; }());
  auto bridge_keysetId = keysetId;
  auto result = ::cashucrypto::bridge::createRandomOutputs(bridge_amounts, bridge_keysetId);
  return ([&]{ const auto& src0 = (result); std::vector<BlindedOutput> out0; out0.reserve(src0.size()); for (const auto& item0 : src0) { out0.push_back(fromBridgeBlindedOutput(item0)); } return out0; }());
}

std::vector<BlindedOutput> HybridCashuCrypto::createRestoreOutputs(const std::shared_ptr<ArrayBuffer>& seed, const std::string& keysetId, double startCounter, double endCounter) {
  auto bridge_seed = ([&]{ const auto& src0 = (seed); return std::vector<uint8_t>(src0->data(), src0->data() + src0->size()); }());
  auto bridge_keysetId = keysetId;
  auto bridge_startCounter = static_cast<uint32_t>(startCounter);
  auto bridge_endCounter = static_cast<uint32_t>(endCounter);
  auto result = ::cashucrypto::bridge::createRestoreOutputs(bridge_seed, bridge_keysetId, bridge_startCounter, bridge_endCounter);
  return ([&]{ const auto& src0 = (result); std::vector<BlindedOutput> out0; out0.reserve(src0.size()); for (const auto& item0 : src0) { out0.push_back(fromBridgeBlindedOutput(item0)); } return out0; }());
}

BlindedOutput HybridCashuCrypto::createSingleDeterministicOutput(uint64_t amount, const std::shared_ptr<ArrayBuffer>& seed, double counter, const std::string& keysetId) {
  auto bridge_amount = amount;
  auto bridge_seed = ([&]{ const auto& src0 = (seed); return std::vector<uint8_t>(src0->data(), src0->data() + src0->size()); }());
  auto bridge_counter = static_cast<uint32_t>(counter);
  auto bridge_keysetId = keysetId;
  auto result = ::cashucrypto::bridge::createSingleDeterministicOutput(bridge_amount, bridge_seed, bridge_counter, bridge_keysetId);
  return fromBridgeBlindedOutput(result);
}

BlindedOutput HybridCashuCrypto::createSingleP2pkOutput(const P2pkOptions& p2pk, uint64_t amount, const std::string& keysetId) {
  auto bridge_p2pk = toBridgeP2pkOptions(p2pk);
  auto bridge_amount = amount;
  auto bridge_keysetId = keysetId;
  auto result = ::cashucrypto::bridge::createSingleP2pkOutput(bridge_p2pk, bridge_amount, bridge_keysetId);
  return fromBridgeBlindedOutput(result);
}

BlindedOutput HybridCashuCrypto::createSingleRandomOutput(uint64_t amount, const std::string& keysetId) {
  auto bridge_amount = amount;
  auto bridge_keysetId = keysetId;
  auto result = ::cashucrypto::bridge::createSingleRandomOutput(bridge_amount, bridge_keysetId);
  return fromBridgeBlindedOutput(result);
}

std::shared_ptr<ArrayBuffer> HybridCashuCrypto::hashToCurve(const std::shared_ptr<ArrayBuffer>& message) {
  auto bridge_message = ([&]{ const auto& src0 = (message); return std::vector<uint8_t>(src0->data(), src0->data() + src0->size()); }());
  auto result = ::cashucrypto::bridge::hashToCurve(bridge_message);
  return ArrayBuffer::move(std::move(result));
}

std::string HybridCashuCrypto::keysetIdV1(const std::vector<KeyEntry>& keys) {
  auto bridge_keys = ([&]{ const auto& src0 = (keys); std::vector<::cashucrypto::bridge::KeyEntry> out0; out0.reserve(src0.size()); for (const auto& item0 : src0) { out0.push_back(toBridgeKeyEntry(item0)); } return out0; }());
  auto result = ::cashucrypto::bridge::keysetIdV1(bridge_keys);
  return std::move(result);
}

std::shared_ptr<ArrayBuffer> HybridCashuCrypto::sha256Digest(const std::shared_ptr<ArrayBuffer>& data) {
  auto bridge_data = ([&]{ const auto& src0 = (data); return std::vector<uint8_t>(src0->data(), src0->data() + src0->size()); }());
  auto result = ::cashucrypto::bridge::sha256Digest(bridge_data);
  return ArrayBuffer::move(std::move(result));
}

std::vector<uint64_t> HybridCashuCrypto::splitAmount(uint64_t amount, const std::vector<uint64_t>& denominations, const std::optional<std::vector<uint64_t>>& customSplit) {
  auto bridge_amount = amount;
  auto bridge_denominations = ([&]{ const auto& src0 = (denominations); std::vector<uint64_t> out0; out0.reserve(src0.size()); for (const auto& item0 : src0) { out0.push_back(item0); } return out0; }());
  auto bridge_customSplit = ([&]{ const auto& src0 = (customSplit); std::optional<std::vector<uint64_t>> out0; if (src0.has_value()) { out0 = ([&]{ const auto& src1 = ((*src0)); std::vector<uint64_t> out1; out1.reserve(src1.size()); for (const auto& item1 : src1) { out1.push_back(item1); } return out1; }()); } return out0; }());
  auto result = ::cashucrypto::bridge::splitAmount(bridge_amount, bridge_denominations, bridge_customSplit);
  return ([&]{ const auto& src0 = (result); std::vector<uint64_t> out0; out0.reserve(src0.size()); for (const auto& item0 : src0) { out0.push_back(item0); } return out0; }());
}

std::string HybridCashuCrypto::unblindSignature(const std::string& blindedSignature, const std::string& blindingFactor, const std::string& mintPubkey) {
  auto bridge_blindedSignature = blindedSignature;
  auto bridge_blindingFactor = blindingFactor;
  auto bridge_mintPubkey = mintPubkey;
  auto result = ::cashucrypto::bridge::unblindSignature(bridge_blindedSignature, bridge_blindingFactor, bridge_mintPubkey);
  return std::move(result);
}

bool HybridCashuCrypto::verifyProofDleq(const std::string& secret, const std::string& unblindedSignature, const DleqProof& dleq, const std::string& blindingFactor, const std::string& mintPubkey) {
  auto bridge_secret = secret;
  auto bridge_unblindedSignature = unblindedSignature;
  auto bridge_dleq = toBridgeDleqProof(dleq);
  auto bridge_blindingFactor = blindingFactor;
  auto bridge_mintPubkey = mintPubkey;
  auto result = ::cashucrypto::bridge::verifyProofDleq(bridge_secret, bridge_unblindedSignature, bridge_dleq, bridge_blindingFactor, bridge_mintPubkey);
  return result;
}

} // namespace margelo::nitro::cashucrypto
