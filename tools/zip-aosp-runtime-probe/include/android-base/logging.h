// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.
//
// android-base/logging.h shim — provides a stream-discarding LOG
// macro plus CHECK family no-ops. The probe is headless.

#pragma once

#include <ostream>
#include <sstream>

namespace android {
namespace base {

// A throw-away ostream sink. Anything streamed into it is dropped.
class NullStream : public std::ostream {
 public:
  NullStream() : std::ostream(nullptr) {}
};

inline NullStream& null_stream() {
  static NullStream s;
  return s;
}

// LogSeverity is `INFO`, `WARNING`, `ERROR`, `FATAL` — passed as the
// macro arg. We accept anything here.
inline NullStream& LogStream(int /*severity*/) { return null_stream(); }

}  // namespace base
}  // namespace android

#define LOG(severity) ::android::base::null_stream()
#define PLOG(severity) ::android::base::null_stream()
#define LOG_IF(severity, cond) if (false) ::android::base::null_stream()

#define CHECK(condition) if (!(condition)) ::android::base::null_stream()
#define CHECK_EQ(a, b) if (!((a) == (b))) ::android::base::null_stream()
#define CHECK_NE(a, b) if (!((a) != (b))) ::android::base::null_stream()
#define CHECK_GT(a, b) if (!((a) > (b))) ::android::base::null_stream()
#define CHECK_GE(a, b) if (!((a) >= (b))) ::android::base::null_stream()
#define CHECK_LT(a, b) if (!((a) < (b))) ::android::base::null_stream()
#define CHECK_LE(a, b) if (!((a) <= (b))) ::android::base::null_stream()
#define DCHECK(condition) CHECK(condition)
#define DCHECK_EQ(a, b) CHECK_EQ(a, b)
#define DCHECK_NE(a, b) CHECK_NE(a, b)
#define DCHECK_LE(a, b) CHECK_LE(a, b)
#define DCHECK_GE(a, b) CHECK_GE(a, b)
#define DCHECK_LT(a, b) CHECK_LT(a, b)
#define DCHECK_GT(a, b) CHECK_GT(a, b)

#define INFO 0
#define WARNING 1
#define ERROR 2
#define FATAL 3
