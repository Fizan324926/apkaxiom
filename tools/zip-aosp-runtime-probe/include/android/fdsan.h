// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.
//
// android/fdsan.h stub. Bionic-only file-descriptor sanitiser API.
// On Linux (non-Android) we no-op all operations.

#pragma once

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

enum android_fdsan_owner_type {
    ANDROID_FDSAN_OWNER_TYPE_GENERIC_00 = 0,
    ANDROID_FDSAN_OWNER_TYPE_GENERIC_FF = 255,
    ANDROID_FDSAN_OWNER_TYPE_FILE = 1,
};

static inline uint64_t android_fdsan_create_owner_tag(
    enum android_fdsan_owner_type /*type*/, uint64_t /*tag*/) {
    return 0;
}

static inline void android_fdsan_exchange_owner_tag(
    int /*fd*/, uint64_t /*expected_tag*/, uint64_t /*new_tag*/) {}

static inline int android_fdsan_close_with_tag(int fd, uint64_t /*tag*/) {
    extern int close(int);
    return close(fd);
}

#ifdef __cplusplus
}
#endif
