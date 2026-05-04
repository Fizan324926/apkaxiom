// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.
//
// android-base/file.h stub. The in-memory probe doesn't open files
// or extract entries, so all functions here are stubs that abort if
// called.

#pragma once

#include <cstdlib>
#include <cstddef>
#include <sys/types.h>

#include <android-base/macros.h>

namespace android {
namespace base {

inline bool ReadFully(int /*fd*/, void* /*buf*/, size_t /*size*/) {
  std::abort();
}

inline bool ReadFullyAtOffset(int /*fd*/, void* /*buf*/, size_t /*size*/,
                              off64_t /*offset*/) {
  std::abort();
}

inline bool WriteFully(int /*fd*/, const void* /*buf*/, size_t /*size*/) {
  std::abort();
}

class unique_fd {
 public:
  unique_fd() : fd_(-1) {}
  explicit unique_fd(int fd) : fd_(fd) {}
  ~unique_fd() = default;
  int get() const { return fd_; }
  int release() {
    int f = fd_;
    fd_ = -1;
    return f;
  }
 private:
  int fd_;
  DISALLOW_COPY_AND_ASSIGN(unique_fd);
};

}  // namespace base
}  // namespace android
