// GENERATED FILE.
// DO NOT EDIT.
//
// Produced by uniffi-bindgen-nitro from the UniFFI metadata of `cashu_ffi`.
// Change the Rust `#[uniffi::export]` surface and regenerate instead.

#include "CashuCryptoBridge.hpp"
#include "CashuCryptoFfi.hpp"

#include <cstdio>
#include <cstring>

namespace cashucrypto::bridge {
namespace {

using ffi::RustBuffer;
using ffi::RustCallStatus;

/// Big-endian reader over the UniFFI buffer format.
class BufferReader final {
public:
  BufferReader(const uint8_t* data, size_t size) : data_(data), size_(size) {}

  int8_t readI8() { return static_cast<int8_t>(readByte()); }
  uint8_t readU8() { return readByte(); }
  bool readBool() { return readByte() != 0; }
  int16_t readI16() { return static_cast<int16_t>(readU16()); }
  uint16_t readU16() { return static_cast<uint16_t>(readUInt(2)); }
  int32_t readI32() { return static_cast<int32_t>(readU32()); }
  uint32_t readU32() { return static_cast<uint32_t>(readUInt(4)); }
  int64_t readI64() { return static_cast<int64_t>(readU64()); }
  uint64_t readU64() { return readUInt(8); }

  float readF32() {
    uint32_t bits = readU32();
    float value;
    std::memcpy(&value, &bits, sizeof(value));
    return value;
  }

  double readF64() {
    uint64_t bits = readU64();
    double value;
    std::memcpy(&value, &bits, sizeof(value));
    return value;
  }

  std::vector<uint8_t> readBytes() {
    size_t length = readLength();
    require(length);
    std::vector<uint8_t> value(data_ + position_, data_ + position_ + length);
    position_ += length;
    return value;
  }

  std::string readString() {
    size_t length = readLength();
    require(length);
    std::string value(reinterpret_cast<const char*>(data_ + position_), length);
    position_ += length;
    return value;
  }

  /// Number of items or bytes that follow, as UniFFI writes it: an i32.
  size_t readLength() {
    int32_t length = readI32();
    if (length < 0) {
      throw std::runtime_error("uniffi buffer declares a negative length");
    }
    return static_cast<size_t>(length);
  }

  void finish() const {
    if (position_ != size_) {
      throw std::runtime_error("uniffi buffer has trailing bytes");
    }
  }

private:
  uint8_t readByte() {
    require(1);
    return data_[position_++];
  }

  uint64_t readUInt(size_t width) {
    require(width);
    uint64_t value = 0;
    for (size_t i = 0; i < width; i++) {
      value = (value << 8) | data_[position_ + i];
    }
    position_ += width;
    return value;
  }

  void require(size_t count) const {
    if (position_ + count > size_) {
      throw std::runtime_error("uniffi buffer ended early");
    }
  }

  const uint8_t* data_;
  size_t size_;
  size_t position_ = 0;
};

/// Big-endian writer producing the UniFFI buffer format.
class BufferWriter final {
public:
  void writeI8(int8_t value) { bytes_.push_back(static_cast<uint8_t>(value)); }
  void writeU8(uint8_t value) { bytes_.push_back(value); }
  void writeBool(bool value) { writeI8(value ? 1 : 0); }
  void writeI16(int16_t value) { writeUInt(static_cast<uint16_t>(value), 2); }
  void writeU16(uint16_t value) { writeUInt(value, 2); }
  void writeI32(int32_t value) { writeUInt(static_cast<uint32_t>(value), 4); }
  void writeU32(uint32_t value) { writeUInt(value, 4); }
  void writeI64(int64_t value) { writeUInt(static_cast<uint64_t>(value), 8); }
  void writeU64(uint64_t value) { writeUInt(value, 8); }

  void writeF32(float value) {
    uint32_t bits;
    std::memcpy(&bits, &value, sizeof(bits));
    writeU32(bits);
  }

  void writeF64(double value) {
    uint64_t bits;
    std::memcpy(&bits, &value, sizeof(bits));
    writeU64(bits);
  }

  void writeBytes(const std::vector<uint8_t>& value) {
    writeLength(value.size());
    bytes_.insert(bytes_.end(), value.begin(), value.end());
  }

  void writeString(const std::string& value) {
    writeLength(value.size());
    bytes_.insert(bytes_.end(), value.begin(), value.end());
  }

  void writeLength(size_t count) {
    if (count > static_cast<size_t>(INT32_MAX)) {
      throw std::runtime_error("value is too large for the uniffi buffer format");
    }
    writeI32(static_cast<int32_t>(count));
  }

  const std::vector<uint8_t>& bytes() const { return bytes_; }

private:
  void writeUInt(uint64_t value, size_t width) {
    for (size_t i = width; i > 0; i--) {
      bytes_.push_back(static_cast<uint8_t>((value >> ((i - 1) * 8)) & 0xff));
    }
  }

  std::vector<uint8_t> bytes_;
};

RustBuffer emptyBuffer() {
  RustBuffer buffer;
  buffer.capacity = 0;
  buffer.len = 0;
  buffer.data = nullptr;
  return buffer;
}

void freeBuffer(RustBuffer buffer) {
  if (buffer.data == nullptr && buffer.capacity == 0) {
    return;
  }
  RustCallStatus status;
  status.code = 0;
  status.errorBuf = emptyBuffer();
  ffi::ffi_cashu_ffi_rustbuffer_free(buffer, &status);
}

/// Copy a Rust-owned buffer into C++ memory and hand the allocation back.
///
/// The copy is unavoidable: the bytes belong to Rust's allocator, so they
/// cannot outlive the call without being duplicated.
std::vector<uint8_t> consumeBuffer(RustBuffer buffer) {
  std::vector<uint8_t> bytes;
  if (buffer.data != nullptr && buffer.len > 0) {
    bytes.assign(buffer.data, buffer.data + buffer.len);
  }
  freeBuffer(buffer);
  return bytes;
}

std::string consumeBufferAsString(RustBuffer buffer) {
  std::string text;
  if (buffer.data != nullptr && buffer.len > 0) {
    text.assign(reinterpret_cast<const char*>(buffer.data), buffer.len);
  }
  freeBuffer(buffer);
  return text;
}

/// Hand foreign-owned bytes to Rust as a `RustBuffer` it takes ownership of.
RustBuffer bytesToBuffer(const std::vector<uint8_t>& bytes) {
  ffi::ForeignBytes borrowed;
  borrowed.len = static_cast<int32_t>(bytes.size());
  borrowed.data = bytes.data();
  RustCallStatus status;
  status.code = 0;
  status.errorBuf = emptyBuffer();
  RustBuffer buffer = ffi::ffi_cashu_ffi_rustbuffer_from_bytes(borrowed, &status);
  if (status.code != ffi::RUST_CALL_SUCCESS) {
    freeBuffer(status.errorBuf);
    throw std::runtime_error("uniffi could not allocate a buffer");
  }
  return buffer;
}

RustBuffer stringToBuffer(const std::string& text) {
  std::vector<uint8_t> bytes(text.begin(), text.end());
  return bytesToBuffer(bytes);
}

std::string jsonEscape(const std::string& text) {
  std::string escaped;
  escaped.reserve(text.size() + 2);
  for (unsigned char character : text) {
    switch (character) {
      case '"': escaped += "\\\""; break;
      case '\\': escaped += "\\\\"; break;
      case '\n': escaped += "\\n"; break;
      case '\r': escaped += "\\r"; break;
      case '\t': escaped += "\\t"; break;
      default:
        if (character < 0x20) {
          char slot[7];
          std::snprintf(slot, sizeof(slot), "\\u%04x", character);
          escaped += slot;
        } else {
          escaped += static_cast<char>(character);
        }
    }
  }
  return escaped;
}

std::string toHexString(const std::vector<uint8_t>& bytes) {
  static const char* digits = "0123456789abcdef";
  std::string hex;
  hex.reserve(bytes.size() * 2);
  for (uint8_t byte : bytes) {
    hex.push_back(digits[byte >> 4]);
    hex.push_back(digits[byte & 0x0f]);
  }
  return hex;
}

/// Turn a panic or a cancellation into a C++ exception.
///
/// An expected `Result::Err` is left alone: only the caller knows which error
/// enum to decode it as.
void checkUnexpected(RustCallStatus& status) {
  if (status.code == ffi::RUST_CALL_UNEXPECTED_ERROR) {
    std::string message = consumeBufferAsString(status.errorBuf);
    status.errorBuf = emptyBuffer();
    throw std::runtime_error("rust panicked: " + message);
  }
  if (status.code == ffi::RUST_CALL_CANCELLED) {
    throw std::runtime_error("rust call was cancelled");
  }
}

// Forward declarations so serializers can reference each other freely.
BlindPair readBlindPair(BufferReader& reader);
void writeBlindPair(BufferWriter& writer, const BlindPair& value);
RustBuffer lowerBlindPair(const BlindPair& value);
BlindPair liftBlindPair(RustBuffer buffer);
BlindedOutput readBlindedOutput(BufferReader& reader);
void writeBlindedOutput(BufferWriter& writer, const BlindedOutput& value);
RustBuffer lowerBlindedOutput(const BlindedOutput& value);
BlindedOutput liftBlindedOutput(RustBuffer buffer);
bool readBool(BufferReader& reader);
void writeBool(BufferWriter& writer, const bool& value);
std::vector<uint8_t> readBytes(BufferReader& reader);
void writeBytes(BufferWriter& writer, const std::vector<uint8_t>& value);
RustBuffer lowerBytes(const std::vector<uint8_t>& value);
std::vector<uint8_t> liftBytes(RustBuffer buffer);
std::shared_ptr<DeterministicOutputFactory> readDeterministicOutputFactory(BufferReader& reader);
void writeDeterministicOutputFactory(BufferWriter& writer, const std::shared_ptr<DeterministicOutputFactory>& value);
DleqProof readDleqProof(BufferReader& reader);
void writeDleqProof(BufferWriter& writer, const DleqProof& value);
RustBuffer lowerDleqProof(const DleqProof& value);
DleqProof liftDleqProof(RustBuffer buffer);
KeyEntry readKeyEntry(BufferReader& reader);
void writeKeyEntry(BufferWriter& writer, const KeyEntry& value);
RustBuffer lowerKeyEntry(const KeyEntry& value);
KeyEntry liftKeyEntry(RustBuffer buffer);
std::optional<std::vector<uint8_t>> readOptBytes(BufferReader& reader);
void writeOptBytes(BufferWriter& writer, const std::optional<std::vector<uint8_t>>& value);
RustBuffer lowerOptBytes(const std::optional<std::vector<uint8_t>>& value);
std::optional<std::vector<uint8_t>> liftOptBytes(RustBuffer buffer);
std::optional<std::vector<std::string>> readOptSeqString(BufferReader& reader);
void writeOptSeqString(BufferWriter& writer, const std::optional<std::vector<std::string>>& value);
RustBuffer lowerOptSeqString(const std::optional<std::vector<std::string>>& value);
std::optional<std::vector<std::string>> liftOptSeqString(RustBuffer buffer);
std::optional<std::vector<uint64_t>> readOptSeqU64(BufferReader& reader);
void writeOptSeqU64(BufferWriter& writer, const std::optional<std::vector<uint64_t>>& value);
RustBuffer lowerOptSeqU64(const std::optional<std::vector<uint64_t>>& value);
std::optional<std::vector<uint64_t>> liftOptSeqU64(RustBuffer buffer);
std::optional<uint32_t> readOptU32(BufferReader& reader);
void writeOptU32(BufferWriter& writer, const std::optional<uint32_t>& value);
RustBuffer lowerOptU32(const std::optional<uint32_t>& value);
std::optional<uint32_t> liftOptU32(RustBuffer buffer);
std::optional<uint64_t> readOptU64(BufferReader& reader);
void writeOptU64(BufferWriter& writer, const std::optional<uint64_t>& value);
RustBuffer lowerOptU64(const std::optional<uint64_t>& value);
std::optional<uint64_t> liftOptU64(RustBuffer buffer);
P2pkOptions readP2pkOptions(BufferReader& reader);
void writeP2pkOptions(BufferWriter& writer, const P2pkOptions& value);
RustBuffer lowerP2pkOptions(const P2pkOptions& value);
P2pkOptions liftP2pkOptions(RustBuffer buffer);
std::vector<BlindPair> readSeqBlindPair(BufferReader& reader);
void writeSeqBlindPair(BufferWriter& writer, const std::vector<BlindPair>& value);
RustBuffer lowerSeqBlindPair(const std::vector<BlindPair>& value);
std::vector<BlindPair> liftSeqBlindPair(RustBuffer buffer);
std::vector<BlindedOutput> readSeqBlindedOutput(BufferReader& reader);
void writeSeqBlindedOutput(BufferWriter& writer, const std::vector<BlindedOutput>& value);
RustBuffer lowerSeqBlindedOutput(const std::vector<BlindedOutput>& value);
std::vector<BlindedOutput> liftSeqBlindedOutput(RustBuffer buffer);
std::vector<std::vector<uint8_t>> readSeqBytes(BufferReader& reader);
void writeSeqBytes(BufferWriter& writer, const std::vector<std::vector<uint8_t>>& value);
RustBuffer lowerSeqBytes(const std::vector<std::vector<uint8_t>>& value);
std::vector<std::vector<uint8_t>> liftSeqBytes(RustBuffer buffer);
std::vector<KeyEntry> readSeqKeyEntry(BufferReader& reader);
void writeSeqKeyEntry(BufferWriter& writer, const std::vector<KeyEntry>& value);
RustBuffer lowerSeqKeyEntry(const std::vector<KeyEntry>& value);
std::vector<KeyEntry> liftSeqKeyEntry(RustBuffer buffer);
std::vector<std::string> readSeqString(BufferReader& reader);
void writeSeqString(BufferWriter& writer, const std::vector<std::string>& value);
RustBuffer lowerSeqString(const std::vector<std::string>& value);
std::vector<std::string> liftSeqString(RustBuffer buffer);
std::vector<uint64_t> readSeqU64(BufferReader& reader);
void writeSeqU64(BufferWriter& writer, const std::vector<uint64_t>& value);
RustBuffer lowerSeqU64(const std::vector<uint64_t>& value);
std::vector<uint64_t> liftSeqU64(RustBuffer buffer);
SigFlag readSigFlag(BufferReader& reader);
void writeSigFlag(BufferWriter& writer, const SigFlag& value);
RustBuffer lowerSigFlag(const SigFlag& value);
SigFlag liftSigFlag(RustBuffer buffer);
std::string readString(BufferReader& reader);
void writeString(BufferWriter& writer, const std::string& value);
uint32_t readU32(BufferReader& reader);
void writeU32(BufferWriter& writer, const uint32_t& value);
uint64_t readU64(BufferReader& reader);
void writeU64(BufferWriter& writer, const uint64_t& value);

BlindPair readBlindPair(BufferReader& reader) {
  BlindPair value;
  value.blindedSecret = readString(reader);
  value.blindingFactor = readString(reader);
  return value;
}

void writeBlindPair(BufferWriter& writer, const BlindPair& value) {
  writeString(writer, value.blindedSecret);
  writeString(writer, value.blindingFactor);

}

RustBuffer lowerBlindPair(const BlindPair& value) {
  BufferWriter writer;
  writeBlindPair(writer, value);
  return bytesToBuffer(writer.bytes());
}

BlindPair liftBlindPair(RustBuffer buffer) {
  std::vector<uint8_t> bytes = consumeBuffer(buffer);
  BufferReader reader(bytes.data(), bytes.size());
  BlindPair value = readBlindPair(reader);
  reader.finish();
  return value;
}

BlindedOutput readBlindedOutput(BufferReader& reader) {
  BlindedOutput value;
  value.amount = readU64(reader);
  value.keysetId = readString(reader);
  value.blindedSecret = readString(reader);
  value.blindingFactor = readString(reader);
  value.secret = readString(reader);
  value.derivationIndex = readOptU32(reader);
  return value;
}

void writeBlindedOutput(BufferWriter& writer, const BlindedOutput& value) {
  writeU64(writer, value.amount);
  writeString(writer, value.keysetId);
  writeString(writer, value.blindedSecret);
  writeString(writer, value.blindingFactor);
  writeString(writer, value.secret);
  writeOptU32(writer, value.derivationIndex);

}

RustBuffer lowerBlindedOutput(const BlindedOutput& value) {
  BufferWriter writer;
  writeBlindedOutput(writer, value);
  return bytesToBuffer(writer.bytes());
}

BlindedOutput liftBlindedOutput(RustBuffer buffer) {
  std::vector<uint8_t> bytes = consumeBuffer(buffer);
  BufferReader reader(bytes.data(), bytes.size());
  BlindedOutput value = readBlindedOutput(reader);
  reader.finish();
  return value;
}

bool readBool(BufferReader& reader) {
  return reader.readBool();
}

void writeBool(BufferWriter& writer, const bool& value) {
  writer.writeBool(value);
}

std::vector<uint8_t> readBytes(BufferReader& reader) {
  return reader.readBytes();
}

void writeBytes(BufferWriter& writer, const std::vector<uint8_t>& value) {
  writer.writeBytes(value);
}

RustBuffer lowerBytes(const std::vector<uint8_t>& value) {
  BufferWriter writer;
  writeBytes(writer, value);
  return bytesToBuffer(writer.bytes());
}

std::vector<uint8_t> liftBytes(RustBuffer buffer) {
  std::vector<uint8_t> bytes = consumeBuffer(buffer);
  BufferReader reader(bytes.data(), bytes.size());
  std::vector<uint8_t> value = readBytes(reader);
  reader.finish();
  return value;
}

std::shared_ptr<DeterministicOutputFactory> readDeterministicOutputFactory(BufferReader& reader) {
  return std::make_shared<DeterministicOutputFactory>(reader.readU64());
}

void writeDeterministicOutputFactory(BufferWriter& writer, const std::shared_ptr<DeterministicOutputFactory>& value) {
  writer.writeU64(value->cloneHandle());
}

DleqProof readDleqProof(BufferReader& reader) {
  DleqProof value;
  value.e = readString(reader);
  value.s = readString(reader);
  return value;
}

void writeDleqProof(BufferWriter& writer, const DleqProof& value) {
  writeString(writer, value.e);
  writeString(writer, value.s);

}

RustBuffer lowerDleqProof(const DleqProof& value) {
  BufferWriter writer;
  writeDleqProof(writer, value);
  return bytesToBuffer(writer.bytes());
}

DleqProof liftDleqProof(RustBuffer buffer) {
  std::vector<uint8_t> bytes = consumeBuffer(buffer);
  BufferReader reader(bytes.data(), bytes.size());
  DleqProof value = readDleqProof(reader);
  reader.finish();
  return value;
}

KeyEntry readKeyEntry(BufferReader& reader) {
  KeyEntry value;
  value.amount = readU64(reader);
  value.pubkey = readString(reader);
  return value;
}

void writeKeyEntry(BufferWriter& writer, const KeyEntry& value) {
  writeU64(writer, value.amount);
  writeString(writer, value.pubkey);

}

RustBuffer lowerKeyEntry(const KeyEntry& value) {
  BufferWriter writer;
  writeKeyEntry(writer, value);
  return bytesToBuffer(writer.bytes());
}

KeyEntry liftKeyEntry(RustBuffer buffer) {
  std::vector<uint8_t> bytes = consumeBuffer(buffer);
  BufferReader reader(bytes.data(), bytes.size());
  KeyEntry value = readKeyEntry(reader);
  reader.finish();
  return value;
}

std::optional<std::vector<uint8_t>> readOptBytes(BufferReader& reader) {
  if (reader.readI8() == 0) {
    return std::nullopt;
  }
  return readBytes(reader);
}

void writeOptBytes(BufferWriter& writer, const std::optional<std::vector<uint8_t>>& value) {
  if (!value.has_value()) {
    writer.writeI8(0);
    return;
  }
  writer.writeI8(1);
  writeBytes(writer, *value);
}

RustBuffer lowerOptBytes(const std::optional<std::vector<uint8_t>>& value) {
  BufferWriter writer;
  writeOptBytes(writer, value);
  return bytesToBuffer(writer.bytes());
}

std::optional<std::vector<uint8_t>> liftOptBytes(RustBuffer buffer) {
  std::vector<uint8_t> bytes = consumeBuffer(buffer);
  BufferReader reader(bytes.data(), bytes.size());
  std::optional<std::vector<uint8_t>> value = readOptBytes(reader);
  reader.finish();
  return value;
}

std::optional<std::vector<std::string>> readOptSeqString(BufferReader& reader) {
  if (reader.readI8() == 0) {
    return std::nullopt;
  }
  return readSeqString(reader);
}

void writeOptSeqString(BufferWriter& writer, const std::optional<std::vector<std::string>>& value) {
  if (!value.has_value()) {
    writer.writeI8(0);
    return;
  }
  writer.writeI8(1);
  writeSeqString(writer, *value);
}

RustBuffer lowerOptSeqString(const std::optional<std::vector<std::string>>& value) {
  BufferWriter writer;
  writeOptSeqString(writer, value);
  return bytesToBuffer(writer.bytes());
}

std::optional<std::vector<std::string>> liftOptSeqString(RustBuffer buffer) {
  std::vector<uint8_t> bytes = consumeBuffer(buffer);
  BufferReader reader(bytes.data(), bytes.size());
  std::optional<std::vector<std::string>> value = readOptSeqString(reader);
  reader.finish();
  return value;
}

std::optional<std::vector<uint64_t>> readOptSeqU64(BufferReader& reader) {
  if (reader.readI8() == 0) {
    return std::nullopt;
  }
  return readSeqU64(reader);
}

void writeOptSeqU64(BufferWriter& writer, const std::optional<std::vector<uint64_t>>& value) {
  if (!value.has_value()) {
    writer.writeI8(0);
    return;
  }
  writer.writeI8(1);
  writeSeqU64(writer, *value);
}

RustBuffer lowerOptSeqU64(const std::optional<std::vector<uint64_t>>& value) {
  BufferWriter writer;
  writeOptSeqU64(writer, value);
  return bytesToBuffer(writer.bytes());
}

std::optional<std::vector<uint64_t>> liftOptSeqU64(RustBuffer buffer) {
  std::vector<uint8_t> bytes = consumeBuffer(buffer);
  BufferReader reader(bytes.data(), bytes.size());
  std::optional<std::vector<uint64_t>> value = readOptSeqU64(reader);
  reader.finish();
  return value;
}

std::optional<uint32_t> readOptU32(BufferReader& reader) {
  if (reader.readI8() == 0) {
    return std::nullopt;
  }
  return readU32(reader);
}

void writeOptU32(BufferWriter& writer, const std::optional<uint32_t>& value) {
  if (!value.has_value()) {
    writer.writeI8(0);
    return;
  }
  writer.writeI8(1);
  writeU32(writer, *value);
}

RustBuffer lowerOptU32(const std::optional<uint32_t>& value) {
  BufferWriter writer;
  writeOptU32(writer, value);
  return bytesToBuffer(writer.bytes());
}

std::optional<uint32_t> liftOptU32(RustBuffer buffer) {
  std::vector<uint8_t> bytes = consumeBuffer(buffer);
  BufferReader reader(bytes.data(), bytes.size());
  std::optional<uint32_t> value = readOptU32(reader);
  reader.finish();
  return value;
}

std::optional<uint64_t> readOptU64(BufferReader& reader) {
  if (reader.readI8() == 0) {
    return std::nullopt;
  }
  return readU64(reader);
}

void writeOptU64(BufferWriter& writer, const std::optional<uint64_t>& value) {
  if (!value.has_value()) {
    writer.writeI8(0);
    return;
  }
  writer.writeI8(1);
  writeU64(writer, *value);
}

RustBuffer lowerOptU64(const std::optional<uint64_t>& value) {
  BufferWriter writer;
  writeOptU64(writer, value);
  return bytesToBuffer(writer.bytes());
}

std::optional<uint64_t> liftOptU64(RustBuffer buffer) {
  std::vector<uint8_t> bytes = consumeBuffer(buffer);
  BufferReader reader(bytes.data(), bytes.size());
  std::optional<uint64_t> value = readOptU64(reader);
  reader.finish();
  return value;
}

P2pkOptions readP2pkOptions(BufferReader& reader) {
  P2pkOptions value;
  value.pubkey = readString(reader);
  value.additionalPubkeys = readOptSeqString(reader);
  value.numSigs = readOptU64(reader);
  value.locktime = readOptU64(reader);
  value.refundPubkeys = readOptSeqString(reader);
  value.numSigsRefund = readOptU64(reader);
  value.sigFlag = readSigFlag(reader);
  return value;
}

void writeP2pkOptions(BufferWriter& writer, const P2pkOptions& value) {
  writeString(writer, value.pubkey);
  writeOptSeqString(writer, value.additionalPubkeys);
  writeOptU64(writer, value.numSigs);
  writeOptU64(writer, value.locktime);
  writeOptSeqString(writer, value.refundPubkeys);
  writeOptU64(writer, value.numSigsRefund);
  writeSigFlag(writer, value.sigFlag);

}

RustBuffer lowerP2pkOptions(const P2pkOptions& value) {
  BufferWriter writer;
  writeP2pkOptions(writer, value);
  return bytesToBuffer(writer.bytes());
}

P2pkOptions liftP2pkOptions(RustBuffer buffer) {
  std::vector<uint8_t> bytes = consumeBuffer(buffer);
  BufferReader reader(bytes.data(), bytes.size());
  P2pkOptions value = readP2pkOptions(reader);
  reader.finish();
  return value;
}

std::vector<BlindPair> readSeqBlindPair(BufferReader& reader) {
  size_t count = reader.readLength();
  std::vector<BlindPair> items;
  items.reserve(count);
  for (size_t i = 0; i < count; i++) {
    items.push_back(readBlindPair(reader));
  }
  return items;
}

void writeSeqBlindPair(BufferWriter& writer, const std::vector<BlindPair>& value) {
  writer.writeLength(value.size());
  for (const auto& item : value) {
    writeBlindPair(writer, item);
  }
}

RustBuffer lowerSeqBlindPair(const std::vector<BlindPair>& value) {
  BufferWriter writer;
  writeSeqBlindPair(writer, value);
  return bytesToBuffer(writer.bytes());
}

std::vector<BlindPair> liftSeqBlindPair(RustBuffer buffer) {
  std::vector<uint8_t> bytes = consumeBuffer(buffer);
  BufferReader reader(bytes.data(), bytes.size());
  std::vector<BlindPair> value = readSeqBlindPair(reader);
  reader.finish();
  return value;
}

std::vector<BlindedOutput> readSeqBlindedOutput(BufferReader& reader) {
  size_t count = reader.readLength();
  std::vector<BlindedOutput> items;
  items.reserve(count);
  for (size_t i = 0; i < count; i++) {
    items.push_back(readBlindedOutput(reader));
  }
  return items;
}

void writeSeqBlindedOutput(BufferWriter& writer, const std::vector<BlindedOutput>& value) {
  writer.writeLength(value.size());
  for (const auto& item : value) {
    writeBlindedOutput(writer, item);
  }
}

RustBuffer lowerSeqBlindedOutput(const std::vector<BlindedOutput>& value) {
  BufferWriter writer;
  writeSeqBlindedOutput(writer, value);
  return bytesToBuffer(writer.bytes());
}

std::vector<BlindedOutput> liftSeqBlindedOutput(RustBuffer buffer) {
  std::vector<uint8_t> bytes = consumeBuffer(buffer);
  BufferReader reader(bytes.data(), bytes.size());
  std::vector<BlindedOutput> value = readSeqBlindedOutput(reader);
  reader.finish();
  return value;
}

std::vector<std::vector<uint8_t>> readSeqBytes(BufferReader& reader) {
  size_t count = reader.readLength();
  std::vector<std::vector<uint8_t>> items;
  items.reserve(count);
  for (size_t i = 0; i < count; i++) {
    items.push_back(readBytes(reader));
  }
  return items;
}

void writeSeqBytes(BufferWriter& writer, const std::vector<std::vector<uint8_t>>& value) {
  writer.writeLength(value.size());
  for (const auto& item : value) {
    writeBytes(writer, item);
  }
}

RustBuffer lowerSeqBytes(const std::vector<std::vector<uint8_t>>& value) {
  BufferWriter writer;
  writeSeqBytes(writer, value);
  return bytesToBuffer(writer.bytes());
}

std::vector<std::vector<uint8_t>> liftSeqBytes(RustBuffer buffer) {
  std::vector<uint8_t> bytes = consumeBuffer(buffer);
  BufferReader reader(bytes.data(), bytes.size());
  std::vector<std::vector<uint8_t>> value = readSeqBytes(reader);
  reader.finish();
  return value;
}

std::vector<KeyEntry> readSeqKeyEntry(BufferReader& reader) {
  size_t count = reader.readLength();
  std::vector<KeyEntry> items;
  items.reserve(count);
  for (size_t i = 0; i < count; i++) {
    items.push_back(readKeyEntry(reader));
  }
  return items;
}

void writeSeqKeyEntry(BufferWriter& writer, const std::vector<KeyEntry>& value) {
  writer.writeLength(value.size());
  for (const auto& item : value) {
    writeKeyEntry(writer, item);
  }
}

RustBuffer lowerSeqKeyEntry(const std::vector<KeyEntry>& value) {
  BufferWriter writer;
  writeSeqKeyEntry(writer, value);
  return bytesToBuffer(writer.bytes());
}

std::vector<KeyEntry> liftSeqKeyEntry(RustBuffer buffer) {
  std::vector<uint8_t> bytes = consumeBuffer(buffer);
  BufferReader reader(bytes.data(), bytes.size());
  std::vector<KeyEntry> value = readSeqKeyEntry(reader);
  reader.finish();
  return value;
}

std::vector<std::string> readSeqString(BufferReader& reader) {
  size_t count = reader.readLength();
  std::vector<std::string> items;
  items.reserve(count);
  for (size_t i = 0; i < count; i++) {
    items.push_back(readString(reader));
  }
  return items;
}

void writeSeqString(BufferWriter& writer, const std::vector<std::string>& value) {
  writer.writeLength(value.size());
  for (const auto& item : value) {
    writeString(writer, item);
  }
}

RustBuffer lowerSeqString(const std::vector<std::string>& value) {
  BufferWriter writer;
  writeSeqString(writer, value);
  return bytesToBuffer(writer.bytes());
}

std::vector<std::string> liftSeqString(RustBuffer buffer) {
  std::vector<uint8_t> bytes = consumeBuffer(buffer);
  BufferReader reader(bytes.data(), bytes.size());
  std::vector<std::string> value = readSeqString(reader);
  reader.finish();
  return value;
}

std::vector<uint64_t> readSeqU64(BufferReader& reader) {
  size_t count = reader.readLength();
  std::vector<uint64_t> items;
  items.reserve(count);
  for (size_t i = 0; i < count; i++) {
    items.push_back(readU64(reader));
  }
  return items;
}

void writeSeqU64(BufferWriter& writer, const std::vector<uint64_t>& value) {
  writer.writeLength(value.size());
  for (const auto& item : value) {
    writeU64(writer, item);
  }
}

RustBuffer lowerSeqU64(const std::vector<uint64_t>& value) {
  BufferWriter writer;
  writeSeqU64(writer, value);
  return bytesToBuffer(writer.bytes());
}

std::vector<uint64_t> liftSeqU64(RustBuffer buffer) {
  std::vector<uint8_t> bytes = consumeBuffer(buffer);
  BufferReader reader(bytes.data(), bytes.size());
  std::vector<uint64_t> value = readSeqU64(reader);
  reader.finish();
  return value;
}

SigFlag readSigFlag(BufferReader& reader) {
  int32_t tag = reader.readI32();
  switch (tag) {
    case 1: return SigFlag::SigInputs;
    case 2: return SigFlag::SigAll;
    default:
      throw std::runtime_error("unknown SigFlag variant tag");
  }
}

void writeSigFlag(BufferWriter& writer, const SigFlag& value) {
  writer.writeI32(static_cast<int32_t>(value));
}

RustBuffer lowerSigFlag(const SigFlag& value) {
  BufferWriter writer;
  writeSigFlag(writer, value);
  return bytesToBuffer(writer.bytes());
}

SigFlag liftSigFlag(RustBuffer buffer) {
  std::vector<uint8_t> bytes = consumeBuffer(buffer);
  BufferReader reader(bytes.data(), bytes.size());
  SigFlag value = readSigFlag(reader);
  reader.finish();
  return value;
}

std::string readString(BufferReader& reader) {
  return reader.readString();
}

void writeString(BufferWriter& writer, const std::string& value) {
  writer.writeString(value);
}

uint32_t readU32(BufferReader& reader) {
  return reader.readU32();
}

void writeU32(BufferWriter& writer, const uint32_t& value) {
  writer.writeU32(value);
}

uint64_t readU64(BufferReader& reader) {
  return reader.readU64();
}

void writeU64(BufferWriter& writer, const uint64_t& value) {
  writer.writeU64(value);
}


[[noreturn]] void throwCashuFfiError(RustBuffer buffer) {
  std::vector<uint8_t> bytes = consumeBuffer(buffer);
  BufferReader reader(bytes.data(), bytes.size());
  int32_t tag = reader.readI32();
  std::string payload;
  switch (tag) {
    case 1: {
      std::string field = readString(reader);
      std::string reason = readString(reader);
      std::string fields = "{";
      fields += "\"field\":" + ("\"" + jsonEscape(field) + "\"");
      fields += ",\"reason\":" + ("\"" + jsonEscape(reason) + "\"");
      fields += "}";
      std::string message = std::string("InvalidHex") + "(" + "field=" + field + ", reason=" + reason + ")";
      payload = std::string("{\"type\":\"CashuFfiError\",\"kind\":\"InvalidHex\",\"message\":\"") + jsonEscape(message) + "\",\"fields\":" + fields + "}";
      break;
    }
    case 2: {
      std::string field = readString(reader);
      std::string reason = readString(reader);
      std::string fields = "{";
      fields += "\"field\":" + ("\"" + jsonEscape(field) + "\"");
      fields += ",\"reason\":" + ("\"" + jsonEscape(reason) + "\"");
      fields += "}";
      std::string message = std::string("InvalidPublicKey") + "(" + "field=" + field + ", reason=" + reason + ")";
      payload = std::string("{\"type\":\"CashuFfiError\",\"kind\":\"InvalidPublicKey\",\"message\":\"") + jsonEscape(message) + "\",\"fields\":" + fields + "}";
      break;
    }
    case 3: {
      std::string field = readString(reader);
      std::string reason = readString(reader);
      std::string fields = "{";
      fields += "\"field\":" + ("\"" + jsonEscape(field) + "\"");
      fields += ",\"reason\":" + ("\"" + jsonEscape(reason) + "\"");
      fields += "}";
      std::string message = std::string("InvalidSecretKey") + "(" + "field=" + field + ", reason=" + reason + ")";
      payload = std::string("{\"type\":\"CashuFfiError\",\"kind\":\"InvalidSecretKey\",\"message\":\"") + jsonEscape(message) + "\",\"fields\":" + fields + "}";
      break;
    }
    case 4: {
      std::string id = readString(reader);
      std::string reason = readString(reader);
      std::string fields = "{";
      fields += "\"id\":" + ("\"" + jsonEscape(id) + "\"");
      fields += ",\"reason\":" + ("\"" + jsonEscape(reason) + "\"");
      fields += "}";
      std::string message = std::string("InvalidKeysetId") + "(" + "id=" + id + ", reason=" + reason + ")";
      payload = std::string("{\"type\":\"CashuFfiError\",\"kind\":\"InvalidKeysetId\",\"message\":\"") + jsonEscape(message) + "\",\"fields\":" + fields + "}";
      break;
    }
    case 5: {
      uint64_t length = readU64(reader);
      std::string fields = "{";
      fields += "\"length\":" + ("\"" + std::to_string(length) + "\"");
      fields += "}";
      std::string message = std::string("InvalidSeedLength") + "(" + "length=" + std::to_string(length) + ")";
      payload = std::string("{\"type\":\"CashuFfiError\",\"kind\":\"InvalidSeedLength\",\"message\":\"") + jsonEscape(message) + "\",\"fields\":" + fields + "}";
      break;
    }
    case 6: {
      std::string reason = readString(reader);
      std::string fields = "{";
      fields += "\"reason\":" + ("\"" + jsonEscape(reason) + "\"");
      fields += "}";
      std::string message = std::string("Split") + "(" + "reason=" + reason + ")";
      payload = std::string("{\"type\":\"CashuFfiError\",\"kind\":\"Split\",\"message\":\"") + jsonEscape(message) + "\",\"fields\":" + fields + "}";
      break;
    }
    case 7: {
      std::string reason = readString(reader);
      std::string fields = "{";
      fields += "\"reason\":" + ("\"" + jsonEscape(reason) + "\"");
      fields += "}";
      std::string message = std::string("Dhke") + "(" + "reason=" + reason + ")";
      payload = std::string("{\"type\":\"CashuFfiError\",\"kind\":\"Dhke\",\"message\":\"") + jsonEscape(message) + "\",\"fields\":" + fields + "}";
      break;
    }
    case 8: {
      std::string reason = readString(reader);
      std::string fields = "{";
      fields += "\"reason\":" + ("\"" + jsonEscape(reason) + "\"");
      fields += "}";
      std::string message = std::string("SpendingConditions") + "(" + "reason=" + reason + ")";
      payload = std::string("{\"type\":\"CashuFfiError\",\"kind\":\"SpendingConditions\",\"message\":\"") + jsonEscape(message) + "\",\"fields\":" + fields + "}";
      break;
    }
    case 9: {
      std::string reason = readString(reader);
      std::string fields = "{";
      fields += "\"reason\":" + ("\"" + jsonEscape(reason) + "\"");
      fields += "}";
      std::string message = std::string("Dleq") + "(" + "reason=" + reason + ")";
      payload = std::string("{\"type\":\"CashuFfiError\",\"kind\":\"Dleq\",\"message\":\"") + jsonEscape(message) + "\",\"fields\":" + fields + "}";
      break;
    }
    case 10: {
      uint32_t counter = readU32(reader);
      std::string reason = readString(reader);
      std::string fields = "{";
      fields += "\"counter\":" + std::to_string(counter);
      fields += ",\"reason\":" + ("\"" + jsonEscape(reason) + "\"");
      fields += "}";
      std::string message = std::string("Derivation") + "(" + "counter=" + std::to_string(counter) + ", reason=" + reason + ")";
      payload = std::string("{\"type\":\"CashuFfiError\",\"kind\":\"Derivation\",\"message\":\"") + jsonEscape(message) + "\",\"fields\":" + fields + "}";
      break;
    }
    default:
      payload = "{\"type\":\"CashuFfiError\",\"kind\":\"Unknown\",\"message\":\"unknown variant\",\"fields\":{}}";
  }
  throw CashuFfiError(static_cast<CashuFfiErrorKind>(tag), "uniffi-nitro-error:" + payload);
}

} // namespace

void assertAbiCompatible() {
  uint32_t version = ffi::ffi_cashu_ffi_uniffi_contract_version();
  if (version != kUniffiContractVersion) {
    throw std::runtime_error("uniffi ABI mismatch: rebuild the bindings");
  }
  if (ffi::uniffi_cashu_ffi_checksum_func_blind_message() != 14388) { throw std::runtime_error("uniffi checksum mismatch for uniffi_cashu_ffi_checksum_func_blind_message: rebuild the bindings"); }
  if (ffi::uniffi_cashu_ffi_checksum_func_blind_messages() != 1071) { throw std::runtime_error("uniffi checksum mismatch for uniffi_cashu_ffi_checksum_func_blind_messages: rebuild the bindings"); }
  if (ffi::uniffi_cashu_ffi_checksum_func_create_deterministic_outputs() != 48451) { throw std::runtime_error("uniffi checksum mismatch for uniffi_cashu_ffi_checksum_func_create_deterministic_outputs: rebuild the bindings"); }
  if (ffi::uniffi_cashu_ffi_checksum_func_create_p2pk_outputs() != 47646) { throw std::runtime_error("uniffi checksum mismatch for uniffi_cashu_ffi_checksum_func_create_p2pk_outputs: rebuild the bindings"); }
  if (ffi::uniffi_cashu_ffi_checksum_func_create_random_outputs() != 32634) { throw std::runtime_error("uniffi checksum mismatch for uniffi_cashu_ffi_checksum_func_create_random_outputs: rebuild the bindings"); }
  if (ffi::uniffi_cashu_ffi_checksum_func_create_restore_outputs() != 65503) { throw std::runtime_error("uniffi checksum mismatch for uniffi_cashu_ffi_checksum_func_create_restore_outputs: rebuild the bindings"); }
  if (ffi::uniffi_cashu_ffi_checksum_func_create_single_deterministic_output() != 60767) { throw std::runtime_error("uniffi checksum mismatch for uniffi_cashu_ffi_checksum_func_create_single_deterministic_output: rebuild the bindings"); }
  if (ffi::uniffi_cashu_ffi_checksum_func_create_single_p2pk_output() != 41235) { throw std::runtime_error("uniffi checksum mismatch for uniffi_cashu_ffi_checksum_func_create_single_p2pk_output: rebuild the bindings"); }
  if (ffi::uniffi_cashu_ffi_checksum_func_create_single_random_output() != 53694) { throw std::runtime_error("uniffi checksum mismatch for uniffi_cashu_ffi_checksum_func_create_single_random_output: rebuild the bindings"); }
  if (ffi::uniffi_cashu_ffi_checksum_func_hash_to_curve() != 62146) { throw std::runtime_error("uniffi checksum mismatch for uniffi_cashu_ffi_checksum_func_hash_to_curve: rebuild the bindings"); }
  if (ffi::uniffi_cashu_ffi_checksum_func_keyset_id_v1() != 63806) { throw std::runtime_error("uniffi checksum mismatch for uniffi_cashu_ffi_checksum_func_keyset_id_v1: rebuild the bindings"); }
  if (ffi::uniffi_cashu_ffi_checksum_func_sha256_digest() != 34561) { throw std::runtime_error("uniffi checksum mismatch for uniffi_cashu_ffi_checksum_func_sha256_digest: rebuild the bindings"); }
  if (ffi::uniffi_cashu_ffi_checksum_func_split_amount() != 65348) { throw std::runtime_error("uniffi checksum mismatch for uniffi_cashu_ffi_checksum_func_split_amount: rebuild the bindings"); }
  if (ffi::uniffi_cashu_ffi_checksum_func_unblind_signature() != 55990) { throw std::runtime_error("uniffi checksum mismatch for uniffi_cashu_ffi_checksum_func_unblind_signature: rebuild the bindings"); }
  if (ffi::uniffi_cashu_ffi_checksum_func_verify_proof_dleq() != 47466) { throw std::runtime_error("uniffi checksum mismatch for uniffi_cashu_ffi_checksum_func_verify_proof_dleq: rebuild the bindings"); }
  if (ffi::uniffi_cashu_ffi_checksum_method_deterministicoutputfactory_keyset_id() != 6974) { throw std::runtime_error("uniffi checksum mismatch for uniffi_cashu_ffi_checksum_method_deterministicoutputfactory_keyset_id: rebuild the bindings"); }
  if (ffi::uniffi_cashu_ffi_checksum_method_deterministicoutputfactory_outputs() != 34621) { throw std::runtime_error("uniffi checksum mismatch for uniffi_cashu_ffi_checksum_method_deterministicoutputfactory_outputs: rebuild the bindings"); }
  if (ffi::uniffi_cashu_ffi_checksum_method_deterministicoutputfactory_restore_batch() != 19625) { throw std::runtime_error("uniffi checksum mismatch for uniffi_cashu_ffi_checksum_method_deterministicoutputfactory_restore_batch: rebuild the bindings"); }
  if (ffi::uniffi_cashu_ffi_checksum_method_deterministicoutputfactory_single_output() != 24845) { throw std::runtime_error("uniffi checksum mismatch for uniffi_cashu_ffi_checksum_method_deterministicoutputfactory_single_output: rebuild the bindings"); }
  if (ffi::uniffi_cashu_ffi_checksum_constructor_deterministicoutputfactory_new() != 31949) { throw std::runtime_error("uniffi checksum mismatch for uniffi_cashu_ffi_checksum_constructor_deterministicoutputfactory_new: rebuild the bindings"); }
}

uint64_t DeterministicOutputFactory::cloneHandle() const {
  if (handle_ == 0) {
    throw std::logic_error("use of a native object after close()");
  }
  ffi::RustCallStatus status;
  status.code = 0;
  status.errorBuf = ffi::RustBuffer{0, 0, nullptr};
  uint64_t cloned = ffi::uniffi_cashu_ffi_fn_clone_deterministicoutputfactory(handle_, &status);
  checkUnexpected(status);
  return cloned;
}

void DeterministicOutputFactory::close() noexcept {
  if (handle_ == 0) {
    return;
  }
  ffi::RustCallStatus status;
  status.code = 0;
  status.errorBuf = ffi::RustBuffer{0, 0, nullptr};
  ffi::uniffi_cashu_ffi_fn_free_deterministicoutputfactory(handle_, &status);
  handle_ = 0;
}

std::string DeterministicOutputFactory::keysetId() const {
  ffi::RustCallStatus status;
  status.code = 0;
  status.errorBuf = ffi::RustBuffer{0, 0, nullptr};
  auto rawResult = ffi::uniffi_cashu_ffi_fn_method_deterministicoutputfactory_keyset_id(cloneHandle(), &status);
  checkUnexpected(status);
  return consumeBufferAsString(rawResult);
}

std::vector<BlindedOutput> DeterministicOutputFactory::outputs(const std::vector<uint64_t>& amounts, uint32_t counter) const {
  auto lowered0 = lowerSeqU64(amounts);
  auto lowered1 = counter;
  ffi::RustCallStatus status;
  status.code = 0;
  status.errorBuf = ffi::RustBuffer{0, 0, nullptr};
  auto rawResult = ffi::uniffi_cashu_ffi_fn_method_deterministicoutputfactory_outputs(cloneHandle(), lowered0, lowered1, &status);
  if (status.code == ffi::RUST_CALL_ERROR) {
    throwCashuFfiError(status.errorBuf);
  }
  checkUnexpected(status);
  return liftSeqBlindedOutput(rawResult);
}

std::vector<BlindedOutput> DeterministicOutputFactory::restoreBatch(uint32_t startCounter, uint32_t endCounter) const {
  auto lowered0 = startCounter;
  auto lowered1 = endCounter;
  ffi::RustCallStatus status;
  status.code = 0;
  status.errorBuf = ffi::RustBuffer{0, 0, nullptr};
  auto rawResult = ffi::uniffi_cashu_ffi_fn_method_deterministicoutputfactory_restore_batch(cloneHandle(), lowered0, lowered1, &status);
  if (status.code == ffi::RUST_CALL_ERROR) {
    throwCashuFfiError(status.errorBuf);
  }
  checkUnexpected(status);
  return liftSeqBlindedOutput(rawResult);
}

BlindedOutput DeterministicOutputFactory::singleOutput(uint64_t amount, uint32_t counter) const {
  auto lowered0 = amount;
  auto lowered1 = counter;
  ffi::RustCallStatus status;
  status.code = 0;
  status.errorBuf = ffi::RustBuffer{0, 0, nullptr};
  auto rawResult = ffi::uniffi_cashu_ffi_fn_method_deterministicoutputfactory_single_output(cloneHandle(), lowered0, lowered1, &status);
  if (status.code == ffi::RUST_CALL_ERROR) {
    throwCashuFfiError(status.errorBuf);
  }
  checkUnexpected(status);
  return liftBlindedOutput(rawResult);
}

BlindPair blindMessage(const std::vector<uint8_t>& secret, const std::optional<std::vector<uint8_t>>& blindingFactor) {
  auto lowered0 = lowerBytes(secret);
  auto lowered1 = lowerOptBytes(blindingFactor);
  ffi::RustCallStatus status;
  status.code = 0;
  status.errorBuf = ffi::RustBuffer{0, 0, nullptr};
  auto rawResult = ffi::uniffi_cashu_ffi_fn_func_blind_message(lowered0, lowered1, &status);
  if (status.code == ffi::RUST_CALL_ERROR) {
    throwCashuFfiError(status.errorBuf);
  }
  checkUnexpected(status);
  return liftBlindPair(rawResult);
}

std::vector<BlindPair> blindMessages(const std::vector<std::vector<uint8_t>>& secrets) {
  auto lowered0 = lowerSeqBytes(secrets);
  ffi::RustCallStatus status;
  status.code = 0;
  status.errorBuf = ffi::RustBuffer{0, 0, nullptr};
  auto rawResult = ffi::uniffi_cashu_ffi_fn_func_blind_messages(lowered0, &status);
  if (status.code == ffi::RUST_CALL_ERROR) {
    throwCashuFfiError(status.errorBuf);
  }
  checkUnexpected(status);
  return liftSeqBlindPair(rawResult);
}

std::shared_ptr<DeterministicOutputFactory> createDeterministicOutputFactory(const std::vector<uint8_t>& seed, const std::string& keysetId) {
  auto lowered0 = lowerBytes(seed);
  auto lowered1 = stringToBuffer(keysetId);
  ffi::RustCallStatus status;
  status.code = 0;
  status.errorBuf = ffi::RustBuffer{0, 0, nullptr};
  auto rawResult = ffi::uniffi_cashu_ffi_fn_constructor_deterministicoutputfactory_new(lowered0, lowered1, &status);
  if (status.code == ffi::RUST_CALL_ERROR) {
    throwCashuFfiError(status.errorBuf);
  }
  checkUnexpected(status);
  return std::make_shared<DeterministicOutputFactory>(rawResult);
}

std::vector<BlindedOutput> createDeterministicOutputs(const std::vector<uint64_t>& amounts, const std::vector<uint8_t>& seed, uint32_t counter, const std::string& keysetId) {
  auto lowered0 = lowerSeqU64(amounts);
  auto lowered1 = lowerBytes(seed);
  auto lowered2 = counter;
  auto lowered3 = stringToBuffer(keysetId);
  ffi::RustCallStatus status;
  status.code = 0;
  status.errorBuf = ffi::RustBuffer{0, 0, nullptr};
  auto rawResult = ffi::uniffi_cashu_ffi_fn_func_create_deterministic_outputs(lowered0, lowered1, lowered2, lowered3, &status);
  if (status.code == ffi::RUST_CALL_ERROR) {
    throwCashuFfiError(status.errorBuf);
  }
  checkUnexpected(status);
  return liftSeqBlindedOutput(rawResult);
}

std::vector<BlindedOutput> createP2pkOutputs(const P2pkOptions& p2pk, const std::vector<uint64_t>& amounts, const std::string& keysetId) {
  auto lowered0 = lowerP2pkOptions(p2pk);
  auto lowered1 = lowerSeqU64(amounts);
  auto lowered2 = stringToBuffer(keysetId);
  ffi::RustCallStatus status;
  status.code = 0;
  status.errorBuf = ffi::RustBuffer{0, 0, nullptr};
  auto rawResult = ffi::uniffi_cashu_ffi_fn_func_create_p2pk_outputs(lowered0, lowered1, lowered2, &status);
  if (status.code == ffi::RUST_CALL_ERROR) {
    throwCashuFfiError(status.errorBuf);
  }
  checkUnexpected(status);
  return liftSeqBlindedOutput(rawResult);
}

std::vector<BlindedOutput> createRandomOutputs(const std::vector<uint64_t>& amounts, const std::string& keysetId) {
  auto lowered0 = lowerSeqU64(amounts);
  auto lowered1 = stringToBuffer(keysetId);
  ffi::RustCallStatus status;
  status.code = 0;
  status.errorBuf = ffi::RustBuffer{0, 0, nullptr};
  auto rawResult = ffi::uniffi_cashu_ffi_fn_func_create_random_outputs(lowered0, lowered1, &status);
  if (status.code == ffi::RUST_CALL_ERROR) {
    throwCashuFfiError(status.errorBuf);
  }
  checkUnexpected(status);
  return liftSeqBlindedOutput(rawResult);
}

std::vector<BlindedOutput> createRestoreOutputs(const std::vector<uint8_t>& seed, const std::string& keysetId, uint32_t startCounter, uint32_t endCounter) {
  auto lowered0 = lowerBytes(seed);
  auto lowered1 = stringToBuffer(keysetId);
  auto lowered2 = startCounter;
  auto lowered3 = endCounter;
  ffi::RustCallStatus status;
  status.code = 0;
  status.errorBuf = ffi::RustBuffer{0, 0, nullptr};
  auto rawResult = ffi::uniffi_cashu_ffi_fn_func_create_restore_outputs(lowered0, lowered1, lowered2, lowered3, &status);
  if (status.code == ffi::RUST_CALL_ERROR) {
    throwCashuFfiError(status.errorBuf);
  }
  checkUnexpected(status);
  return liftSeqBlindedOutput(rawResult);
}

BlindedOutput createSingleDeterministicOutput(uint64_t amount, const std::vector<uint8_t>& seed, uint32_t counter, const std::string& keysetId) {
  auto lowered0 = amount;
  auto lowered1 = lowerBytes(seed);
  auto lowered2 = counter;
  auto lowered3 = stringToBuffer(keysetId);
  ffi::RustCallStatus status;
  status.code = 0;
  status.errorBuf = ffi::RustBuffer{0, 0, nullptr};
  auto rawResult = ffi::uniffi_cashu_ffi_fn_func_create_single_deterministic_output(lowered0, lowered1, lowered2, lowered3, &status);
  if (status.code == ffi::RUST_CALL_ERROR) {
    throwCashuFfiError(status.errorBuf);
  }
  checkUnexpected(status);
  return liftBlindedOutput(rawResult);
}

BlindedOutput createSingleP2pkOutput(const P2pkOptions& p2pk, uint64_t amount, const std::string& keysetId) {
  auto lowered0 = lowerP2pkOptions(p2pk);
  auto lowered1 = amount;
  auto lowered2 = stringToBuffer(keysetId);
  ffi::RustCallStatus status;
  status.code = 0;
  status.errorBuf = ffi::RustBuffer{0, 0, nullptr};
  auto rawResult = ffi::uniffi_cashu_ffi_fn_func_create_single_p2pk_output(lowered0, lowered1, lowered2, &status);
  if (status.code == ffi::RUST_CALL_ERROR) {
    throwCashuFfiError(status.errorBuf);
  }
  checkUnexpected(status);
  return liftBlindedOutput(rawResult);
}

BlindedOutput createSingleRandomOutput(uint64_t amount, const std::string& keysetId) {
  auto lowered0 = amount;
  auto lowered1 = stringToBuffer(keysetId);
  ffi::RustCallStatus status;
  status.code = 0;
  status.errorBuf = ffi::RustBuffer{0, 0, nullptr};
  auto rawResult = ffi::uniffi_cashu_ffi_fn_func_create_single_random_output(lowered0, lowered1, &status);
  if (status.code == ffi::RUST_CALL_ERROR) {
    throwCashuFfiError(status.errorBuf);
  }
  checkUnexpected(status);
  return liftBlindedOutput(rawResult);
}

std::vector<uint8_t> hashToCurve(const std::vector<uint8_t>& message) {
  auto lowered0 = lowerBytes(message);
  ffi::RustCallStatus status;
  status.code = 0;
  status.errorBuf = ffi::RustBuffer{0, 0, nullptr};
  auto rawResult = ffi::uniffi_cashu_ffi_fn_func_hash_to_curve(lowered0, &status);
  if (status.code == ffi::RUST_CALL_ERROR) {
    throwCashuFfiError(status.errorBuf);
  }
  checkUnexpected(status);
  return liftBytes(rawResult);
}

std::string keysetIdV1(const std::vector<KeyEntry>& keys) {
  auto lowered0 = lowerSeqKeyEntry(keys);
  ffi::RustCallStatus status;
  status.code = 0;
  status.errorBuf = ffi::RustBuffer{0, 0, nullptr};
  auto rawResult = ffi::uniffi_cashu_ffi_fn_func_keyset_id_v1(lowered0, &status);
  if (status.code == ffi::RUST_CALL_ERROR) {
    throwCashuFfiError(status.errorBuf);
  }
  checkUnexpected(status);
  return consumeBufferAsString(rawResult);
}

std::vector<uint8_t> sha256Digest(const std::vector<uint8_t>& data) {
  auto lowered0 = lowerBytes(data);
  ffi::RustCallStatus status;
  status.code = 0;
  status.errorBuf = ffi::RustBuffer{0, 0, nullptr};
  auto rawResult = ffi::uniffi_cashu_ffi_fn_func_sha256_digest(lowered0, &status);
  checkUnexpected(status);
  return liftBytes(rawResult);
}

std::vector<uint64_t> splitAmount(uint64_t amount, const std::vector<uint64_t>& denominations, const std::optional<std::vector<uint64_t>>& customSplit) {
  auto lowered0 = amount;
  auto lowered1 = lowerSeqU64(denominations);
  auto lowered2 = lowerOptSeqU64(customSplit);
  ffi::RustCallStatus status;
  status.code = 0;
  status.errorBuf = ffi::RustBuffer{0, 0, nullptr};
  auto rawResult = ffi::uniffi_cashu_ffi_fn_func_split_amount(lowered0, lowered1, lowered2, &status);
  if (status.code == ffi::RUST_CALL_ERROR) {
    throwCashuFfiError(status.errorBuf);
  }
  checkUnexpected(status);
  return liftSeqU64(rawResult);
}

std::string unblindSignature(const std::string& blindedSignature, const std::string& blindingFactor, const std::string& mintPubkey) {
  auto lowered0 = stringToBuffer(blindedSignature);
  auto lowered1 = stringToBuffer(blindingFactor);
  auto lowered2 = stringToBuffer(mintPubkey);
  ffi::RustCallStatus status;
  status.code = 0;
  status.errorBuf = ffi::RustBuffer{0, 0, nullptr};
  auto rawResult = ffi::uniffi_cashu_ffi_fn_func_unblind_signature(lowered0, lowered1, lowered2, &status);
  if (status.code == ffi::RUST_CALL_ERROR) {
    throwCashuFfiError(status.errorBuf);
  }
  checkUnexpected(status);
  return consumeBufferAsString(rawResult);
}

bool verifyProofDleq(const std::string& secret, const std::string& unblindedSignature, const DleqProof& dleq, const std::string& blindingFactor, const std::string& mintPubkey) {
  auto lowered0 = stringToBuffer(secret);
  auto lowered1 = stringToBuffer(unblindedSignature);
  auto lowered2 = lowerDleqProof(dleq);
  auto lowered3 = stringToBuffer(blindingFactor);
  auto lowered4 = stringToBuffer(mintPubkey);
  ffi::RustCallStatus status;
  status.code = 0;
  status.errorBuf = ffi::RustBuffer{0, 0, nullptr};
  auto rawResult = ffi::uniffi_cashu_ffi_fn_func_verify_proof_dleq(lowered0, lowered1, lowered2, lowered3, lowered4, &status);
  if (status.code == ffi::RUST_CALL_ERROR) {
    throwCashuFfiError(status.errorBuf);
  }
  checkUnexpected(status);
  return (rawResult != 0);
}

} // namespace cashucrypto::bridge
