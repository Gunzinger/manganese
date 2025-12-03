# Changelog

## Version 2.0.0 - Rust Rewrite (Unreleased)

### Major Changes
- **Complete Rust rewrite** of the entire codebase
- **Removed all C code** and dependencies
- **New build system** using Cargo instead of Make/CMake
- **Updated GitHub Actions** for Rust-based CI/CD

### Removed
- All C source files (`*.c`, `*.h`)
- Build systems: `Makefile`, `Makefile.windows`, `CMakeLists.txt`
- Git submodules:
  - `SIMDxorshift` - Replaced with native Rust SIMD implementation
  - `OpenBLAS` - SGEMM test temporarily removed (may be re-added)
  - `mman-win32` - Replaced with `winapi` crate for Windows support

### Added
- Rust source code in `src/`:
  - `main.rs` - Entry point and main loop
  - `platform.rs` - Cross-platform abstractions (memory locking, sysinfo)
  - `hardware.rs` - CPU detection and feature detection
  - `simd_xorshift.rs` - Native Rust SIMD RNG implementation
  - `tests.rs` - Test orchestration
  - `tests_avx2.rs` - AVX2 test implementations
  - `tests_avx512.rs` - AVX-512 test implementations
- `Cargo.toml` - Rust dependency management
- `build.rs` - Build configuration
- `.gitignore` - Rust-appropriate gitignore
- `MIGRATION.md` - Migration documentation
- Updated GitHub Actions workflows:
  - `build.yml` - CI for pull requests
  - `release.yml` - Release pipeline with multiple CPU targets

### Technical Changes
- **Parallelism**: OpenMP → Rayon (work-stealing parallelism)
- **SIMD**: SIMDxorshift C library → Native Rust `std::arch::x86_64` intrinsics
- **Platform APIs**: mman-win32 → Native `libc` (Linux) and `winapi` (Windows)
- **Build targets**:
  - x86-64-v3 (AVX2 compatible)
  - x86-64-v4 (AVX-512 optimized)
- **Platforms**:
  - Linux: Static MUSL binaries
  - Windows: MinGW cross-compiled binaries

### Performance
- Comparable performance to C version (~53-62 GB/s on i5-12600K)
- Same SIMD algorithms and memory access patterns
- Rayon provides efficient work-stealing parallelism

### Breaking Changes
- New binary naming convention: `manganese-v{VERSION}-{platform}-{cpu_target}`
- Different command-line invocation (no longer separate avx2/avx512 binaries)
- SGEMM test temporarily disabled (was optional in C version)

## Version 1.x - C Implementation

Previous versions were implemented in C with OpenMP parallelism.
See git history for details.

