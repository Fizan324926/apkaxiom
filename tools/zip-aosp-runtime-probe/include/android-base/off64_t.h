// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.
//
// android-base/off64_t.h shim. On Linux glibc, off64_t is in
// <sys/types.h> with _LARGEFILE64_SOURCE.

#pragma once

#ifndef _LARGEFILE64_SOURCE
#define _LARGEFILE64_SOURCE 1
#endif
#ifndef _FILE_OFFSET_BITS
#define _FILE_OFFSET_BITS 64
#endif

#include <sys/types.h>
#include <unistd.h>

#ifndef __off64_t_defined
#ifndef __APPLE__
// off64_t is provided by glibc with _LARGEFILE64_SOURCE.
#endif
#endif
