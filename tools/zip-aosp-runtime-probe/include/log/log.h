// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.
//
// log/log.h stub. AOSP `liblog` macros become no-ops in the
// runtime probe — the probe is headless and the log output isn't
// part of the differential signal.

#pragma once

#define ALOGV(...) ((void)0)
#define ALOGD(...) ((void)0)
#define ALOGI(...) ((void)0)
#define ALOGW(...) ((void)0)
#define ALOGE(...) ((void)0)
#define ALOGF(...) ((void)0)
#define ALOG_ASSERT(cond, ...) ((void)0)

#define IF_ALOGV() if (false)
