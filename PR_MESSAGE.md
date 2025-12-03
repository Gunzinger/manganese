# Add Windows Support, Improve CI/CD, and Enhance DRAM Testing

## Summary

This PR adds comprehensive Windows support, significantly improves the GitHub Actions CI/CD pipeline, implements semantic releases, and adds targeted DRAM-specific test patterns for better fault detection on DDR4/DDR5 platforms.

## Key Changes

### 🪟 Windows Support
- **Cross-platform compatibility**: Added Windows build support using `mman-win32` submodule
- **Platform abstraction layer**: Created `platform.h` with cross-platform abstractions for:
  - Memory mapping (`mmap`/`mman-win32`)
  - Memory locking (`mlock`/`VirtualLock`)
  - System information (`sysinfo`/Windows API)
  - Page size detection
- **Windows Makefile**: Added `Makefile.windows` for MinGW-w64/MSYS2 builds
- **Hardware detection**: Updated `hardware.c` with Windows-specific CPU count and RAM speed detection

### 🚀 CI/CD Improvements
- **Optimized builds**: Added OpenBLAS caching to reduce build times from ~30+ minutes to seconds on cache hits
- **Optional OpenBLAS**: Made OpenBLAS optional - builds work with or without it (SGEMM test skipped if unavailable)
- **Parallel builds**: Enabled parallel OpenBLAS compilation with `-j$(nproc)`
- **Fallback logic**: Added graceful fallback if OpenBLAS build fails
- **Error handling**: Improved error checking for mman-win32 builds

### 📦 Semantic Releases
- **Automatic releases**: Implemented semantic versioning with automatic GitHub Releases
- **Multi-platform binaries**: Builds and uploads both Linux and Windows binaries
- **Release automation**: Creates releases automatically on version tags (`v*.*.*`)
- **Binary artifacts**: Strips binaries and uploads to GitHub Releases with installation instructions

### 🧪 Enhanced DRAM Testing
Added 6 new DRAM-specific test patterns designed to detect common faults on DDR4/DDR5 platforms:

1. **Walking-1**: Single 1-bit walks through all positions (detects stuck-at faults, coupling faults)
2. **Walking-0**: Single 0-bit walks through all positions (detects stuck-at-1 faults)
3. **Checkerboard**: Alternating 0xAA/0x55 patterns (detects adjacent cell coupling)
4. **Address Line Test**: Enhanced address decoding tests (detects decoder faults, stuck address lines, bridging faults)
5. **Anti-Patterns**: Inverse pattern testing with 34 different patterns (detects pattern sensitivity)
6. **Inverse Data Patterns**: Byte/word/dword level inversions (detects data-dependent faults)

### 🔧 Build System Improvements
- **Conditional compilation**: Added `HAVE_OPENBLAS` define for optional OpenBLAS support
- **Submodule management**: Added `mman-win32` as a Git submodule
- **Dependency handling**: Improved Makefile dependency management with automatic builds
- **Error messages**: Better error reporting throughout the build process

## Files Changed

- **New files**:
  - `platform.h` - Cross-platform abstraction layer
  - `Makefile.windows` - Windows build configuration
  - `.github/workflows/release.yml` - Semantic release automation

- **Modified files**:
  - `manganese.c` - Added platform-specific includes
  - `hardware.c` - Windows compatibility for CPU/RAM detection
  - `tests-256.c` - Added DRAM test patterns, conditional OpenBLAS
  - `tests-512.c` - Added DRAM test patterns, conditional OpenBLAS
  - `Makefile` - Made OpenBLAS optional, improved caching
  - `.github/workflows/build.yml` - Optimized with caching, Windows builds
  - `.gitmodules` - Added mman-win32 submodule
  - `README.md` - Comprehensive usage documentation

## Testing

- ✅ Linux builds work with and without OpenBLAS
- ✅ Windows builds work with MSYS2/MinGW-w64
- ✅ All DRAM test patterns compile and run correctly
- ✅ GitHub Actions workflows tested and optimized
- ✅ Semantic releases tested with version tags

## Breaking Changes

None - all changes are backward compatible.

## Migration Guide

### For Users
No migration needed. Existing Linux builds continue to work as before.

### For Developers
To build on Windows:
```bash
# Install MSYS2 and MinGW-w64
pacman -S mingw-w64-x86_64-gcc mingw-w64-x86_64-openmp make

# Clone with submodules
git clone --recursive https://github.com/Gunzinger/manganese.git
cd manganese

# Build
make -f Makefile.windows
```

## Future Work

- [ ] Add AVX-512 support for Windows builds
- [ ] Consider CMake for better cross-platform build management
- [ ] Add more DRAM test patterns based on feedback
- [ ] Optimize OpenBLAS build further with pre-built binaries

## Related Issues

Fixes issues related to:
- Long CI/CD build times
- Missing Windows support
- Lack of semantic release automation
- Limited DRAM fault detection patterns

## Checklist

- [x] Code compiles on Linux (with and without OpenBLAS)
- [x] Code compiles on Windows (MSYS2/MinGW-w64)
- [x] All tests pass
- [x] Documentation updated
- [x] GitHub Actions workflows tested
- [x] Semantic releases tested
- [x] No breaking changes introduced


