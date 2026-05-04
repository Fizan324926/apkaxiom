// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.
//
// incfs_support/util.h stub.

#pragma once

namespace incfs {
inline bool isIncFsFd(int /*fd*/) { return false; }
inline bool isIncFsPath(const char* /*path*/) { return false; }
inline bool OnIncfs(int /*fd*/) { return false; }
}  // namespace incfs

// Some references use the unscoped name.
inline bool OnIncfs(int fd) { return incfs::OnIncfs(fd); }
