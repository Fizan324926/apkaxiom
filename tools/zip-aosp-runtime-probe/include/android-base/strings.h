// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.
//
// android-base/strings.h shim — minimal `StartsWith` / `EndsWith`
// used by `IterationHandle::Match`. Header-only.

#pragma once

#include <string>
#include <string_view>

namespace android {
namespace base {

inline bool StartsWith(std::string_view s, std::string_view prefix) {
  return s.size() >= prefix.size() && s.compare(0, prefix.size(), prefix) == 0;
}

inline bool EndsWith(std::string_view s, std::string_view suffix) {
  return s.size() >= suffix.size()
      && s.compare(s.size() - suffix.size(), suffix.size(), suffix) == 0;
}

}  // namespace base
}  // namespace android
