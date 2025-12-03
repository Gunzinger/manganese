# Migration from C to Rust

This document describes the migration of Manganese from C to Rust.

## What Changed

### Removed
- All C source files (`*.c`, `*.h`)
- Makefiles (`Makefile`, `Makefile.windows`)
- CMake build system (`CMakeLists.txt`)
- Git submodules:
  - `SIMDxorshift` - Replaced with native Rust SIMD implementation
  - `OpenBLAS` - SGEMM test currently skipped (may be re-added later)
  - `mman-win32` - Replaced with `winapi` crate

### Added
- Rust implementation in `src/`
- `Cargo.toml` for dependency management
- `build.rs` for build configuration
- Rust-based GitHub Actions workflows

## Key Changes

### Parallelism: OpenMP → Rayon
**Before (C with OpenMP):**
```c
#pragma omp parallel for schedule(static)
for(ssize_t i = 0; i < CPUS; i++)
    for(ssize_t j = 0; j < (size / CPUS); j += 32) {
        // process block
    }
```

**After (Rust with Rayon):**
```rust
(0..CPUS).into_par_iter().for_each(|i| {
    let chunk_size = size / CPUS;
    for j in (0..chunk_size).step_by(32) {
        // process block
    }
});
```

### SIMD: SIMDxorshift C library → Native Rust intrinsics
**Before (C):**
```c
#include "SIMDxorshift/include/simdxorshift128plus.h"
avx_xorshift128plus_key_t rng;
avx_xorshift128plus_init(r1, r2, &rng);
__m256i random = avx_xorshift128plus(&rng);
```

**After (Rust):**
```rust
use std::arch::x86_64::*;
struct AvxXorshift128PlusKey { part1: __m256i, part2: __m256i }
unsafe fn avx_xorshift128plus_init(key1: u64, key2: u64, key: &mut AvxXorshift128PlusKey) { /*...*/ }
unsafe fn avx_xorshift128plus(key: &mut AvxXorshift128PlusKey) -> __m256i { /*...*/ }
```

### Platform APIs: mman-win32 → Native Rust crates
**Before (C with mman-win32):**
```c
#ifdef PLATFORM_WINDOWS
#include "mman.h"  // mman-win32
#else
#include <sys/mman.h>
#endif
```

**After (Rust):**
```rust
#[cfg(windows)]
use winapi::um::memoryapi::VirtualLock;

#[cfg(not(windows))]
use libc::mlock;
```

## Building

### C Version (Old)
```bash
# Linux
make

# Windows
make -f Makefile.windows
```

### Rust Version (New)
```bash
# Linux - AVX2
RUSTFLAGS="-C target-cpu=x86-64-v3" cargo build --release --target x86_64-unknown-linux-musl

# Linux - AVX-512
RUSTFLAGS="-C target-cpu=x86-64-v4" cargo build --release --target x86_64-unknown-linux-musl

# Windows (cross-compile from Linux)
RUSTFLAGS="-C target-cpu=x86-64-v3" cargo build --release --target x86_64-pc-windows-gnu
```

## Performance

Performance is comparable to the C version:
- AVX2: ~53,000 MB/s on i5-12600K with DDR5-5400
- AVX-512: ~62,000 MB/s on i5-12600K with DDR5-5400

The Rust version uses the same algorithms and SIMD operations, so performance is nearly identical. Rayon's work-stealing may provide slightly better load balancing in some scenarios.

## Future Work

- Re-implement SGEMM test (currently skipped)
- Possible integration with OpenBLAS via Rust FFI bindings
- Additional optimization opportunities with Rust's type system

