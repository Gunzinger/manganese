# Manganese

A memory tester for the modern age. Manganese utilizes modern CPU features to
run significantly faster than traditional memory tests, letting you run more
passes in less time.

## Features

- Supports as many CPU cores as OpenMP does, and automatically uses all of them.
- Supports pure AVX2 and AVX-512 codepaths with runtime feature detection
- Uses non-temporal stores to bypass the CPU cache without additional performance penalties
- Prints per-error warnings, total error counts, and average bandwidth used after each loop
- **Cross-platform support**: Linux and Windows (via mman-win32)
- **Comprehensive DRAM testing**: Includes walking-1, walking-0, checkerboard, address line tests, anti-patterns, and inverse data patterns
- **Optional OpenBLAS**: Builds work with or without OpenBLAS (SGEMM test skipped if unavailable)

## Performance

All benchmarks conducted on an i5 12600k paired with dual-channel DDR5 at 5400MT/s.

| ISA     | Threading | Avg. Bandwidth |
| ------- | --------- | -------------- |
| AVX2    | 1c/1T     | 5640MB/s       |
| AVX2    | 1C/1T     | 8600MB/s       |
| AVX-512 | 1C/1T     | 9400MB/s       |
| AVX2    | 6C+4c/16T | 53000MB/s      |
| AVX-512 | 6C/12T    | 62000MB/s      |

## Requirements

### Linux
- Linux 5.x or newer
- C11 Compiler w/ GNU extensions (GCC 10+ recommended)
- OpenMP development libraries
- A CPU with AVX2 (slower) or AVX-512F and AVX-512BW (faster)

### Windows
- Windows 10/11 (64-bit)
- MinGW-w64 with GCC 10+ (via MSYS2 recommended)
- OpenMP support
- A CPU with AVX2 (AVX-512 support coming soon)

## Installation & Usage

### Quick Start (Pre-built Binaries)

Download the latest release from the [Releases page](https://github.com/Gunzinger/manganese/releases):
- **Linux**: Download `manganese`, make it executable: `chmod +x manganese`
- **Windows**: Download `manganese.exe`

Then run:
```bash
# Linux
./manganese 10%

# Windows
manganese.exe 10%
```

### Building from Source

#### Linux

```bash
# Install prerequisites (Ubuntu/Debian)
sudo apt install make gcc libomp-dev

# Clone repository and initialize submodules
git clone https://github.com/Gunzinger/manganese.git
cd manganese
git submodule update --init --recursive

# Build (OpenBLAS is optional but recommended)
make

# Run tests
./manganese 10%

# As memory must be locked, running might require ulimit adjustments or running as root
sudo ./manganese 10%

# Test specific CPU cores with taskset
sudo taskset -c 0-7 ./manganese 10%
```

**Note**: Building OpenBLAS takes a significant amount of time. The build will work without it, but the SGEMM test will be skipped. OpenBLAS builds are cached in CI/CD for faster subsequent builds.

#### Windows

```bash
# Install MSYS2 and MinGW-w64
# Download from: https://www.msys2.org/

# Install required packages in MSYS2 terminal
pacman -S mingw-w64-x86_64-gcc mingw-w64-x86_64-openmp make

# Clone repository and initialize submodules
git clone https://github.com/Gunzinger/manganese.git
cd manganese
git submodule update --init --recursive

# Build using Windows Makefile
make -f Makefile.windows

# Run tests (may require Administrator privileges for memory locking)
manganese.exe 10%
```

### Submodules

This project uses Git submodules for dependencies:
- **SIMDxorshift**: Fast AVX2/AVX-512 random number generator
- **OpenBLAS**: Optimized BLAS library (optional, for SGEMM test)
- **mman-win32**: Windows memory mapping compatibility layer

Always initialize submodules when cloning:
```bash
git submodule update --init --recursive
```

Or clone with submodules in one command:
```bash
git clone --recursive https://github.com/Gunzinger/manganese.git
```

## Test Patterns

Manganese includes comprehensive DRAM test patterns designed to detect common faults:

### Standard Tests
- **Basic Tests**: Common data patterns (0x00, 0xFF, 0x55, 0xAA, etc.)
- **March Tests**: Sequential memory access patterns
- **Random Inversions**: Random data patterns and their inverses
- **Moving Inversions**: Bit-shifted patterns at various granularities
- **Moving Saturations**: Saturation patterns with transitions
- **Addressing**: Address-as-data patterns for decoder testing
- **SGEMM**: Matrix multiplication stress test (requires OpenBLAS)

### DRAM-Specific Tests
- **Walking-1**: Single 1-bit walks through all positions (detects stuck-at faults, coupling faults)
- **Walking-0**: Single 0-bit walks through all positions (detects stuck-at-1 faults)
- **Checkerboard**: Alternating 0xAA/0x55 patterns (detects adjacent cell coupling)
- **Address Line Test**: Enhanced address decoding tests (detects decoder faults, stuck address lines)
- **Anti-Patterns**: Inverse pattern testing (detects pattern sensitivity)
- **Inverse Data Patterns**: Byte/word/dword level inversions (detects data-dependent faults)

These patterns are specifically designed to trigger common faults on DDR4/DDR5 platforms and weaknesses in memory controllers (IMC) and memory ICs.

## Usage Examples

### Basic Memory Test
```bash
# Test 10% of total RAM
./manganese 10%

# Test 50% of total RAM
./manganese 50%

# Test all available RAM (not recommended unless you know what you're doing)
sudo ./manganese 100%
```

### Advanced Usage
```bash
# Test specific CPU cores (Linux)
sudo taskset -c 0-3 ./manganese 25%  # Use cores 0-3

# Run with limited memory lock (if ulimit is set)
./manganese 5%
```

### Example Output
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

## Troubleshooting

### Linux: "can't lock any memory"
Increase the memlock ulimit:
```bash
ulimit -l unlimited
# Or edit /etc/security/limits.conf
```

Or run as root:
```bash
sudo ./manganese 10%
```

### Windows: Memory locking fails
Run as Administrator to allow memory locking:
```bash
# Right-click Command Prompt/PowerShell -> Run as Administrator
manganese.exe 10%
```

### Build fails without OpenBLAS
OpenBLAS is optional. The build will work without it, but the SGEMM test will be skipped. To build without OpenBLAS:
```bash
# Linux
make  # Will continue even if OpenBLAS build fails

# Windows
make -f Makefile.windows  # OpenBLAS not required
```

### Submodule issues
If submodules aren't initialized:
```bash
git submodule update --init --recursive
```

If submodules are out of date:
```bash
git submodule update --remote --recursive
```

## Contributing

Contributions are welcome! Please ensure:
- Code follows the existing style
- Tests pass on both Linux and Windows
- Submodules are properly initialized
- OpenBLAS is optional (don't break builds without it)

## Credits

- Thanks to Daniel Lemire for his AVX2 and AVX-512 random number generator (SIMDxorshift)
- Thanks to the OpenBLAS project for optimized BLAS routines
- Thanks to witwall for mman-win32 Windows compatibility layer
