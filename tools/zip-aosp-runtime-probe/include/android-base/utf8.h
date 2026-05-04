// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.
//
// android-base/utf8.h stub. Only used by OpenArchive (file-path
// constructor). The runtime probe uses OpenArchiveFromMemory which
// never invokes utf8::open or utf8::stat.

#pragma once

#include <cstdlib>

namespace android {
namespace base {
namespace utf8 {

inline int open(const char* /*path*/, int /*flags*/, ...) { std::abort(); }
inline int unlink(const char* /*path*/) { std::abort(); }

}  // namespace utf8
}  // namespace base
}  // namespace android
