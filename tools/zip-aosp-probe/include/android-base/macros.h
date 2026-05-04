// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.
//
// Minimal shim for `android-base/macros.h`. The vendored
// `external/libziparchive/zip_archive_common.h` uses only one
// macro: `DISALLOW_COPY_AND_ASSIGN`. We provide an empty
// definition so the wire-format struct declarations compile
// against stock g++ without pulling in libbase.

#pragma once

#ifndef DISALLOW_COPY_AND_ASSIGN
// In production AOSP this macro deletes the copy constructor and
// the copy-assignment operator. For our header-only read-only probe
// that `memcpy`s bytes into the structs and stores them in
// `std::vector<…>`, deleting copy makes the structs non-vector-able.
// Define the macro to a no-op declaration so the structs remain
// trivially copyable for our purposes — production AOSP integrity
// is unaffected because we don't link against AOSP runtime code.
#define DISALLOW_COPY_AND_ASSIGN(TypeName)
#endif
