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
// the copy-assignment operator. For a *header-only* read-only
// probe that just `memcpy`s bytes into the structs, the macro is
// pure documentation.
#define DISALLOW_COPY_AND_ASSIGN(TypeName) \
  TypeName(const TypeName&) = delete;      \
  TypeName& operator=(const TypeName&) = delete
#endif
