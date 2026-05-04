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

// -------- CDR error taxonomy (matches Lean + Rust ParseError tags) --
constexpr uint8_t kCdrShortHeader   = 1;
constexpr uint8_t kCdrBadSignature  = 2;
constexpr uint8_t kCdrShortName     = 3;
constexpr uint8_t kCdrShortExtra    = 4;
constexpr uint8_t kCdrShortComment  = 5;

constexpr size_t kLfhFixedSize  = sizeof(LocalFileHeader);
constexpr size_t kEocdFixedSize = sizeof(EocdRecord);
constexpr size_t kCdrFixedSize  = sizeof(CentralDirectoryRecord);

static_assert(kLfhFixedSize  == 30, "LFH struct must be 30 bytes");
static_assert(kEocdFixedSize == 22, "EOCD struct must be 22 bytes");
static_assert(kCdrFixedSize  == 46, "CDR struct must be 46 bytes");

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

int parse_cdr(const std::vector<uint8_t>& bs) {
  if (bs.size() < kCdrFixedSize) {
    print_err(kCdrShortHeader);
    return 0;
  }
  CentralDirectoryRecord rec;
  std::memcpy(&rec, bs.data(), kCdrFixedSize);
  if (rec.record_signature != CentralDirectoryRecord::kSignature) {
    print_err(kCdrBadSignature);
    return 0;
  }
  size_t name_end    = kCdrFixedSize + rec.file_name_length;
  size_t extra_end   = name_end + rec.extra_field_length;
  size_t comment_end = extra_end + rec.comment_length;
  if (name_end > bs.size()) {
    print_err(kCdrShortName);
    return 0;
  }
  if (extra_end > bs.size()) {
    print_err(kCdrShortExtra);
    return 0;
  }
  if (comment_end > bs.size()) {
    print_err(kCdrShortComment);
    return 0;
  }
  print_ok(comment_end);
  return 0;
}

// -------- Archive error taxonomy (matches Lean ArchiveError tags) ----
constexpr uint8_t kArchiveNoEocd            = 1;
constexpr uint8_t kArchiveEocdInvalid       = 2;
constexpr uint8_t kArchiveCdOutOfRange      = 3;
constexpr uint8_t kArchiveCdrInvalid        = 4;
constexpr uint8_t kArchiveCdrCountMismatch  = 5;
constexpr uint8_t kArchiveLfhOffsetOob      = 6;
constexpr uint8_t kArchiveLfhInvalid        = 7;
constexpr uint8_t kArchiveFilenameMismatch  = 8;
constexpr uint8_t kArchiveFieldMismatch     = 9;
constexpr uint8_t kArchiveEocdTooFarFromEof = 10;
constexpr uint8_t kArchiveCdAfterEocd       = 11;
constexpr uint8_t kArchiveInvalidEntryName  = 12;

// AOSP `kMaxEOCDSearch` = `kMaxCommentLen + sizeof(EocdRecord)`.
constexpr size_t kMaxEocdSearch = 65557;

// Validate a filename byte sequence per AOSP's `IsValidEntryName`
// (entry_name_utils-inl.h). Rejects NUL bytes + invalid UTF-8.
bool is_valid_entry_name(const uint8_t* name, size_t len) {
  if (len > 0xffff) return false;
  for (size_t i = 0; i < len; ++i) {
    uint8_t b = name[i];
    if (b == 0) return false;
    if ((b & 0x80) == 0) continue;
    if ((b & 0xc0) == 0x80 || (b & 0xfe) == 0xfe) return false;
    uint8_t first = static_cast<uint8_t>((b & 0x7f) << 1);
    while (first & 0x80) {
      if (++i >= len) return false;
      uint8_t cont = name[i];
      if ((cont & 0xc0) != 0x80) return false;
      first = static_cast<uint8_t>((first & 0x7f) << 1);
    }
  }
  return true;
}

// Suffix-locate the EOCD signature by scanning backwards from EOF.
// Returns size_t(-1) if not found.
size_t find_eocd(const std::vector<uint8_t>& bs) {
  if (bs.size() < kEocdFixedSize) return static_cast<size_t>(-1);
  for (size_t off = bs.size() - kEocdFixedSize;; --off) {
    if (off + 4 <= bs.size()) {
      uint32_t sig;
      std::memcpy(&sig, bs.data() + off, 4);
      if (sig == EocdRecord::kSignature) return off;
    }
    if (off == 0) break;
  }
  return static_cast<size_t>(-1);
}

// Whole-archive parse mirroring the Lean / Rust drivers. Uses AOSP's
// authoritative LFH / CDR / EOCD struct definitions for wire-format
// truth; applies the APKAXIOM cross-record consistency rules
// (filename agreement, lfh-offset bounds) on top.
int parse_archive(const std::vector<uint8_t>& bs) {
  // (1) Locate the EOCD.
  size_t eocd_off = find_eocd(bs);
  if (eocd_off == static_cast<size_t>(-1)) {
    print_err(kArchiveNoEocd);
    return 0;
  }
  // (1½) Runtime parity: EOCD must be within kMaxEocdSearch bytes
  // of EOF.
  if (bs.size() > eocd_off + kMaxEocdSearch) {
    print_err(kArchiveEocdTooFarFromEof);
    return 0;
  }
  // (2) Parse the EOCD record at the located offset.
  if (eocd_off + kEocdFixedSize > bs.size()) {
    print_err(kArchiveEocdInvalid);
    return 0;
  }
  EocdRecord eocd;
  std::memcpy(&eocd, bs.data() + eocd_off, kEocdFixedSize);
  if (eocd.eocd_signature != EocdRecord::kSignature) {
    print_err(kArchiveEocdInvalid);
    return 0;
  }
  if (eocd.disk_num != eocd.cd_start_disk) {
    print_err(kArchiveEocdInvalid);
    return 0;
  }
  size_t comment_end = eocd_off + kEocdFixedSize + eocd.comment_length;
  if (comment_end > bs.size()) {
    print_err(kArchiveEocdInvalid);
    return 0;
  }
  // (3) Validate CD bounds.
  size_t cd_start = eocd.cd_start_offset;
  size_t cd_size  = eocd.cd_size;
  if (cd_start + cd_size > bs.size()) {
    print_err(kArchiveCdOutOfRange);
    return 0;
  }
  // (3½) Runtime parity: CD must end before the EOCD.
  if (cd_start + cd_size > eocd_off) {
    print_err(kArchiveCdAfterEocd);
    return 0;
  }
  // (4) Parse the CDR sequence in the CD region.
  std::vector<CentralDirectoryRecord> cdrs;
  std::vector<std::vector<uint8_t>> cdr_filenames;
  std::vector<uint32_t> cdr_lfh_offsets;
  size_t off = cd_start;
  size_t cd_end = cd_start + cd_size;
  while (off < cd_end) {
    if (off + kCdrFixedSize > cd_end) {
      print_err(kArchiveCdrInvalid);
      return 0;
    }
    CentralDirectoryRecord cdr;
    std::memcpy(&cdr, bs.data() + off, kCdrFixedSize);
    if (cdr.record_signature != CentralDirectoryRecord::kSignature) {
      print_err(kArchiveCdrInvalid);
      return 0;
    }
    size_t name_off = off + kCdrFixedSize;
    size_t extra_off = name_off + cdr.file_name_length;
    size_t comment_off = extra_off + cdr.extra_field_length;
    size_t cdr_end = comment_off + cdr.comment_length;
    if (cdr_end > cd_end) {
      print_err(kArchiveCdrInvalid);
      return 0;
    }
    std::vector<uint8_t> fname(bs.data() + name_off,
                               bs.data() + extra_off);
    cdr_filenames.push_back(fname);
    cdr_lfh_offsets.push_back(cdr.local_file_header_offset);
    cdrs.push_back(cdr);
    off = cdr_end;
  }
  // (4½) Runtime parity: every CDR's filename must be a valid entry
  // name (UTF-8, no NUL bytes).
  for (const auto& fname : cdr_filenames) {
    if (!is_valid_entry_name(fname.data(), fname.size())) {
      print_err(kArchiveInvalidEntryName);
      return 0;
    }
  }
  // (5) Count agreement.
  if (cdrs.size() != eocd.num_records) {
    print_err(kArchiveCdrCountMismatch);
    return 0;
  }
  // (6) Per-CDR LFH consistency.
  for (size_t i = 0; i < cdrs.size(); ++i) {
    size_t lo = cdr_lfh_offsets[i];
    if (lo + kLfhFixedSize > bs.size()) {
      print_err(kArchiveLfhOffsetOob);
      return 0;
    }
    LocalFileHeader lfh;
    std::memcpy(&lfh, bs.data() + lo, kLfhFixedSize);
    if (lfh.lfh_signature != LocalFileHeader::kSignature) {
      print_err(kArchiveLfhInvalid);
      return 0;
    }
    size_t lfh_name_end = lo + kLfhFixedSize + lfh.file_name_length;
    size_t lfh_extra_end = lfh_name_end + lfh.extra_field_length;
    if (lfh_extra_end > bs.size()) {
      print_err(kArchiveLfhInvalid);
      return 0;
    }
    std::vector<uint8_t> lfh_fname(bs.data() + lo + kLfhFixedSize,
                                   bs.data() + lfh_name_end);
    if (lfh_fname != cdr_filenames[i]) {
      print_err(kArchiveFilenameMismatch);
      return 0;
    }
    // Field-set consistency. Two cases (mirrors the Lean
    // `cdrLfhFieldsAgree` definition):
    //   1. No data descriptor (LFH bit 3 unset): strict equality on
    //      crc32 / compressed_size / uncompressed_size /
    //      compression_method.
    //   2. Data descriptor present (LFH bit 3 set): LFH's three
    //      crc/size fields must be zero (per APPNOTE.TXT §4.4.4),
    //      compression_method must still agree.
    const auto& cdr = cdrs[i];
    constexpr uint16_t kGpbDataDescriptorMask = 0x0008;
    bool dd = (lfh.gpb_flags & kGpbDataDescriptorMask) != 0;
    if (dd) {
      if (lfh.crc32              != 0 ||
          lfh.compressed_size    != 0 ||
          lfh.uncompressed_size  != 0 ||
          cdr.compression_method != lfh.compression_method) {
        print_err(kArchiveFieldMismatch);
        return 0;
      }
    } else {
      if (cdr.crc32              != lfh.crc32 ||
          cdr.compressed_size    != lfh.compressed_size ||
          cdr.uncompressed_size  != lfh.uncompressed_size ||
          cdr.compression_method != lfh.compression_method) {
        print_err(kArchiveFieldMismatch);
        return 0;
      }
    }
  }
  print_ok(bs.size());
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
    std::fprintf(stderr, "usage: zip-aosp-probe --lfh | --eocd | --cdr\n");
    return 2;
  }
  const std::string mode(argv[1]);
  const auto bs = read_stdin();
  if (mode == "--lfh")     return parse_lfh(bs);
  if (mode == "--eocd")    return parse_eocd(bs);
  if (mode == "--cdr")     return parse_cdr(bs);
  if (mode == "--archive") return parse_archive(bs);
  std::fprintf(stderr, "unknown mode: %s\n", mode.c_str());
  return 2;
}
