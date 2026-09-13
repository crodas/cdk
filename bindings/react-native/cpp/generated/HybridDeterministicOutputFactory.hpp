// GENERATED FILE.
// DO NOT EDIT.
//
// Produced by uniffi-bindgen-nitro from the UniFFI metadata of `cashu_ffi`.
// Change the Rust `#[uniffi::export]` surface and regenerate instead.

#pragma once

#include "HybridDeterministicOutputFactorySpec.hpp"
#include "CashuCryptoBridge.hpp"

#include <memory>
#include <optional>
#include <string>
#include <vector>

namespace margelo::nitro::cashucrypto {

/// Wraps the Rust `DeterministicOutputFactory`; the handle dies with this object.
class HybridDeterministicOutputFactory final : public HybridDeterministicOutputFactorySpec {
public:
  explicit HybridDeterministicOutputFactory(std::shared_ptr<::cashucrypto::bridge::DeterministicOutputFactory> inner)
    : HybridObject(TAG), inner_(std::move(inner)) {}

  std::string keysetId() override;
  std::vector<BlindedOutput> outputs(const std::vector<uint64_t>& amounts, double counter) override;
  std::vector<BlindedOutput> restoreBatch(double startCounter, double endCounter) override;
  BlindedOutput singleOutput(uint64_t amount, double counter) override;

private:
  std::shared_ptr<::cashucrypto::bridge::DeterministicOutputFactory> inner_;
};

} // namespace margelo::nitro::cashucrypto
