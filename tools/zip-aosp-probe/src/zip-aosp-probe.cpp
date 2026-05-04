// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.
//
// P1.5 — third-prong differential probe.
//
// Reads a single ZIP byte sequence from stdin, parses it via AOSP's
// authoritative wire-format struct definitions (vendored at
// `external/libziparchive/zip_archive_common.h`), and prints the
// verdict in the same shape Lean and Rust use:
//
//     ok <consumed-bytes>
//     err <tag>           (tag values match the Lean / Rust enums)
//
// We don't link AOSP's `zip_archive.cc` runtime (that pulls libbase /
// liblog / etc.); the wire-format structs are header-only and that's
// the authoritative thing we want third-prong agreement on.
//
// Two modes: --lfh (parse a Local File Header) and --eocd (parse an
// End Of Central Directory record). The differential harness picks
// the mode per corpus subdirectory.

#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <string>
#include <vector>

// AOSP-canonical wire-format definitions. We pull only the struct
// declarations from the vendored upstream tree; the
// `DISALLOW_COPY_AND_ASSIGN` macro is provided by the shim at
// `tools/zip-aosp-probe/include/android-base/macros.h` (see the
// -I flag in `make p15-aosp-probe`).
#include "external/libziparchive/zip_archive_common.h"

namespace {

// -------- LFH error taxonomy (matches Lean + Rust ParseError tags) --
constexpr uint8_t kLfhShortHeader  = 1;
constexpr uint8_t kLfhBadSignature = 2;
constexpr uint8_t kLfhShortName    = 3;
constexpr uint8_t kLfhShortExtra   = 4;

// -------- EOCD error taxonomy (matches Lean + Rust ParseError tags) --
constexpr uint8_t kEocdShortFixed         = 1;
constexpr uint8_t kEocdBadSignature       = 2;
constexpr uint8_t kEocdShortComment       = 3;
constexpr uint8_t kEocdInconsistentDisks  = 4;

constexpr size_t kLfhFixedSize  = sizeof(LocalFileHeader);
constexpr size_t kEocdFixedSize = sizeof(EocdRecord);

static_assert(kLfhFixedSize  == 30, "LFH struct must be 30 bytes");
static_assert(kEocdFixedSize == 22, "EOCD struct must be 22 bytes");

void print_ok(size_t consumed) {
  std::printf("ok %zu\n", consumed);
}

void print_err(uint8_t tag) {
  std::printf("err %u\n", static_cast<unsigned>(tag));
}

int parse_lfh(const std::vector<uint8_t>& bs) {
  if (bs.size() < kLfhFixedSize) {
    print_err(kLfhShortHeader);
    return 0;
  }
  LocalFileHeader hdr;
  std::memcpy(&hdr, bs.data(), kLfhFixedSize);
  if (hdr.lfh_signature != LocalFileHeader::kSignature) {
    print_err(kLfhBadSignature);
    return 0;
  }
  size_t name_end  = kLfhFixedSize + hdr.file_name_length;
  size_t extra_end = name_end + hdr.extra_field_length;
  if (name_end > bs.size()) {
    print_err(kLfhShortName);
    return 0;
  }
  if (extra_end > bs.size()) {
    print_err(kLfhShortExtra);
    return 0;
  }
  print_ok(extra_end);
  return 0;
}

int parse_eocd(const std::vector<uint8_t>& bs) {
  if (bs.size() < kEocdFixedSize) {
    print_err(kEocdShortFixed);
    return 0;
  }
  EocdRecord rec;
  std::memcpy(&rec, bs.data(), kEocdFixedSize);
  if (rec.eocd_signature != EocdRecord::kSignature) {
    print_err(kEocdBadSignature);
    return 0;
  }
  if (rec.disk_num != rec.cd_start_disk) {
    print_err(kEocdInconsistentDisks);
    return 0;
  }
  size_t comment_end = kEocdFixedSize + rec.comment_length;
  if (comment_end > bs.size()) {
    print_err(kEocdShortComment);
    return 0;
  }
  print_ok(comment_end);
  return 0;
}

std::vector<uint8_t> read_stdin() {
  std::vector<uint8_t> bs;
  uint8_t buf[4096];
  size_t n;
  while ((n = std::fread(buf, 1, sizeof(buf), stdin)) > 0) {
    bs.insert(bs.end(), buf, buf + n);
  }
  return bs;
}

}  // anonymous namespace

int main(int argc, char** argv) {
  if (argc != 2) {
    std::fprintf(stderr, "usage: zip-aosp-probe --lfh | --eocd\n");
    return 2;
  }
  const std::string mode(argv[1]);
  const auto bs = read_stdin();
  if (mode == "--lfh")  return parse_lfh(bs);
  if (mode == "--eocd") return parse_eocd(bs);
  std::fprintf(stderr, "unknown mode: %s\n", mode.c_str());
  return 2;
}
