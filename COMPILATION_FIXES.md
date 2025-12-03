# Compilation Fixes Applied

This document describes the fixes applied to resolve the 29 compilation errors encountered during CI/CD builds.

## Issues Fixed

### 1. Module Declaration Errors (E0583)
**Problem:** `tests_avx2` and `tests_avx512` were declared as `pub mod` in `tests.rs`

**Fix:** Changed to private `mod` declarations and added `use` statements:
```rust
// Module declarations - files are in src/ directory
mod tests_avx2;
mod tests_avx512;

// Re-export init functions
use tests_avx2::avx2_tests_init;
use tests_avx512::avx512_tests_init;
```

### 2. Windows API Import Errors (E0432)
**Problem:** Incorrect winapi module paths for Windows functions

**Fixes:**
- `SYSTEM_INFO` and `GetSystemInfo` moved from `winbase` to `sysinfoapi`
- `MEMORYSTATUSEX_SIZE` replaced with `std::mem::size_of::<MEMORYSTATUSEX>()`
- `_aligned_malloc/_aligned_free` replaced with `HeapAlloc/HeapFree` from `heapapi`
- Added `heapapi` and `winnt` to Cargo.toml winapi features

### 3. SIMD Intrinsic Constant Parameters (E0435, E0308)
**Problem:** Shift intrinsics like `_mm256_slli_epi64` require compile-time constants, not runtime values

**Fix:** Used macro-based match expressions to generate compile-time constants:
```rust
macro_rules! do_shift {
    ($i:expr) => {{
        let pattern = _mm256_slli_epi64::<$i>(_mm256_set1_epi64x(0x0000000000000001));
        // ... test code ...
    }};
}

for i in 0..64 {
    match i {
        0 => do_shift!(0), 1 => do_shift!(1), 2 => do_shift!(2), 3 => do_shift!(3),
        // ... all 64 cases ...
        _ => {}
    }
}
```

Applied to:
- `avx2_moving_inversions_left_64/right_32/left_16/right_8/left_4`
- `avx2_moving_saturations_right_16/left_8`
- `avx512_moving_inversions_*`
- `avx512_moving_saturations_*`

### 4. Thread Safety with Raw Pointers (E0277)
**Problem:** Rayon's `Sync` bound not satisfied for `*const u8` and `*mut u8` captured in closures

**Fix:** Convert pointers to `usize` before closure, reconvert inside:
```rust
let mem_usize = mem as usize;  // Outside closure - usize is Sync
(0..CPUS).into_par_iter().for_each(|i| {
    let mem_ptr = mem_usize as *mut u8;  // Inside closure - recreate pointer
    // ... use mem_ptr ...
});
```

### 5. Function Pointer Call Syntax (E0599)
**Problem:** `test.run` tried to call field as method

**Fix:** Parenthesize field access:
```rust
// Before: test.run(mem_ptr, size);
// After:
(test.run)(mem_ptr, size);
```

### 6. Missing Unsafe Blocks (E0133)
**Problem:** Calls to unsafe functions without unsafe blocks

**Fix:** Wrapped init calls in unsafe blocks:
```rust
unsafe { avx512_tests_init(cpus, errors); }
unsafe { avx2_tests_init(cpus, errors); }
```

### 7. SYSTEM_INFO Initialization (E0599)
**Problem:** `SYSTEM_INFO::default()` doesn't exist

**Fix:** Use `mem::zeroed()`:
```rust
let mut sys_info: SYSTEM_INFO = unsafe { mem::zeroed() };
```

### 8. Wrong SIMD Intrinsic (E0425)
**Problem:** `_mm_popcnt_u64` doesn't exist in AVX2 code

**Fix:** Simplified error detection - just count errors as 1 per mismatch instead of popcount

### 9. Unused Warnings
**Fixed:**
- Removed unused imports: `std::arch::x86_64::*`, `std::is_x86_feature_detected`, `Test`
- Renamed unused variable: `s1` → `_s1`

### 10. AVX-512 Function Availability
**Problem:** AVX-512 functions gated behind `target_feature = "avx512f"` but imported unconditionally

**Fix:** Added `#[cfg]` guards to all AVX-512 `use` statements

### 11. Rust Version Requirement
**Problem:** Rayon 1.11 requires Rust 1.80+

**Fix:** 
- Set `rust-version = "1.80"` in Cargo.toml
- GitHub Actions uses `dtolnay/rust-toolchain@stable` which uses latest Rust

## Build Verification

The code now compiles successfully with:
```bash
RUSTFLAGS="-C target-cpu=x86-64-v3 -C target-feature=+avx2,+fma" \
  cargo build --release --target x86_64-unknown-linux-musl

RUSTFLAGS="-C target-cpu=x86-64-v4 -C target-feature=+avx2,+avx512f,+avx512bw" \
  cargo build --release --target x86_64-unknown-linux-musl
```

## CI/CD Configuration

Workflows now properly build with explicit target features:
- x86-64-v3: `-C target-feature=+avx2,+fma`
- x86-64-v4: `-C target-feature=+avx2,+avx512f,+avx512bw`

Both Linux (MUSL) and Windows (MinGW) targets are supported.

