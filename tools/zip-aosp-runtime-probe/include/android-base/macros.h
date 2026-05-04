// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.
//
// android-base/macros.h shim for the runtime probe.
// Same posture as tools/zip-aosp-probe — production AOSP's macro
// declares deleted copy/assignment, but our header-only memcpy
// usage requires copy-construction for std::vector storage. The
// production-AOSP build path is unaffected because we don't link
// AOSP runtime code into Android.

#pragma once

#include <cstddef>

#ifndef DISALLOW_COPY_AND_ASSIGN
#define DISALLOW_COPY_AND_ASSIGN(TypeName)
#endif

#ifndef TEMP_FAILURE_RETRY
// `TEMP_FAILURE_RETRY` retries on EINTR; for our in-memory probe we
// never make syscalls that get interrupted, so this is the identity.
#define TEMP_FAILURE_RETRY(expr) (expr)
#endif

#ifndef O_BINARY
#define O_BINARY 0
#endif

#ifndef WARN_UNUSED
#define WARN_UNUSED
#endif

#ifndef ABORT_ON_ERROR
#define ABORT_ON_ERROR(...) ((void)0)
#endif

// arraysize used by zip_error.cpp.
#ifndef arraysize
template <typename T, size_t N>
constexpr size_t _arraysize_impl(T (&)[N]) noexcept { return N; }
#define arraysize(a) _arraysize_impl(a)
#endif
