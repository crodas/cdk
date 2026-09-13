// GENERATED FILE.
// DO NOT EDIT.
//
// Produced by uniffi-bindgen-nitro from the UniFFI metadata of `cashu_ffi`.
// Change the Rust `#[uniffi::export]` surface and regenerate instead.

#include "HybridDeterministicOutputFactory.hpp"

#include <NitroModules/ArrayBuffer.hpp>
#include <NitroModules/Promise.hpp>

#include <utility>

namespace margelo::nitro::cashucrypto {

namespace {


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

} // namespace

std::string HybridDeterministicOutputFactory::keysetId() {
  auto result = inner_->keysetId();
  return std::move(result);
}

std::vector<BlindedOutput> HybridDeterministicOutputFactory::outputs(const std::vector<uint64_t>& amounts, double counter) {
  auto bridge_amounts = ([&]{ const auto& src0 = (amounts); std::vector<uint64_t> out0; out0.reserve(src0.size()); for (const auto& item0 : src0) { out0.push_back(item0); } return out0; }());
  auto bridge_counter = static_cast<uint32_t>(counter);
  auto result = inner_->outputs(bridge_amounts, bridge_counter);
  return ([&]{ const auto& src0 = (result); std::vector<BlindedOutput> out0; out0.reserve(src0.size()); for (const auto& item0 : src0) { out0.push_back(fromBridgeBlindedOutput(item0)); } return out0; }());
}

std::vector<BlindedOutput> HybridDeterministicOutputFactory::restoreBatch(double startCounter, double endCounter) {
  auto bridge_startCounter = static_cast<uint32_t>(startCounter);
  auto bridge_endCounter = static_cast<uint32_t>(endCounter);
  auto result = inner_->restoreBatch(bridge_startCounter, bridge_endCounter);
  return ([&]{ const auto& src0 = (result); std::vector<BlindedOutput> out0; out0.reserve(src0.size()); for (const auto& item0 : src0) { out0.push_back(fromBridgeBlindedOutput(item0)); } return out0; }());
}

BlindedOutput HybridDeterministicOutputFactory::singleOutput(uint64_t amount, double counter) {
  auto bridge_amount = amount;
  auto bridge_counter = static_cast<uint32_t>(counter);
  auto result = inner_->singleOutput(bridge_amount, bridge_counter);
  return fromBridgeBlindedOutput(result);
}

} // namespace margelo::nitro::cashucrypto
