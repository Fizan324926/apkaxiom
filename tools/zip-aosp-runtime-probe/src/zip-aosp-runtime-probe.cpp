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
    // Map AOSP's negative ZipError codes to a single archive-rejected
    // tag (100). The differential harness compares (ok | reject)
    // categories — we don't claim 1:1 enum parity with our 12-tag
    // ArchiveError. The Lean / Rust / struct-probe legs already
    // give per-tag agreement; this leg supplies the binary
    // accept/reject signal from the actual AOSP runtime.
    (void)code;
    std::printf("err %u\n", static_cast<unsigned>(kRuntimeReject));
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

int main(int argc, char** argv) {
    if (argc != 2 || std::strcmp(argv[1], "--archive-runtime") != 0) {
        std::fprintf(stderr, "usage: zip-aosp-runtime-probe --archive-runtime\n");
        return 2;
    }
    const auto bs = read_stdin();
    return parse_archive_runtime(bs);
}
