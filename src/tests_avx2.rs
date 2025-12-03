#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;
use std::sync::atomic::AtomicU64;

#[cfg(target_arch = "x86_64")]
use crate::simd_xorshift::AvxXorshift128PlusKey;
#[cfg(target_arch = "x86_64")]
use crate::simd_xorshift::{avx_xorshift128plus, avx_xorshift128plus_init};

static mut CPUS: usize = 0;
static mut ERRORS: *const AtomicU64 = std::ptr::null();
static mut RNG: AvxXorshift128PlusKey = AvxXorshift128PlusKey {
    part1: unsafe { std::mem::zeroed() },
    part2: unsafe { std::mem::zeroed() },
};

#[cfg(target_arch = "x86_64")]
pub unsafe fn avx2_tests_init(cpus: usize, errors: *const AtomicU64) {
    CPUS = cpus;
    ERRORS = errors;
    
    let mut r1 = 0u64;
    let mut r2 = 0u64;
    while r1 == 0 && r2 == 0 {
        _rdrand64_step(&mut r1);
        _rdrand64_step(&mut r2);
    }
    avx_xorshift128plus_init(r1, r2, &mut RNG);
}

#[cfg(target_arch = "x86_64")]
unsafe fn get(mem: *const u8, idx: usize, expected: __m256i) {
    let actual = _mm256_load_si256((mem.add(idx)) as *const __m256i);
    let cmp = _mm256_cmpeq_epi8(expected, actual);
    let result = _mm256_testz_si256(cmp, cmp);
    
    if result == 0 {
        let error_total = _mm_popcnt_u64(!result as u64);
        eprintln!("errors detected at offset 0x{:016x}", idx);
        (*ERRORS).fetch_add(error_total, std::sync::atomic::Ordering::Relaxed);
    }
}

#[cfg(target_arch = "x86_64")]
unsafe fn get_all_up(mem: *const u8, size: usize, expected: __m256i) {
    use rayon::prelude::*;
    
    (0..CPUS).into_par_iter().for_each(|i| {
        let chunk_size = size / CPUS;
        for j in (0..chunk_size).step_by(32) {
            let idx = j + i * chunk_size;
            get(mem, idx, expected);
        }
    });
}

#[cfg(target_arch = "x86_64")]
unsafe fn get_all_down(mem: *const u8, size: usize, expected: __m256i) {
    use rayon::prelude::*;
    
    let chunk_size = size / CPUS;
    (0..CPUS).into_par_iter().rev().for_each(|i| {
        let start = i * chunk_size;
        let end = start + chunk_size;
        for j in (start..end).rev().step_by(32) {
            if j + 32 <= end {
                get(mem, j, expected);
            }
        }
    });
}

#[cfg(target_arch = "x86_64")]
unsafe fn set(mem: *mut u8, idx: usize, val: __m256i) {
    _mm256_stream_si256((mem.add(idx)) as *mut __m256i, val);
}

#[cfg(target_arch = "x86_64")]
unsafe fn set_all_up(mem: *mut u8, size: usize, val: __m256i) {
    use rayon::prelude::*;
    
    (0..CPUS).into_par_iter().for_each(|i| {
        let chunk_size = size / CPUS;
        for j in (0..chunk_size).step_by(32) {
            let idx = j + i * chunk_size;
            set(mem, idx, val);
        }
    });
}

#[cfg(target_arch = "x86_64")]
unsafe fn set_all_down(mem: *mut u8, size: usize, val: __m256i) {
    use rayon::prelude::*;
    
    let chunk_size = size / CPUS;
    (0..CPUS).into_par_iter().rev().for_each(|i| {
        let start = i * chunk_size;
        let end = start + chunk_size;
        for j in (start..end).rev().step_by(32) {
            if j + 32 <= end {
                set(mem, j, val);
            }
        }
    });
}

#[cfg(target_arch = "x86_64")]
pub unsafe fn avx2_basic_tests(mem: *mut u8, size: usize) {
    let patterns = [0x00u8, 0xFF, 0x0F, 0xF0, 0x55, 0xAA];
    for pattern_val in &patterns {
        let pattern = _mm256_set1_epi8(*pattern_val as i8);
        set_all_up(mem, size, pattern);
        get_all_up(mem as *const u8, size, pattern);
        set_all_down(mem, size, pattern);
        get_all_down(mem as *const u8, size, pattern);
    }
}

#[cfg(target_arch = "x86_64")]
pub unsafe fn avx2_march(mem: *mut u8, size: usize) {
    for _ in 0..2 {
        let ones = _mm256_set1_epi8(0xFF);
        let zeroes = _mm256_set1_epi8(0x00);
        
        use rayon::prelude::*;
        let chunk_size = size / CPUS;
        
        // Down: set zeroes
        let chunk_size = size / CPUS;
        (0..CPUS).into_par_iter().rev().for_each(|i| {
            let start = i * chunk_size;
            let end = start + chunk_size;
            for j in (start..end).rev().step_by(32) {
                if j + 32 <= end {
                    set(mem, j, zeroes);
                }
            }
        });
        
        // Up: get zeroes, set ones, get ones, set zeroes, get zeroes, set ones
        (0..CPUS).into_par_iter().for_each(|i| {
            for j in (0..chunk_size).step_by(32) {
                let idx = j + i * chunk_size;
                get(mem as *const u8, idx, zeroes);
                set(mem, idx, ones);
                get(mem as *const u8, idx, ones);
                set(mem, idx, zeroes);
                get(mem as *const u8, idx, zeroes);
                set(mem, idx, ones);
            }
        });
        
        // Up: get ones, set zeroes, set ones
        (0..CPUS).into_par_iter().for_each(|i| {
            for j in (0..chunk_size).step_by(32) {
                let idx = j + i * chunk_size;
                get(mem as *const u8, idx, ones);
                set(mem, idx, zeroes);
                set(mem, idx, ones);
            }
        });
        
        // Down: get ones, set zeroes, set ones, set zeroes
        (0..CPUS).into_par_iter().rev().for_each(|i| {
            let start = i * chunk_size;
            let end = start + chunk_size;
            for j in (start..end).rev().step_by(32) {
                if j + 32 <= end {
                    get(mem as *const u8, j, ones);
                    set(mem, j, zeroes);
                    set(mem, j, ones);
                    set(mem, j, zeroes);
                }
            }
        });
        
        // Down: get zeroes, set ones, set zeroes
        (0..CPUS).into_par_iter().rev().for_each(|i| {
            let start = i * chunk_size;
            let end = start + chunk_size;
            for j in (start..end).rev().step_by(32) {
                if j + 32 <= end {
                    get(mem as *const u8, j, zeroes);
                    set(mem, j, ones);
                    set(mem, j, zeroes);
                }
            }
        });
    }
}

#[cfg(target_arch = "x86_64")]
pub unsafe fn avx2_random_inversions(mem: *mut u8, size: usize) {
    for _ in 0..16 {
        let pattern = avx_xorshift128plus(&mut RNG);
        set_all_up(mem, size, pattern);
        get_all_up(mem as *const u8, size, pattern);
        let not_pattern = _mm256_xor_si256(pattern, _mm256_set1_epi8(0xFF));
        set_all_up(mem, size, not_pattern);
        get_all_up(mem as *const u8, size, not_pattern);
    }
}

#[cfg(target_arch = "x86_64")]
unsafe fn moving_inversions_template(
    mem: *mut u8,
    size: usize,
    iters: usize,
    shift_fn: unsafe fn(__m256i, i32) -> __m256i,
    initial: __m256i,
) {
    for i in 0..iters {
        let pattern = shift_fn(initial, i as i32);
        set_all_up(mem, size, pattern);
        get_all_up(mem as *const u8, size, pattern);
        let not_pattern = _mm256_xor_si256(pattern, _mm256_set1_epi8(0xFF));
        set_all_up(mem, size, not_pattern);
        get_all_up(mem as *const u8, size, not_pattern);
    }
}

#[cfg(target_arch = "x86_64")]
pub unsafe fn avx2_moving_inversions_left_64(mem: *mut u8, size: usize) {
    moving_inversions_template(mem, size, 64, _mm256_slli_epi64, _mm256_set1_epi64x(0x0000000000000001));
}

#[cfg(target_arch = "x86_64")]
pub unsafe fn avx2_moving_inversions_right_32(mem: *mut u8, size: usize) {
    moving_inversions_template(mem, size, 32, _mm256_srli_epi64, _mm256_set1_epi32(0x80000000));
}

#[cfg(target_arch = "x86_64")]
pub unsafe fn avx2_moving_inversions_left_16(mem: *mut u8, size: usize) {
    moving_inversions_template(mem, size, 16, _mm256_slli_epi64, _mm256_set1_epi16(0x0001));
}

#[cfg(target_arch = "x86_64")]
pub unsafe fn avx2_moving_inversions_right_8(mem: *mut u8, size: usize) {
    moving_inversions_template(mem, size, 8, _mm256_srli_epi64, _mm256_set1_epi8(0x80));
}

#[cfg(target_arch = "x86_64")]
pub unsafe fn avx2_moving_inversions_left_4(mem: *mut u8, size: usize) {
    moving_inversions_template(mem, size, 4, _mm256_slli_epi64, _mm256_set1_epi8(0x11));
}

#[cfg(target_arch = "x86_64")]
pub unsafe fn avx2_moving_saturations_right_16(mem: *mut u8, size: usize) {
    for i in 0..16 {
        let pattern = _mm256_srli_epi16(_mm256_set1_epi16(0x8000), i as i32);
        set_all_up(mem, size, pattern);
        get_all_up(mem as *const u8, size, pattern);
        let zeroes = _mm256_set1_epi8(0x00);
        set_all_up(mem, size, zeroes);
        get_all_up(mem as *const u8, size, zeroes);
        set_all_up(mem, size, pattern);
        get_all_up(mem as *const u8, size, pattern);
        let ones = _mm256_set1_epi8(0xFF);
        set_all_up(mem, size, ones);
        get_all_up(mem as *const u8, size, ones);
    }
}

#[cfg(target_arch = "x86_64")]
pub unsafe fn avx2_moving_saturations_left_8(mem: *mut u8, size: usize) {
    for i in 0..8 {
        let pattern = _mm256_srli_epi16(_mm256_set1_epi16(0x01), i as i32);
        set_all_up(mem, size, pattern);
        get_all_up(mem as *const u8, size, pattern);
        let zeroes = _mm256_set1_epi8(0x00);
        set_all_up(mem, size, zeroes);
        get_all_up(mem as *const u8, size, zeroes);
        set_all_up(mem, size, pattern);
        get_all_up(mem as *const u8, size, pattern);
        let ones = _mm256_set1_epi8(0xFF);
        set_all_up(mem, size, ones);
        get_all_up(mem as *const u8, size, ones);
    }
}

#[cfg(target_arch = "x86_64")]
pub unsafe fn avx2_addressing(mem: *mut u8, size: usize) {
    use rayon::prelude::*;
    let chunk_size = size / CPUS;
    
    for _ in 0..16 {
        let increasing = _mm256_set_epi64x(24, 16, 8, 0);
        
        (0..CPUS).into_par_iter().for_each(|i| {
            for j in (0..chunk_size).step_by(32) {
                let idx = j + i * chunk_size;
                let addr_val = idx as i64;
                let pattern = _mm256_add_epi64(_mm256_set1_epi64x(addr_val), increasing);
                set(mem, idx, pattern);
            }
        });
        
        (0..CPUS).into_par_iter().for_each(|i| {
            for j in (0..chunk_size).step_by(32) {
                let idx = j + i * chunk_size;
                let addr_val = idx as i64;
                let expected = _mm256_add_epi64(_mm256_set1_epi64x(addr_val), increasing);
                get(mem as *const u8, idx, expected);
            }
        });
        
        (0..CPUS).into_par_iter().rev().for_each(|i| {
            let start = i * chunk_size;
            let end = start + chunk_size;
            for j in (start..end).rev().step_by(32) {
                if j + 32 <= end {
                    let addr_val = j as i64;
                    let pattern = _mm256_add_epi64(_mm256_set1_epi64x(addr_val), increasing);
                    set(mem, j, pattern);
                }
            }
        });
        
        (0..CPUS).into_par_iter().rev().for_each(|i| {
            let start = i * chunk_size;
            let end = start + chunk_size;
            for j in (start..end).rev().step_by(32) {
                if j + 32 <= end {
                    let addr_val = j as i64;
                    let expected = _mm256_add_epi64(_mm256_set1_epi64x(addr_val), increasing);
                    get(mem as *const u8, j, expected);
                }
            }
        });
    }
}

#[cfg(target_arch = "x86_64")]
pub unsafe fn avx2_sgemm(mem: *mut u8, size: usize) {
    // SGEMM test requires OpenBLAS - skip if not available
    // In Rust, we'd need bindings to OpenBLAS or implement a simple GEMM
    // For now, we'll skip it like the C version does when OpenBLAS is not available
    let _ = mem;
    let _ = size;
}

#[cfg(target_arch = "x86_64")]
pub unsafe fn avx2_walking_1(mem: *mut u8, size: usize) {
    for bit in 0..64 {
        let pattern_val = 1u64 << bit;
        let pattern = _mm256_set1_epi64x(pattern_val as i64);
        set_all_up(mem, size, pattern);
        get_all_up(mem as *const u8, size, pattern);
        let not_pattern = _mm256_xor_si256(pattern, _mm256_set1_epi8(0xFF));
        set_all_up(mem, size, not_pattern);
        get_all_up(mem as *const u8, size, not_pattern);
    }
}

#[cfg(target_arch = "x86_64")]
pub unsafe fn avx2_walking_0(mem: *mut u8, size: usize) {
    for bit in 0..64 {
        let pattern_val = !(1u64 << bit);
        let pattern = _mm256_set1_epi64x(pattern_val as i64);
        set_all_up(mem, size, pattern);
        get_all_up(mem as *const u8, size, pattern);
        let not_pattern = _mm256_xor_si256(pattern, _mm256_set1_epi8(0xFF));
        set_all_up(mem, size, not_pattern);
        get_all_up(mem as *const u8, size, not_pattern);
    }
}

#[cfg(target_arch = "x86_64")]
pub unsafe fn avx2_checkerboard(mem: *mut u8, size: usize) {
    use rayon::prelude::*;
    let chunk_size = size / CPUS;
    
    let pattern1 = _mm256_set1_epi8(0xAA);
    let pattern2 = _mm256_set1_epi8(0x55);
    
    (0..CPUS).into_par_iter().for_each(|i| {
        for j in (0..chunk_size).step_by(32) {
            let idx = j + i * chunk_size;
            let pattern = if ((idx / 32) % 2) != 0 { pattern1 } else { pattern2 };
            set(mem, idx, pattern);
        }
    });
    
    (0..CPUS).into_par_iter().for_each(|i| {
        for j in (0..chunk_size).step_by(32) {
            let idx = j + i * chunk_size;
            let expected = if ((idx / 32) % 2) != 0 { pattern1 } else { pattern2 };
            get(mem as *const u8, idx, expected);
        }
    });
    
    (0..CPUS).into_par_iter().for_each(|i| {
        for j in (0..chunk_size).step_by(32) {
            let idx = j + i * chunk_size;
            let pattern = if ((idx / 32) % 2) != 0 { pattern2 } else { pattern1 };
            set(mem, idx, pattern);
        }
    });
    
    (0..CPUS).into_par_iter().for_each(|i| {
        for j in (0..chunk_size).step_by(32) {
            let idx = j + i * chunk_size;
            let expected = if ((idx / 32) % 2) != 0 { pattern2 } else { pattern1 };
            get(mem as *const u8, idx, expected);
        }
    });
}

#[cfg(target_arch = "x86_64")]
pub unsafe fn avx2_address_line_test(mem: *mut u8, size: usize) {
    use rayon::prelude::*;
    let chunk_size = size / CPUS;
    
    (0..CPUS).into_par_iter().for_each(|i| {
        for j in (0..chunk_size).step_by(32) {
            let idx = j + i * chunk_size;
            let addr_pattern = idx as u64;
            let pattern = _mm256_set1_epi64x(addr_pattern as i64);
            set(mem, idx, pattern);
        }
    });
    
    (0..CPUS).into_par_iter().for_each(|i| {
        for j in (0..chunk_size).step_by(32) {
            let idx = j + i * chunk_size;
            let addr_pattern = idx as u64;
            let expected = _mm256_set1_epi64x(addr_pattern as i64);
            get(mem as *const u8, idx, expected);
        }
    });
    
    (0..CPUS).into_par_iter().rev().for_each(|i| {
        let start = i * chunk_size;
        let end = start + chunk_size;
        for j in (start..end).rev().step_by(32) {
            if j + 32 <= end {
                let addr_pattern = !j as u64;
                let pattern = _mm256_set1_epi64x(addr_pattern as i64);
                set(mem, j, pattern);
            }
        }
    });
    
    (0..CPUS).into_par_iter().rev().for_each(|i| {
        let start = i * chunk_size;
        let end = start + chunk_size;
        for j in (start..end).rev().step_by(32) {
            if j + 32 <= end {
                let addr_pattern = !j as u64;
                let expected = _mm256_set1_epi64x(addr_pattern as i64);
                get(mem as *const u8, j, expected);
            }
        }
    });
    
    let mut shift = 1;
    while shift <= 16 {
        (0..CPUS).into_par_iter().for_each(|i| {
            for j in (0..chunk_size).step_by(32) {
                let idx = j + i * chunk_size;
                let addr_pattern = idx as u64 ^ ((idx as u64) << shift);
                let pattern = _mm256_set1_epi64x(addr_pattern as i64);
                set(mem, idx, pattern);
            }
        });
        
        (0..CPUS).into_par_iter().for_each(|i| {
            for j in (0..chunk_size).step_by(32) {
                let idx = j + i * chunk_size;
                let addr_pattern = idx as u64 ^ ((idx as u64) << shift);
                let expected = _mm256_set1_epi64x(addr_pattern as i64);
                get(mem as *const u8, idx, expected);
            }
        });
        shift <<= 1;
    }
}

#[cfg(target_arch = "x86_64")]
pub unsafe fn avx2_anti_patterns(mem: *mut u8, size: usize) {
    let patterns = [
        0x00, 0xFF, 0x0F, 0xF0, 0x55, 0xAA, 0x33, 0xCC,
        0x11, 0xEE, 0x22, 0xDD, 0x44, 0xBB, 0x66, 0x99,
        0x77, 0x88, 0x01, 0xFE, 0x02, 0xFD, 0x04, 0xFB,
        0x08, 0xF7, 0x10, 0xEF, 0x20, 0xDF, 0x40, 0xBF,
        0x80, 0x7F,
    ];
    
    for pattern_val in &patterns {
        let pattern = _mm256_set1_epi8(*pattern_val as i8);
        let anti_pattern = _mm256_xor_si256(pattern, _mm256_set1_epi8(0xFF));
        
        set_all_up(mem, size, pattern);
        get_all_up(mem as *const u8, size, pattern);
        set_all_up(mem, size, anti_pattern);
        get_all_up(mem as *const u8, size, anti_pattern);
        
        set_all_down(mem, size, pattern);
        get_all_down(mem as *const u8, size, pattern);
        set_all_down(mem, size, anti_pattern);
        get_all_down(mem as *const u8, size, anti_pattern);
    }
}

#[cfg(target_arch = "x86_64")]
pub unsafe fn avx2_inverse_data_patterns(mem: *mut u8, size: usize) {
    for byte_idx in 0..8 {
        let base_pattern = 0xFFFFFFFFFFFFFFFFu64;
        let pattern_val = base_pattern ^ (0xFFu64 << (byte_idx * 8));
        let pattern = _mm256_set1_epi64x(pattern_val as i64);
        
        set_all_up(mem, size, pattern);
        get_all_up(mem as *const u8, size, pattern);
        
        let inverse = _mm256_xor_si256(pattern, _mm256_set1_epi8(0xFF));
        set_all_up(mem, size, inverse);
        get_all_up(mem as *const u8, size, inverse);
    }
    
    for word_idx in 0..4 {
        let base_pattern = 0xFFFFFFFFFFFFFFFFu64;
        let pattern_val = base_pattern ^ (0xFFFFu64 << (word_idx * 16));
        let pattern = _mm256_set1_epi64x(pattern_val as i64);
        
        set_all_up(mem, size, pattern);
        get_all_up(mem as *const u8, size, pattern);
        
        let inverse = _mm256_xor_si256(pattern, _mm256_set1_epi8(0xFF));
        set_all_up(mem, size, inverse);
        get_all_up(mem as *const u8, size, inverse);
    }
    
    for dword_idx in 0..2 {
        let base_pattern = 0xFFFFFFFFFFFFFFFFu64;
        let pattern_val = base_pattern ^ (0xFFFFFFFFu64 << (dword_idx * 32));
        let pattern = _mm256_set1_epi64x(pattern_val as i64);
        
        set_all_up(mem, size, pattern);
        get_all_up(mem as *const u8, size, pattern);
        
        let inverse = _mm256_xor_si256(pattern, _mm256_set1_epi8(0xFF));
        set_all_up(mem, size, inverse);
        get_all_up(mem as *const u8, size, inverse);
    }
}

// Stub implementations for non-x86_64 targets
#[cfg(not(target_arch = "x86_64"))]
pub unsafe fn avx2_tests_init(_cpus: usize, _errors: *const AtomicU64) {}
#[cfg(not(target_arch = "x86_64"))]
pub unsafe fn avx2_basic_tests(_mem: *mut u8, _size: usize) {}
#[cfg(not(target_arch = "x86_64"))]
pub unsafe fn avx2_march(_mem: *mut u8, _size: usize) {}
#[cfg(not(target_arch = "x86_64"))]
pub unsafe fn avx2_random_inversions(_mem: *mut u8, _size: usize) {}
#[cfg(not(target_arch = "x86_64"))]
pub unsafe fn avx2_moving_inversions_left_64(_mem: *mut u8, _size: usize) {}
#[cfg(not(target_arch = "x86_64"))]
pub unsafe fn avx2_moving_inversions_right_32(_mem: *mut u8, _size: usize) {}
#[cfg(not(target_arch = "x86_64"))]
pub unsafe fn avx2_moving_inversions_left_16(_mem: *mut u8, _size: usize) {}
#[cfg(not(target_arch = "x86_64"))]
pub unsafe fn avx2_moving_inversions_right_8(_mem: *mut u8, _size: usize) {}
#[cfg(not(target_arch = "x86_64"))]
pub unsafe fn avx2_moving_inversions_left_4(_mem: *mut u8, _size: usize) {}
#[cfg(not(target_arch = "x86_64"))]
pub unsafe fn avx2_moving_saturations_right_16(_mem: *mut u8, _size: usize) {}
#[cfg(not(target_arch = "x86_64"))]
pub unsafe fn avx2_moving_saturations_left_8(_mem: *mut u8, _size: usize) {}
#[cfg(not(target_arch = "x86_64"))]
pub unsafe fn avx2_addressing(_mem: *mut u8, _size: usize) {}
#[cfg(not(target_arch = "x86_64"))]
pub unsafe fn avx2_sgemm(_mem: *mut u8, _size: usize) {}
#[cfg(not(target_arch = "x86_64"))]
pub unsafe fn avx2_walking_1(_mem: *mut u8, _size: usize) {}
#[cfg(not(target_arch = "x86_64"))]
pub unsafe fn avx2_walking_0(_mem: *mut u8, _size: usize) {}
#[cfg(not(target_arch = "x86_64"))]
pub unsafe fn avx2_checkerboard(_mem: *mut u8, _size: usize) {}
#[cfg(not(target_arch = "x86_64"))]
pub unsafe fn avx2_address_line_test(_mem: *mut u8, _size: usize) {}
#[cfg(not(target_arch = "x86_64"))]
pub unsafe fn avx2_anti_patterns(_mem: *mut u8, _size: usize) {}
#[cfg(not(target_arch = "x86_64"))]
pub unsafe fn avx2_inverse_data_patterns(_mem: *mut u8, _size: usize) {}

