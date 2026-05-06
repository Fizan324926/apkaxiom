// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.
//
// P1.6 §I final closure — AOSP runtime source-link probe.
//
// Stable Lean ↔ Rust ↔ AOSP-struct probe (--archive in
// tools/zip-aosp-probe) covers wire-format truth via memcpy into
// AOSP's authoritative struct definitions. *This* probe goes one
// step further and links AOSP's actual zip_archive.cc runtime,
// then calls OpenArchiveFromMemory on the input bytes.
//
// What's vendored / stubbed:
//   - external/libziparchive/zip_archive.cc            — the runtime
//   - external/libziparchive/zip_archive_stream_entry.cc — iter
//   - external/libziparchive/zip_cd_entry_map.cc       — CD map
//   - external/libziparchive/zip_error.cpp             — error strings
//   - tools/zip-aosp-runtime-probe/include/...         — libbase shims
//
// Modes: --archive-runtime (the only mode). Returns the same
// `ok N` / `err T` shape the other probes use; `T` is AOSP's
// `ZipError` integer (1..=9 enum range).

#include <cstdint>
#include <cstdio>
#include <cstring>
#include <vector>

#include <ziparchive/zip_archive.h>

namespace {

constexpr uint8_t kRuntimeOk     = 0;
constexpr uint8_t kRuntimeReject = 100;

void print_ok(size_t n) { std::printf("ok %zu\n", n); }
void print_err(int code) {
    // Print AOSP's signed ZipError code on the err line so
    // downstream parity tools can categorise rejections by
    // reason (kInvalidFile=-3, kEmptyArchive=-6, kInvalidOffset=-8,
    // kInconsistentInformation=-9, kInvalidEntryName=-10, …).
    // Format: `err <reject-tag> <aosp-code>`. The reject tag
    // (100) is preserved for backward compatibility with the
    // P1.6 differential harness that only does accept/reject.
    std::printf("err %u %d\n", static_cast<unsigned>(kRuntimeReject), code);
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

int parse_archive_runtime(const std::vector<uint8_t>& bs) {
    ZipArchiveHandle handle = nullptr;
    int32_t rc = OpenArchiveFromMemory(bs.data(), bs.size(),
                                        "apkaxiom-runtime-probe", &handle);
    if (rc != 0) {
        if (handle) CloseArchive(handle);
        print_err(rc);
        return 0;
    }
    print_ok(bs.size());
    CloseArchive(handle);
    return 0;
}

}  // anonymous namespace

/// Persistent-mode protocol (`--archive-runtime-server`).
///
/// One process serves N inputs over its lifetime. Input frame:
///
///   <u32 length, little-endian> <length bytes>
///
/// Reply frame:
///
///   "ok <len>\n"             — archive accepted, len bytes consumed
///   "err <reject> <code>\n"  — rejected, with AOSP signed ZipError
///
/// EOF on stdin terminates the server cleanly. Used by the
/// p113-fuzz harness to amortise the ~20 ms process-startup
/// cost of the per-call mode across many iterations (~100x
/// speedup on 1 000-iter runs).
int run_archive_runtime_server() {
    while (true) {
        uint8_t hdr[4];
        size_t n = std::fread(hdr, 1, 4, stdin);
        if (n == 0) return 0; // clean EOF
        if (n != 4) {
            std::fprintf(stderr, "short frame header (%zu bytes)\n", n);
            return 1;
        }
        uint32_t len = uint32_t(hdr[0])
                     | (uint32_t(hdr[1]) << 8)
                     | (uint32_t(hdr[2]) << 16)
                     | (uint32_t(hdr[3]) << 24);
        if (len > (256u * 1024u * 1024u)) {
            std::fprintf(stderr, "frame too large (%u bytes)\n", len);
            return 1;
        }
        std::vector<uint8_t> bs(len);
        if (len > 0) {
            size_t got = std::fread(bs.data(), 1, len, stdin);
            if (got != len) {
                std::fprintf(stderr, "short frame body (%zu/%u)\n", got, len);
                return 1;
            }
        }
        ZipArchiveHandle handle = nullptr;
        int32_t rc = OpenArchiveFromMemory(bs.data(), bs.size(),
                                            "apkaxiom-runtime-probe", &handle);
        if (rc != 0) {
            if (handle) CloseArchive(handle);
            print_err(rc);
        } else {
            print_ok(bs.size());
            CloseArchive(handle);
        }
        std::fflush(stdout);
    }
}

int main(int argc, char** argv) {
    if (argc == 2 && std::strcmp(argv[1], "--archive-runtime") == 0) {
        const auto bs = read_stdin();
        return parse_archive_runtime(bs);
    }
    if (argc == 2 && std::strcmp(argv[1], "--archive-runtime-server") == 0) {
        return run_archive_runtime_server();
    }
    std::fprintf(stderr, "usage: zip-aosp-runtime-probe --archive-runtime|--archive-runtime-server\n");
    return 2;
}
