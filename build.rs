fn main() {
    // Note: We use runtime feature detection instead of compile-time
    // This allows the binary to work on CPUs without AVX2/AVX512
    // The code uses cfg(target_feature) checks which are evaluated at runtime
}

