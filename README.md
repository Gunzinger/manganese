# Manganese

A high-performance memory tester written in Rust. Manganese utilizes modern CPU features to
run significantly faster than traditional memory tests, letting you run more
passes in less time.

## Features

- **Rayon parallelism**: Automatic work-stealing parallelism across all CPU cores
- **AVX2 and AVX-512**: Runtime feature detection with optimized code paths
- **Cross-platform**: Native Linux and Windows binaries as static executables without external dependencies
- **Non-temporal stores**: Bypasses CPU cache for maximum memory bandwidth
- **Comprehensive DRAM testing**: Walking-1, walking-0, checkerboard, anti-patterns, and inverse data patterns

## Performance

TODO benchmarks of rust version

#### previous C version:
```text 

All benchmarks conducted on an i5 12600k paired with dual-channel DDR5 at 5400MT/s.

| ISA     | Threading | Avg. Bandwidth |
| ------- | --------- | -------------- |
| AVX2    | 1c/1T     | 5640MB/s       |
| AVX2    | 1C/1T     | 8600MB/s       |
| AVX-512 | 1C/1T     | 9400MB/s       |
| AVX2    | 6C+4c/16T | 53000MB/s      |
| AVX-512 | 6C/12T    | 62000MB/s      |
```

## Requirements

- A CPU with AVX2 (x86-64-v3, 2013+) or AVX-512 (x86-64-v4, 2017+)
- Linux 5.x+ or Windows 10/11 (64-bit)
- Sufficient RAM to lock memory for testing
- Administrator/root privileges for memory locking (semi-optional)

## Installation & Usage

### Quick Start (Pre-built Binaries)

Download the latest release from the [Releases page](https://github.com/Gunzinger/manganese/releases):

**Linux** (static MUSL binaries):
- `manganese-*-avx256` - AVX2 compatible (x86-64-v3)
- `manganese-*-avx512` - AVX-512 optimized (x86-64-v4)

**Windows**:
- `manganese-*-avx256.exe` - AVX2 compatible (x86-64-v3)
- `manganese-*-avx512.exe` - AVX-512 optimized (x86-64-v4)

Then run:
```bash
chmod +x manganese-*
# Linux - AVX2 compatible
sudo ./manganese-*-avx256 10%

# Linux - AVX-512 optimized
sudo ./manganese-*-avx512 10%

# Windows (Run as Administrator)
manganese-*-avx256.exe 10%
# or 
manganese-*-avx512.exe 10%

```

### Building from Source

#### Prerequisites

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add targets for cross-compilation
rustup target add x86_64-unknown-linux-musl x86_64-pc-windows-gnu
```

#### Build Commands

```bash
# Clone repository
git clone https://github.com/Gunzinger/manganese.git
cd manganese

# Build AVX2 version (x86-64-v3, recommended for compatibility)
RUSTFLAGS="-C target-cpu=x86-64-v3 -C target-feature=+avx2,+fma" cargo build --release --target x86_64-unknown-linux-musl

# Build AVX-512 version (x86-64-v4, optimized for newer CPUs)
RUSTFLAGS="-C target-cpu=x86-64-v4 -C target-feature=+avx2,+avx512f,+avx512bw" cargo build --release --target x86_64-unknown-linux-musl

# Run tests
sudo ./target/x86_64-unknown-linux-musl/release/manganese 10%

# Test specific CPU cores with taskset
sudo taskset -c 0-7 ./target/x86_64-unknown-linux-musl/release/manganese 10%
```

#### Windows Cross-Compilation (from Linux)

```bash
# Install MinGW toolchain (Ubuntu/Debian)
sudo apt-get install gcc-mingw-w64-x86-64-win32

# Build for Windows
RUSTFLAGS="-C target-cpu=x86-64-v3 -C target-feature=+avx2,+fma" cargo build --release --target x86_64-pc-windows-gnu
# or for AVX512
RUSTFLAGS="-C target-cpu=x86-64-v4 -C target-feature=+avx2,+avx512f,+avx512bw" cargo build --release --target x86_64-pc-windows-gnu


# Binary will be at: target/x86_64-pc-windows-gnu/release/manganese.exe
```

## Test Patterns

Manganese includes comprehensive DRAM test patterns designed to detect common faults:

### Standard Tests
- **Basic Tests**: Common data patterns (0x00, 0xFF, 0x55, 0xAA, etc.)
- **Random Inversions**: Random data patterns and their inverses
- **Moving Inversions**: Bit-shifted patterns at various granularities
- **Moving Saturations**: Saturation patterns with transitions

```
(broken) - **March Tests**: Sequential memory access patterns
(broken) - **Addressing**: Address-as-data patterns for decoder testing
(broken/unimplemented) - **SGEMM**: Matrix multiplication stress test (requires OpenBLAS)
```

### DRAM-Specific Tests
- **Walking-1**: Single 1-bit walks through all positions (detects stuck-at faults, coupling faults)
- **Walking-0**: Single 0-bit walks through all positions (detects stuck-at-1 faults)
- **Checkerboard**: Alternating 0xAA/0x55 patterns (detects adjacent cell coupling)
- **Anti-Patterns**: Inverse pattern testing (detects pattern sensitivity)
- **Inverse Data Patterns**: Byte/word/dword level inversions (detects data-dependent faults)
``` (broken) **Address Line Test**: Enhanced address decoding tests (detects decoder faults, stuck address lines)```

These patterns are specifically designed to trigger common faults on DDR4/DDR5 platforms
 and weaknesses in memory controllers (IMC) and memory ICs.

## Usage Examples

### Basic Memory Test
```bash
# Test 10% of total RAM
./manganese 10%

# Test 50% of total RAM
./manganese 50%

# Test all available RAM (not recommended unless you know what you're doing [it is impossible, lol])
sudo ./manganese 100%
```

### Advanced Usage
```bash
# Test specific CPU cores (Linux)
sudo taskset -c 0-3 ./manganese 25%  # Use cores 0-3
```

### Example Output (from c version)
Tested at 80mV below the threshold of stability

![Console output for detected instability](run-example.png)

The output shows:
- Number of threads/cores used
- Memory speed (if detected)
- Amount of locked memory
- Instruction set detected (AVX2 or AVX-512)
- Test progress and results
- Error counts (if any errors detected)
- Average bandwidth achieved

## Disclaimer

Do not mount important filesystems with a potentially unstable computer. Only
use this program and other stability tests from a Live CD or a separate
operating system on which all connected devices and mounted filesystems are
disposable.

Tests in this program may not detect certain memory faults. This program cannot
test memory reserved by the Linux kernel and other running programs. If this
program gives you the all-clear and then your million-dollar Bitcoin wallet
becomes corrupt, that's on you. If this program convinces you to throw away a
perfectly good memory module, that's also on you. See the LICENSE file, as well
as the LICENSE file within the SIMDxorshift directory for more information.

## Building for Releases

Releases are automatically built via GitHub Actions when version tags are pushed:
```bash
git tag v1.0.0
git push --tags
```

This triggers automatic builds for both Linux and Windows, with binaries uploaded to GitHub Releases.

## Architecture Support

Manganese binaries are built for two CPU microarchitecture levels:

- **x86-64-v3 (AVX2)**: Compatible with Intel Haswell (2013+), AMD Excavator (2015+) and newer
  - Features: AVX2, BMI1, BMI2, F16C, FMA, LZCNT, MOVBE, OSXSAVE
- **x86-64-v4 (AVX-512)**: Optimized for Intel Skylake-X (2017+), AMD Zen 4 (2022+) and newer
  - Features: AVX-512F, AVX-512BW, AVX-512CD, AVX-512DQ, AVX-512VL

See: https://en.wikipedia.org/wiki/AVX-512#CPUs_with_AVX-512

Choose the AVX2 version for maximum compatibility, or the AVX-512 version for ~15-20% better performance on supported CPUs.

## Troubleshooting

### Linux: "can't lock any memory"
Increase the memlock ulimit:
```bash
ulimit -l unlimited
# Or edit /etc/security/limits.conf
```

Or run as root:
```bash
sudo ./manganese-* 10%
```

### Windows: Memory locking fails
Run as Administrator to allow memory locking:
```bash
# Right-click Command Prompt/PowerShell -> Run as Administrator
manganese-*.exe 10%
```

## Implementation Notes

This is a complete Rust rewrite of the original C implementation:
- **Parallelism**: Uses Rayon instead of OpenMP
- **SIMD**: Native Rust intrinsics (`std::arch::x86_64`) instead of SIMDxorshift C library
- **Platform APIs**: Native `libc` and `winapi` crates instead of mman-win32
- **Memory Safety**: Unsafe blocks only where necessary (SIMD operations, raw memory access)

## Credits

- [Original C implementation](https://github.com/AdamNiederer/manganese) concept and test patterns by [Adam Niederer](https://github.com/AdamNiederer)
- Inspired by Daniel Lemire's SIMDxorshift for RNG algorithm design
