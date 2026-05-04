// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.
//
// android-base/mapped_file.h stub.
//
// The runtime probe uses `OpenArchiveFromMemory`, which never
// constructs a `MappedFile` — the field exists in `MappedZipFile`
// but is left null. The stub provides just enough of the type +
// API surface for zip_archive.cc to compile; any call to the
// stubbed methods at runtime is a programming error and aborts.

#pragma once

#include <sys/types.h>
#include <cstdlib>
#include <memory>

#include <android-base/macros.h>

namespace android {
namespace base {

class MappedFile {
 public:
  // FromFd is only called from OpenArchive (file path) — unused in
  // the in-memory probe. We provide a stub that aborts so a
  // production accidental call is loud rather than silent.
  static std::unique_ptr<MappedFile> FromFd(int /*fd*/, off64_t /*offset*/,
                                            size_t /*length*/, int /*prot*/) {
    std::abort();
  }

  static std::unique_ptr<MappedFile> FromOsHandle(void* /*handle*/, off64_t /*offset*/,
                                                  size_t /*length*/, int /*prot*/) {
    std::abort();
  }

  char* data() const { return data_; }
  size_t size() const { return size_; }

  ~MappedFile() = default;

 private:
  MappedFile(char* data, size_t size) : data_(data), size_(size) {}
  char* data_ = nullptr;
  size_t size_ = 0;

  DISALLOW_COPY_AND_ASSIGN(MappedFile);
};

}  // namespace base
}  // namespace android
