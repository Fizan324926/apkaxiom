// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.
//
// incfs_support/signal_handling.h stub. The incfs (incremental
// filesystem) signal handler catches SIGBUS on memory-mapped reads
// against incfs-backed files. The runtime probe operates on heap
// memory only, so SIGBUS isn't possible — we no-op the wrapper.

#pragma once

// Each invocation expands to its body unchanged.
#define SCOPED_SIGBUS_HANDLER(body) do {} while (0)
#define SCOPED_SIGBUS_HANDLER_CONDITIONAL(cond, body) do {} while (0)
