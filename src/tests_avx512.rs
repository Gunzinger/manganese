#[cfg(all(target_arch = "x86_64", target_feature = "avx512f"))]
use std::arch::x86_64::*;
use std::sync::atomic::AtomicU64;

use crate::simd_xorshift::Avx512Xorshift128PlusKey;
use crate::simd_xorshift::{avx512_xorshift128plus, avx512_xorshift128plus_init};

static mut CPUS: usize = 0;
static mut ERRORS: *const AtomicU64 = std::ptr::null();
static mut RNG: Avx512Xorshift128PlusKey = Avx512Xorshift128PlusKey {
    part1: unsafe { std::mem::zeroed() },
    part2: unsafe { std::mem::zeroed() },
};

#[cfg(all(target_arch = "x86_64", target_feature = "avx512f"))]
pub unsafe fn avx512_tests_init(cpus: usize, errors: *const AtomicU64) {
    CPUS = cpus;
    ERRORS = errors;
    
    let mut r1 = 0u64;
    let mut r2 = 0u64;
    while r1 == 0 && r2 == 0 {
        _rdrand64_step(&mut r1);
        _rdrand64_step(&mut r2);
    }
    avx512_xorshift128plus_init(r1, r2, &mut RNG);
}

#[cfg(all(target_arch = "x86_64", target_feature = "avx512f"))]
unsafe fn get(mem: *const u8, idx: usize, expected: __m512i) {
    let actual = _mm512_load_si512((mem.add(idx)) as *const __m512i);
    _mm_lfence();
    let result = _mm512_cmp_epu8_mask(expected, actual, _MM_CMPINT_NE);
    
    if result != 0 {
        let error_total = _mm_popcnt_u64(result) as u64;
        eprintln!("{} errors detected at offset 0x{:016x} [error mask: 0x{:016x}]", error_total, idx, result);
        (*ERRORS).fetch_add(error_total, std::sync::atomic::Ordering::Relaxed);
    }
}

#[cfg(all(target_arch = "x86_64", target_feature = "avx512f"))]
unsafe fn get_all_up(mem: *const u8, size: usize, expected: __m512i) {
    use rayon::prelude::*;
    
    (0..CPUS).into_par_iter().for_each(|i| {
        let chunk_size = size / CPUS;
        for j in (0..chunk_size).step_by(64) {
            let idx = j + i * chunk_size;
            get(mem, idx, expected);
        }
    });
}

#[cfg(all(target_arch = "x86_64", target_feature = "avx512f"))]
unsafe fn get_all_down(mem: *const u8, size: usize, expected: __m512i) {
    use rayon::prelude::*;
    
    let chunk_size = size / CPUS;
    (0..CPUS).into_par_iter().rev().for_each(|i| {
        let start = i * chunk_size;
        let end = start + chunk_size;
        for j in (start..end).rev().step_by(64) {
            if j + 64 <= end {
                get(mem, j, expected);
            }
        }
    });
}

#[cfg(all(target_arch = "x86_64", target_feature = "avx512f"))]
unsafe fn set(mem: *mut u8, idx: usize, val: __m512i) {
    _mm512_stream_si512((mem.add(idx)) as *mut __m512i, val);
}

#[cfg(all(target_arch = "x86_64", target_feature = "avx512f"))]
unsafe fn set_all_up(mem: *mut u8, size: usize, val: __m512i) {
    use rayon::prelude::*;
    
    (0..CPUS).into_par_iter().for_each(|i| {
        let chunk_size = size / CPUS;
        for j in (0..chunk_size).step_by(64) {
            let idx = j + i * chunk_size;
            set(mem, idx, val);
        }
    });
}

#[cfg(all(target_arch = "x86_64", target_feature = "avx512f"))]
unsafe fn set_all_down(mem: *mut u8, size: usize, val: __m512i) {
    use rayon::prelude::*;
    
    let chunk_size = size / CPUS;
    (0..CPUS).into_par_iter().rev().for_each(|i| {
        let start = i * chunk_size;
        let end = start + chunk_size;
        for j in (start..end).rev().step_by(64) {
            if j + 64 <= end {
                set(mem, j, val);
            }
        }
    });
}

#[cfg(all(target_arch = "x86_64", target_feature = "avx512f"))]
pub unsafe fn avx512_basic_tests(mem: *mut u8, size: usize) {
    let patterns = [0x00u8, 0xFF, 0x0F, 0xF0, 0x55, 0xAA];
    for pattern_val in &patterns {
        let pattern = _mm512_set1_epi8(*pattern_val as i8);
        set_all_up(mem, size, pattern);
        get_all_up(mem as *const u8, size, pattern);
        set_all_down(mem, size, pattern);
        get_all_down(mem as *const u8, size, pattern);
    }
}

#[cfg(all(target_arch = "x86_64", target_feature = "avx512f"))]
pub unsafe fn avx512_march(mem: *mut u8, size: usize) {
    for _ in 0..2 {
        let ones = _mm512_set1_epi8(0xFF);
        let zeroes = _mm512_set1_epi8(0x00);
        
        use rayon::prelude::*;
        let chunk_size = size / CPUS;
        
        let chunk_size = size / CPUS;
        (0..CPUS).into_par_iter().rev().for_each(|i| {
            let start = i * chunk_size;
            let end = start + chunk_size;
            for j in (start..end).rev().step_by(64) {
                if j + 64 <= end {
                    set(mem, j, zeroes);
                }
            }
        });
        
        (0..CPUS).into_par_iter().for_each(|i| {
            for j in (0..chunk_size).step_by(64) {
                let idx = j + i * chunk_size;
                get(mem as *const u8, idx, zeroes);
                set(mem, idx, ones);
                get(mem as *const u8, idx, ones);
                set(mem, idx, zeroes);
                get(mem as *const u8, idx, zeroes);
                set(mem, idx, ones);
            }
        });
        
        (0..CPUS).into_par_iter().for_each(|i| {
            for j in (0..chunk_size).step_by(64) {
                let idx = j + i * chunk_size;
                get(mem as *const u8, idx, ones);
                set(mem, idx, zeroes);
                set(mem, idx, ones);
            }
        });
        
        (0..CPUS).into_par_iter().rev().for_each(|i| {
            let start = i * chunk_size;
            let end = start + chunk_size;
            for j in (start..end).rev().step_by(64) {
                if j + 64 <= end {
                    get(mem as *const u8, j, ones);
                    set(mem, j, zeroes);
                    set(mem, j, ones);
                    set(mem, j, zeroes);
                }
            }
        });
        
        (0..CPUS).into_par_iter().rev().for_each(|i| {
            let start = i * chunk_size;
            let end = start + chunk_size;
            for j in (start..end).rev().step_by(64) {
                if j + 64 <= end {
                    get(mem as *const u8, j, zeroes);
                    set(mem, j, ones);
                    set(mem, j, zeroes);
                }
            }
        });
    }
}

#[cfg(all(target_arch = "x86_64", target_feature = "avx512f"))]
pub unsafe fn avx512_random_inversions(mem: *mut u8, size: usize) {
    for _ in 0..16 {
        let pattern = avx512_xorshift128plus(&mut RNG);
        set_all_up(mem, size, pattern);
        get_all_up(mem as *const u8, size, pattern);
        let not_pattern = _mm512_xor_epi64(pattern, _mm512_set1_epi8(0xFF));
        set_all_up(mem, size, not_pattern);
        get_all_up(mem as *const u8, size, not_pattern);
    }
}

#[cfg(all(target_arch = "x86_64", target_feature = "avx512f"))]
unsafe fn moving_inversions_template(
    mem: *mut u8,
    size: usize,
    iters: usize,
    shift_fn: unsafe fn(__m512i, u32) -> __m512i,
    initial: __m512i,
) {
    for i in 0..iters {
        let pattern = shift_fn(initial, i as u32);
        set_all_up(mem, size, pattern);
        get_all_up(mem as *const u8, size, pattern);
        let not_pattern = _mm512_xor_epi64(pattern, _mm512_set1_epi8(0xFF));
        set_all_up(mem, size, not_pattern);
        get_all_up(mem as *const u8, size, not_pattern);
    }
}

#[cfg(all(target_arch = "x86_64", target_feature = "avx512f"))]
pub unsafe fn avx512_moving_inversions_left_64(mem: *mut u8, size: usize) {
    moving_inversions_template(mem, size, 64, _mm512_slli_epi64, _mm512_set1_epi64(0x0000000000000001));
}

#[cfg(all(target_arch = "x86_64", target_feature = "avx512f"))]
pub unsafe fn avx512_moving_inversions_right_32(mem: *mut u8, size: usize) {
    moving_inversions_template(mem, size, 32, _mm512_srli_epi64, _mm512_set1_epi32(0x80000000));
}

#[cfg(all(target_arch = "x86_64", target_feature = "avx512f"))]
pub unsafe fn avx512_moving_inversions_left_16(mem: *mut u8, size: usize) {
    moving_inversions_template(mem, size, 16, _mm512_slli_epi64, _mm512_set1_epi16(0x0001));
}

#[cfg(all(target_arch = "x86_64", target_feature = "avx512f"))]
pub unsafe fn avx512_moving_inversions_right_8(mem: *mut u8, size: usize) {
    moving_inversions_template(mem, size, 8, _mm512_srli_epi64, _mm512_set1_epi8(0x80));
}

#[cfg(all(target_arch = "x86_64", target_feature = "avx512f"))]
pub unsafe fn avx512_moving_inversions_left_4(mem: *mut u8, size: usize) {
    moving_inversions_template(mem, size, 4, _mm512_slli_epi64, _mm512_set1_epi8(0x11));
}

#[cfg(all(target_arch = "x86_64", target_feature = "avx512f"))]
pub unsafe fn avx512_moving_saturations_right_16(mem: *mut u8, size: usize) {
    for i in 0..16 {
        let pattern = _mm512_srli_epi16(_mm512_set1_epi16(0x8000), i);
        set_all_up(mem, size, pattern);
        get_all_up(mem as *const u8, size, pattern);
        let zeroes = _mm512_set1_epi8(0x00);
        set_all_up(mem, size, zeroes);
        get_all_up(mem as *const u8, size, zeroes);
        set_all_up(mem, size, pattern);
        get_all_up(mem as *const u8, size, pattern);
        let ones = _mm512_set1_epi8(0xFF);
        set_all_up(mem, size, ones);
        get_all_up(mem as *const u8, size, ones);
    }
}

#[cfg(all(target_arch = "x86_64", target_feature = "avx512f"))]
pub unsafe fn avx512_moving_saturations_left_8(mem: *mut u8, size: usize) {
    for i in 0..8 {
        let pattern = _mm512_srli_epi16(_mm512_set1_epi16(0x01), i);
        set_all_up(mem, size, pattern);
        get_all_up(mem as *const u8, size, pattern);
        let zeroes = _mm512_set1_epi8(0x00);
        set_all_up(mem, size, zeroes);
        get_all_up(mem as *const u8, size, zeroes);
        set_all_up(mem, size, pattern);
        get_all_up(mem as *const u8, size, pattern);
        let ones = _mm512_set1_epi8(0xFF);
        set_all_up(mem, size, ones);
        get_all_up(mem as *const u8, size, ones);
    }
}

#[cfg(all(target_arch = "x86_64", target_feature = "avx512f"))]
pub unsafe fn avx512_addressing(mem: *mut u8, size: usize) {
    use rayon::prelude::*;
    let chunk_size = size / CPUS;
    
    for _ in 0..16 {
        let increasing = _mm512_set_epi64(56, 48, 40, 32, 24, 16, 8, 0);
        
        (0..CPUS).into_par_iter().for_each(|i| {
            for j in (0..chunk_size).step_by(64) {
                let idx = j + i * chunk_size;
                let addr_val = idx as i64;
                let pattern = _mm512_add_epi64(_mm512_set1_epi64(addr_val), increasing);
                set(mem, idx, pattern);
            }
        });
        
        (0..CPUS).into_par_iter().for_each(|i| {
            for j in (0..chunk_size).step_by(64) {
                let idx = j + i * chunk_size;
                let addr_val = idx as i64;
                let expected = _mm512_add_epi64(_mm512_set1_epi64(addr_val), increasing);
                get(mem as *const u8, idx, expected);
            }
        });
        
        (0..CPUS).into_par_iter().rev().for_each(|i| {
            let start = i * chunk_size;
            let end = start + chunk_size;
            for j in (start..end).rev().step_by(64) {
                if j + 64 <= end {
                    let addr_val = j as i64;
                    let pattern = _mm512_add_epi64(_mm512_set1_epi64(addr_val), increasing);
                    set(mem, j, pattern);
                }
            }
        });
        
        (0..CPUS).into_par_iter().rev().for_each(|i| {
            let start = i * chunk_size;
            let end = start + chunk_size;
            for j in (start..end).rev().step_by(64) {
                if j + 64 <= end {
                    let addr_val = j as i64;
                    let expected = _mm512_add_epi64(_mm512_set1_epi64(addr_val), increasing);
                    get(mem as *const u8, j, expected);
                }
            }
        });
    }
}

#[cfg(all(target_arch = "x86_64", target_feature = "avx512f"))]
pub unsafe fn avx512_sgemm(mem: *mut u8, size: usize) {
    // SGEMM test requires OpenBLAS - skip if not available
    let _ = mem;
    let _ = size;
}

#[cfg(all(target_arch = "x86_64", target_feature = "avx512f"))]
pub unsafe fn avx512_walking_1(mem: *mut u8, size: usize) {
    for bit in 0..64 {
        let pattern_val = 1u64 << bit;
        let pattern = _mm512_set1_epi64(pattern_val as i64);
        set_all_up(mem, size, pattern);
        get_all_up(mem as *const u8, size, pattern);
        let not_pattern = _mm512_xor_epi64(pattern, _mm512_set1_epi8(0xFF));
        set_all_up(mem, size, not_pattern);
        get_all_up(mem as *const u8, size, not_pattern);
    }
}

#[cfg(all(target_arch = "x86_64", target_feature = "avx512f"))]
pub unsafe fn avx512_walking_0(mem: *mut u8, size: usize) {
    for bit in 0..64 {
        let pattern_val = !(1u64 << bit);
        let pattern = _mm512_set1_epi64(pattern_val as i64);
        set_all_up(mem, size, pattern);
        get_all_up(mem as *const u8, size, pattern);
        let not_pattern = _mm512_xor_epi64(pattern, _mm512_set1_epi8(0xFF));
        set_all_up(mem, size, not_pattern);
        get_all_up(mem as *const u8, size, not_pattern);
    }
}

#[cfg(all(target_arch = "x86_64", target_feature = "avx512f"))]
pub unsafe fn avx512_checkerboard(mem: *mut u8, size: usize) {
    use rayon::prelude::*;
    let chunk_size = size / CPUS;
    
    let pattern1 = _mm512_set1_epi8(0xAA);
    let pattern2 = _mm512_set1_epi8(0x55);
    
    (0..CPUS).into_par_iter().for_each(|i| {
        for j in (0..chunk_size).step_by(64) {
            let idx = j + i * chunk_size;
            let pattern = if ((idx / 64) % 2) != 0 { pattern1 } else { pattern2 };
            set(mem, idx, pattern);
        }
    });
    
    (0..CPUS).into_par_iter().for_each(|i| {
        for j in (0..chunk_size).step_by(64) {
            let idx = j + i * chunk_size;
            let expected = if ((idx / 64) % 2) != 0 { pattern1 } else { pattern2 };
            get(mem as *const u8, idx, expected);
        }
    });
    
    (0..CPUS).into_par_iter().for_each(|i| {
        for j in (0..chunk_size).step_by(64) {
            let idx = j + i * chunk_size;
            let pattern = if ((idx / 64) % 2) != 0 { pattern2 } else { pattern1 };
            set(mem, idx, pattern);
        }
    });
    
    (0..CPUS).into_par_iter().for_each(|i| {
        for j in (0..chunk_size).step_by(64) {
            let idx = j + i * chunk_size;
            let expected = if ((idx / 64) % 2) != 0 { pattern2 } else { pattern1 };
            get(mem as *const u8, idx, expected);
        }
    });
}

#[cfg(all(target_arch = "x86_64", target_feature = "avx512f"))]
pub unsafe fn avx512_address_line_test(mem: *mut u8, size: usize) {
    use rayon::prelude::*;
    let chunk_size = size / CPUS;
    
    (0..CPUS).into_par_iter().for_each(|i| {
        for j in (0..chunk_size).step_by(64) {
            let idx = j + i * chunk_size;
            let addr_pattern = idx as u64;
            let pattern = _mm512_set1_epi64(addr_pattern as i64);
            set(mem, idx, pattern);
        }
    });
    
    (0..CPUS).into_par_iter().for_each(|i| {
        for j in (0..chunk_size).step_by(64) {
            let idx = j + i * chunk_size;
            let addr_pattern = idx as u64;
            let expected = _mm512_set1_epi64(addr_pattern as i64);
            get(mem as *const u8, idx, expected);
        }
    });
    
    (0..CPUS).into_par_iter().rev().for_each(|i| {
        let start = i * chunk_size;
        let end = start + chunk_size;
        for j in (start..end).rev().step_by(64) {
            if j + 64 <= end {
                let addr_pattern = !j as u64;
                let pattern = _mm512_set1_epi64(addr_pattern as i64);
                set(mem, j, pattern);
            }
        }
    });
    
    (0..CPUS).into_par_iter().rev().for_each(|i| {
        let start = i * chunk_size;
        let end = start + chunk_size;
        for j in (start..end).rev().step_by(64) {
            if j + 64 <= end {
                let addr_pattern = !j as u64;
                let expected = _mm512_set1_epi64(addr_pattern as i64);
                get(mem as *const u8, j, expected);
            }
        }
    });
    
    let mut shift = 1;
    while shift <= 16 {
        (0..CPUS).into_par_iter().for_each(|i| {
            for j in (0..chunk_size).step_by(64) {
                let idx = j + i * chunk_size;
                let addr_pattern = idx as u64 ^ ((idx as u64) << shift);
                let pattern = _mm512_set1_epi64(addr_pattern as i64);
                set(mem, idx, pattern);
            }
        });
        
        (0..CPUS).into_par_iter().for_each(|i| {
            for j in (0..chunk_size).step_by(64) {
                let idx = j + i * chunk_size;
                let addr_pattern = idx as u64 ^ ((idx as u64) << shift);
                let expected = _mm512_set1_epi64(addr_pattern as i64);
                get(mem as *const u8, idx, expected);
            }
        });
        shift <<= 1;
    }
}

#[cfg(all(target_arch = "x86_64", target_feature = "avx512f"))]
pub unsafe fn avx512_anti_patterns(mem: *mut u8, size: usize) {
    let patterns = [
        0x00, 0xFF, 0x0F, 0xF0, 0x55, 0xAA, 0x33, 0xCC,
        0x11, 0xEE, 0x22, 0xDD, 0x44, 0xBB, 0x66, 0x99,
        0x77, 0x88, 0x01, 0xFE, 0x02, 0xFD, 0x04, 0xFB,
        0x08, 0xF7, 0x10, 0xEF, 0x20, 0xDF, 0x40, 0xBF,
        0x80, 0x7F,
    ];
    
    for pattern_val in &patterns {
        let pattern = _mm512_set1_epi8(*pattern_val as i8);
        let anti_pattern = _mm512_xor_epi64(pattern, _mm512_set1_epi8(0xFF));
        
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

#[cfg(all(target_arch = "x86_64", target_feature = "avx512f"))]
pub unsafe fn avx512_inverse_data_patterns(mem: *mut u8, size: usize) {
    for byte_idx in 0..8 {
        let base_pattern = 0xFFFFFFFFFFFFFFFFu64;
        let pattern_val = base_pattern ^ (0xFFu64 << (byte_idx * 8));
        let pattern = _mm512_set1_epi64(pattern_val as i64);
        
        set_all_up(mem, size, pattern);
        get_all_up(mem as *const u8, size, pattern);
        
        let inverse = _mm512_xor_epi64(pattern, _mm512_set1_epi8(0xFF));
        set_all_up(mem, size, inverse);
        get_all_up(mem as *const u8, size, inverse);
    }
    
    for word_idx in 0..4 {
        let base_pattern = 0xFFFFFFFFFFFFFFFFu64;
        let pattern_val = base_pattern ^ (0xFFFFu64 << (word_idx * 16));
        let pattern = _mm512_set1_epi64(pattern_val as i64);
        
        set_all_up(mem, size, pattern);
        get_all_up(mem as *const u8, size, pattern);
        
        let inverse = _mm512_xor_epi64(pattern, _mm512_set1_epi8(0xFF));
        set_all_up(mem, size, inverse);
        get_all_up(mem as *const u8, size, inverse);
    }
    
    for dword_idx in 0..2 {
        let base_pattern = 0xFFFFFFFFFFFFFFFFu64;
        let pattern_val = base_pattern ^ (0xFFFFFFFFu64 << (dword_idx * 32));
        let pattern = _mm512_set1_epi64(pattern_val as i64);
        
        set_all_up(mem, size, pattern);
        get_all_up(mem as *const u8, size, pattern);
        
        let inverse = _mm512_xor_epi64(pattern, _mm512_set1_epi8(0xFF));
        set_all_up(mem, size, inverse);
        get_all_up(mem as *const u8, size, inverse);
    }
}

// Stub implementations for non-AVX512 targets
#[cfg(not(all(target_arch = "x86_64", target_feature = "avx512f")))]
pub unsafe fn avx512_tests_init(_cpus: usize, _errors: *const AtomicU64) {}
#[cfg(not(all(target_arch = "x86_64", target_feature = "avx512f")))]
pub unsafe fn avx512_basic_tests(_mem: *mut u8, _size: usize) {}
#[cfg(not(all(target_arch = "x86_64", target_feature = "avx512f")))]
pub unsafe fn avx512_march(_mem: *mut u8, _size: usize) {}
#[cfg(not(all(target_arch = "x86_64", target_feature = "avx512f")))]
pub unsafe fn avx512_random_inversions(_mem: *mut u8, _size: usize) {}
#[cfg(not(all(target_arch = "x86_64", target_feature = "avx512f")))]
pub unsafe fn avx512_moving_inversions_left_64(_mem: *mut u8, _size: usize) {}
#[cfg(not(all(target_arch = "x86_64", target_feature = "avx512f")))]
pub unsafe fn avx512_moving_inversions_right_32(_mem: *mut u8, _size: usize) {}
#[cfg(not(all(target_arch = "x86_64", target_feature = "avx512f")))]
pub unsafe fn avx512_moving_inversions_left_16(_mem: *mut u8, _size: usize) {}
#[cfg(not(all(target_arch = "x86_64", target_feature = "avx512f")))]
pub unsafe fn avx512_moving_inversions_right_8(_mem: *mut u8, _size: usize) {}
#[cfg(not(all(target_arch = "x86_64", target_feature = "avx512f")))]
pub unsafe fn avx512_moving_inversions_left_4(_mem: *mut u8, _size: usize) {}
#[cfg(not(all(target_arch = "x86_64", target_feature = "avx512f")))]
pub unsafe fn avx512_moving_saturations_right_16(_mem: *mut u8, _size: usize) {}
#[cfg(not(all(target_arch = "x86_64", target_feature = "avx512f")))]
pub unsafe fn avx512_moving_saturations_left_8(_mem: *mut u8, _size: usize) {}
#[cfg(not(all(target_arch = "x86_64", target_feature = "avx512f")))]
pub unsafe fn avx512_addressing(_mem: *mut u8, _size: usize) {}
#[cfg(not(all(target_arch = "x86_64", target_feature = "avx512f")))]
pub unsafe fn avx512_sgemm(_mem: *mut u8, _size: usize) {}
#[cfg(not(all(target_arch = "x86_64", target_feature = "avx512f")))]
pub unsafe fn avx512_walking_1(_mem: *mut u8, _size: usize) {}
#[cfg(not(all(target_arch = "x86_64", target_feature = "avx512f")))]
pub unsafe fn avx512_walking_0(_mem: *mut u8, _size: usize) {}
#[cfg(not(all(target_arch = "x86_64", target_feature = "avx512f")))]
pub unsafe fn avx512_checkerboard(_mem: *mut u8, _size: usize) {}
#[cfg(not(all(target_arch = "x86_64", target_feature = "avx512f")))]
pub unsafe fn avx512_address_line_test(_mem: *mut u8, _size: usize) {}
#[cfg(not(all(target_arch = "x86_64", target_feature = "avx512f")))]
pub unsafe fn avx512_anti_patterns(_mem: *mut u8, _size: usize) {}
#[cfg(not(all(target_arch = "x86_64", target_feature = "avx512f")))]
pub unsafe fn avx512_inverse_data_patterns(_mem: *mut u8, _size: usize) {}

