// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.
//
// android-base/memory.h shim — `get_unaligned` / `put_unaligned`.
// Header-only template helpers that don't depend on anything else.
// Mirrors the upstream signature exactly.

#pragma once

#include <cstring>

namespace android {
namespace base {

template <typename T>
static inline T get_unaligned(const void* address) {
  T result;
  std::memcpy(&result, address, sizeof(T));
  return result;
}

template <typename T>
static inline void put_unaligned(void* address, T v) {
  std::memcpy(address, &v, sizeof(T));
}

}  // namespace base
}  // namespace android
