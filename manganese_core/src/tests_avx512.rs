#[cfg(all(target_arch = "x86_64", target_feature = "avx512f"))]
use std::arch::x86_64::*;
use std::sync::atomic::AtomicU64;
use log::error;
use crate::simd_xorshift::Avx512Xorshift128PlusKey;

#[cfg(all(target_arch = "x86_64", target_feature = "avx512f"))]
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
        let error_total = result.count_ones() as u64;
        error!("{} errors detected at offset 0x{:016x} [error mask: 0x{:016x}]", error_total, idx, result);
        (*ERRORS).fetch_add(error_total, std::sync::atomic::Ordering::Relaxed);
    }
}

#[cfg(all(target_arch = "x86_64", target_feature = "avx512f"))]
unsafe fn get_all_up(mem: *const u8, size: usize, expected: __m512i) {
    use rayon::prelude::*;
    let mem_usize = mem as usize;
    
    (0..CPUS).into_par_iter().for_each(|i| {
        let mem_ptr = mem_usize as *const u8;
        let chunk_size = size / CPUS;
        for j in (0..chunk_size).step_by(64) {
            let idx = j + i * chunk_size;
            get(mem_ptr, idx, expected);
        }
    });
}

#[cfg(all(target_arch = "x86_64", target_feature = "avx512f"))]
unsafe fn get_all_down(mem: *const u8, size: usize, expected: __m512i) {
    use rayon::prelude::*;
    let mem_usize = mem as usize;
    
    let chunk_size = size / CPUS;
    (0..CPUS).into_par_iter().rev().for_each(|i| {
        let mem_ptr = mem_usize as *const u8;
        let start = i * chunk_size;
        let end = start + chunk_size;
        // Iterate from end-64 down to start, stepping by 64
        let mut j = ((end - start) / 64) * 64 + start;  // Last aligned position
        while j >= start + 64 {
            j -= 64;
            get(mem_ptr, j, expected);
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
    let mem_usize = mem as usize;
    
    (0..CPUS).into_par_iter().for_each(|i| {
        let mem_ptr = mem_usize as *mut u8;
        let chunk_size = size / CPUS;
        for j in (0..chunk_size).step_by(64) {
            let idx = j + i * chunk_size;
            set(mem_ptr, idx, val);
        }
    });
}

#[cfg(all(target_arch = "x86_64", target_feature = "avx512f"))]
unsafe fn set_all_down(mem: *mut u8, size: usize, val: __m512i) {
    use rayon::prelude::*;
    let mem_usize = mem as usize;
    
    let chunk_size = size / CPUS;
    (0..CPUS).into_par_iter().rev().for_each(|i| {
        let mem_ptr = mem_usize as *mut u8;
        let start = i * chunk_size;
        let end = start + chunk_size;
        // Iterate from end-64 down to start, stepping by 64
        let mut j = ((end - start) / 64) * 64 + start;  // Last aligned position
        while j >= start + 64 {
            j -= 64;
            set(mem_ptr, j, val);
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
    use rayon::prelude::*;
    let mem_usize = mem as usize;
    
    for _ in 0..2 {
        let ones = _mm512_set1_epi8(0xFFu8 as i8);
        let zeroes = _mm512_set1_epi8(0x00u8 as i8);
        let chunk_size = size / CPUS;
        
        (0..CPUS).into_par_iter().rev().for_each(|i| {
            let mem_ptr = mem_usize as *mut u8;
            let start = i * chunk_size;
            let end = start + chunk_size;
            for j in (start..end).rev().step_by(64) {
                if j + 64 <= end {
                    set(mem_ptr, j, zeroes);
                }
            }
        });
        
        (0..CPUS).into_par_iter().for_each(|i| {
            let mem_ptr = mem_usize as *mut u8;
            for j in (0..chunk_size).step_by(64) {
                let idx = j + i * chunk_size;
                get(mem_ptr as *const u8, idx, zeroes);
                set(mem_ptr, idx, ones);
                get(mem_ptr as *const u8, idx, ones);
                set(mem_ptr, idx, zeroes);
                get(mem_ptr as *const u8, idx, zeroes);
                set(mem_ptr, idx, ones);
            }
        });
        
        (0..CPUS).into_par_iter().for_each(|i| {
            let mem_ptr = mem_usize as *mut u8;
            for j in (0..chunk_size).step_by(64) {
                let idx = j + i * chunk_size;
                get(mem_ptr as *const u8, idx, ones);
                set(mem_ptr, idx, zeroes);
                set(mem_ptr, idx, ones);
            }
        });
        
        (0..CPUS).into_par_iter().rev().for_each(|i| {
            let mem_ptr = mem_usize as *mut u8;
            let start = i * chunk_size;
            let end = start + chunk_size;
            for j in (start..end).rev().step_by(64) {
                if j + 64 <= end {
                    get(mem_ptr as *const u8, j, ones);
                    set(mem_ptr, j, zeroes);
                    set(mem_ptr, j, ones);
                    set(mem_ptr, j, zeroes);
                }
            }
        });
        
        (0..CPUS).into_par_iter().rev().for_each(|i| {
            let mem_ptr = mem_usize as *mut u8;
            let start = i * chunk_size;
            let end = start + chunk_size;
            for j in (start..end).rev().step_by(64) {
                if j + 64 <= end {
                    get(mem_ptr as *const u8, j, zeroes);
                    set(mem_ptr, j, ones);
                    set(mem_ptr, j, zeroes);
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
        let not_pattern = _mm512_xor_epi64(pattern, _mm512_set1_epi8(0xFFu8 as i8));
        set_all_up(mem, size, not_pattern);
        get_all_up(mem as *const u8, size, not_pattern);
    }
}

// Moving inversions for AVX-512 - using macros for compile-time constant shifts
#[cfg(all(target_arch = "x86_64", target_feature = "avx512f"))]
pub unsafe fn avx512_moving_inversions_left_64(mem: *mut u8, size: usize) {
    macro_rules! do_shift {
        ($i:expr) => {{
            let pattern = _mm512_slli_epi64::<$i>(_mm512_set1_epi64(0x0000000000000001));
            set_all_up(mem, size, pattern);
            get_all_up(mem as *const u8, size, pattern);
            let not_pattern = _mm512_xor_epi64(pattern, _mm512_set1_epi8(0xFFu8 as i8));
            set_all_up(mem, size, not_pattern);
            get_all_up(mem as *const u8, size, not_pattern);
        }};
    }
    
    for i in 0..64 {
        match i {
            0 => do_shift!(0), 1 => do_shift!(1), 2 => do_shift!(2), 3 => do_shift!(3),
            4 => do_shift!(4), 5 => do_shift!(5), 6 => do_shift!(6), 7 => do_shift!(7),
            8 => do_shift!(8), 9 => do_shift!(9), 10 => do_shift!(10), 11 => do_shift!(11),
            12 => do_shift!(12), 13 => do_shift!(13), 14 => do_shift!(14), 15 => do_shift!(15),
            16 => do_shift!(16), 17 => do_shift!(17), 18 => do_shift!(18), 19 => do_shift!(19),
            20 => do_shift!(20), 21 => do_shift!(21), 22 => do_shift!(22), 23 => do_shift!(23),
            24 => do_shift!(24), 25 => do_shift!(25), 26 => do_shift!(26), 27 => do_shift!(27),
            28 => do_shift!(28), 29 => do_shift!(29), 30 => do_shift!(30), 31 => do_shift!(31),
            32 => do_shift!(32), 33 => do_shift!(33), 34 => do_shift!(34), 35 => do_shift!(35),
            36 => do_shift!(36), 37 => do_shift!(37), 38 => do_shift!(38), 39 => do_shift!(39),
            40 => do_shift!(40), 41 => do_shift!(41), 42 => do_shift!(42), 43 => do_shift!(43),
            44 => do_shift!(44), 45 => do_shift!(45), 46 => do_shift!(46), 47 => do_shift!(47),
            48 => do_shift!(48), 49 => do_shift!(49), 50 => do_shift!(50), 51 => do_shift!(51),
            52 => do_shift!(52), 53 => do_shift!(53), 54 => do_shift!(54), 55 => do_shift!(55),
            56 => do_shift!(56), 57 => do_shift!(57), 58 => do_shift!(58), 59 => do_shift!(59),
            60 => do_shift!(60), 61 => do_shift!(61), 62 => do_shift!(62), 63 => do_shift!(63),
            _ => {}
        }
    }
}

#[cfg(all(target_arch = "x86_64", target_feature = "avx512f"))]
pub unsafe fn avx512_moving_inversions_right_32(mem: *mut u8, size: usize) {
    macro_rules! do_shift {
        ($i:expr) => {{
            let pattern = _mm512_srli_epi64::<$i>(_mm512_set1_epi32(0x80000000u32 as i32));
            set_all_up(mem, size, pattern);
            get_all_up(mem as *const u8, size, pattern);
            let not_pattern = _mm512_xor_epi64(pattern, _mm512_set1_epi8(0xFFu8 as i8));
            set_all_up(mem, size, not_pattern);
            get_all_up(mem as *const u8, size, not_pattern);
        }};
    }
    
    for i in 0..32 {
        match i {
            0 => do_shift!(0), 1 => do_shift!(1), 2 => do_shift!(2), 3 => do_shift!(3),
            4 => do_shift!(4), 5 => do_shift!(5), 6 => do_shift!(6), 7 => do_shift!(7),
            8 => do_shift!(8), 9 => do_shift!(9), 10 => do_shift!(10), 11 => do_shift!(11),
            12 => do_shift!(12), 13 => do_shift!(13), 14 => do_shift!(14), 15 => do_shift!(15),
            16 => do_shift!(16), 17 => do_shift!(17), 18 => do_shift!(18), 19 => do_shift!(19),
            20 => do_shift!(20), 21 => do_shift!(21), 22 => do_shift!(22), 23 => do_shift!(23),
            24 => do_shift!(24), 25 => do_shift!(25), 26 => do_shift!(26), 27 => do_shift!(27),
            28 => do_shift!(28), 29 => do_shift!(29), 30 => do_shift!(30), 31 => do_shift!(31),
            _ => {}
        }
    }
}

#[cfg(all(target_arch = "x86_64", target_feature = "avx512f"))]
pub unsafe fn avx512_moving_inversions_left_16(mem: *mut u8, size: usize) {
    macro_rules! do_shift {
        ($i:expr) => {{
            let pattern = _mm512_slli_epi64::<$i>(_mm512_set1_epi16(0x0001u16 as i16));
            set_all_up(mem, size, pattern);
            get_all_up(mem as *const u8, size, pattern);
            let not_pattern = _mm512_xor_epi64(pattern, _mm512_set1_epi8(0xFFu8 as i8));
            set_all_up(mem, size, not_pattern);
            get_all_up(mem as *const u8, size, not_pattern);
        }};
    }
    
    for i in 0..16 {
        match i {
            0 => do_shift!(0), 1 => do_shift!(1), 2 => do_shift!(2), 3 => do_shift!(3),
            4 => do_shift!(4), 5 => do_shift!(5), 6 => do_shift!(6), 7 => do_shift!(7),
            8 => do_shift!(8), 9 => do_shift!(9), 10 => do_shift!(10), 11 => do_shift!(11),
            12 => do_shift!(12), 13 => do_shift!(13), 14 => do_shift!(14), 15 => do_shift!(15),
            _ => {}
        }
    }
}

#[cfg(all(target_arch = "x86_64", target_feature = "avx512f"))]
pub unsafe fn avx512_moving_inversions_right_8(mem: *mut u8, size: usize) {
    macro_rules! do_shift {
        ($i:expr) => {{
            let pattern = _mm512_srli_epi64::<$i>(_mm512_set1_epi8(0x80u8 as i8));
            set_all_up(mem, size, pattern);
            get_all_up(mem as *const u8, size, pattern);
            let not_pattern = _mm512_xor_epi64(pattern, _mm512_set1_epi8(0xFFu8 as i8));
            set_all_up(mem, size, not_pattern);
            get_all_up(mem as *const u8, size, not_pattern);
        }};
    }
    
    for i in 0..8 {
        match i {
            0 => do_shift!(0), 1 => do_shift!(1), 2 => do_shift!(2), 3 => do_shift!(3),
            4 => do_shift!(4), 5 => do_shift!(5), 6 => do_shift!(6), 7 => do_shift!(7),
            _ => {}
        }
    }
}

#[cfg(all(target_arch = "x86_64", target_feature = "avx512f"))]
pub unsafe fn avx512_moving_inversions_left_4(mem: *mut u8, size: usize) {
    macro_rules! do_shift {
        ($i:expr) => {{
            let pattern = _mm512_slli_epi64::<$i>(_mm512_set1_epi8(0x11u8 as i8));
            set_all_up(mem, size, pattern);
            get_all_up(mem as *const u8, size, pattern);
            let not_pattern = _mm512_xor_epi64(pattern, _mm512_set1_epi8(0xFFu8 as i8));
            set_all_up(mem, size, not_pattern);
            get_all_up(mem as *const u8, size, not_pattern);
        }};
    }
    
    for i in 0..4 {
        match i {
            0 => do_shift!(0), 1 => do_shift!(1), 2 => do_shift!(2), 3 => do_shift!(3),
            _ => {}
        }
    }
}

#[cfg(all(target_arch = "x86_64", target_feature = "avx512f"))]
pub unsafe fn avx512_moving_saturations_right_16(mem: *mut u8, size: usize) {
    macro_rules! do_test {
        ($i:expr) => {{
            let pattern = _mm512_srli_epi16::<$i>(_mm512_set1_epi16(0x8000u16 as i16));
            set_all_up(mem, size, pattern);
            get_all_up(mem as *const u8, size, pattern);
            let zeroes = _mm512_set1_epi8(0x00u8 as i8);
            set_all_up(mem, size, zeroes);
            get_all_up(mem as *const u8, size, zeroes);
            set_all_up(mem, size, pattern);
            get_all_up(mem as *const u8, size, pattern);
            let ones = _mm512_set1_epi8(0xFFu8 as i8);
            set_all_up(mem, size, ones);
            get_all_up(mem as *const u8, size, ones);
        }};
    }
    
    for i in 0..16 {
        match i {
            0 => do_test!(0), 1 => do_test!(1), 2 => do_test!(2), 3 => do_test!(3),
            4 => do_test!(4), 5 => do_test!(5), 6 => do_test!(6), 7 => do_test!(7),
            8 => do_test!(8), 9 => do_test!(9), 10 => do_test!(10), 11 => do_test!(11),
            12 => do_test!(12), 13 => do_test!(13), 14 => do_test!(14), 15 => do_test!(15),
            _ => {}
        }
    }
}

#[cfg(all(target_arch = "x86_64", target_feature = "avx512f"))]
pub unsafe fn avx512_moving_saturations_left_8(mem: *mut u8, size: usize) {
    macro_rules! do_test {
        ($i:expr) => {{
            let pattern = _mm512_srli_epi16::<$i>(_mm512_set1_epi16(0x01u16 as i16));
            set_all_up(mem, size, pattern);
            get_all_up(mem as *const u8, size, pattern);
            let zeroes = _mm512_set1_epi8(0x00u8 as i8);
            set_all_up(mem, size, zeroes);
            get_all_up(mem as *const u8, size, zeroes);
            set_all_up(mem, size, pattern);
            get_all_up(mem as *const u8, size, pattern);
            let ones = _mm512_set1_epi8(0xFFu8 as i8);
            set_all_up(mem, size, ones);
            get_all_up(mem as *const u8, size, ones);
        }};
    }
    
    for i in 0..8 {
        match i {
            0 => do_test!(0), 1 => do_test!(1), 2 => do_test!(2), 3 => do_test!(3),
            4 => do_test!(4), 5 => do_test!(5), 6 => do_test!(6), 7 => do_test!(7),
            _ => {}
        }
    }
}

#[cfg(all(target_arch = "x86_64", target_feature = "avx512f"))]
pub unsafe fn avx512_addressing(mem: *mut u8, size: usize) {
    use rayon::prelude::*;
    let mem_usize = mem as usize;
    let chunk_size = size / CPUS;
    
    for _ in 0..16 {
        let increasing = _mm512_set_epi64(56, 48, 40, 32, 24, 16, 8, 0);
        
        (0..CPUS).into_par_iter().for_each(|i| {
            let mem_ptr = mem_usize as *mut u8;
            for j in (0..chunk_size).step_by(64) {
                let idx = j + i * chunk_size;
                let addr_val = idx as i64;
                let pattern = _mm512_add_epi64(_mm512_set1_epi64(addr_val), increasing);
                set(mem_ptr, idx, pattern);
            }
        });
        
        (0..CPUS).into_par_iter().for_each(|i| {
            let mem_ptr = mem_usize as *const u8;
            for j in (0..chunk_size).step_by(64) {
                let idx = j + i * chunk_size;
                let addr_val = idx as i64;
                let expected = _mm512_add_epi64(_mm512_set1_epi64(addr_val), increasing);
                get(mem_ptr, idx, expected);
            }
        });
        
        (0..CPUS).into_par_iter().rev().for_each(|i| {
            let mem_ptr = mem_usize as *mut u8;
            let start = i * chunk_size;
            let end = start + chunk_size;
            for j in (start..end).rev().step_by(64) {
                if j + 64 <= end {
                    let addr_val = j as i64;
                    let pattern = _mm512_add_epi64(_mm512_set1_epi64(addr_val), increasing);
                    set(mem_ptr, j, pattern);
                }
            }
        });
        
        (0..CPUS).into_par_iter().rev().for_each(|i| {
            let mem_ptr = mem_usize as *const u8;
            let start = i * chunk_size;
            let end = start + chunk_size;
            for j in (start..end).rev().step_by(64) {
                if j + 64 <= end {
                    let addr_val = j as i64;
                    let expected = _mm512_add_epi64(_mm512_set1_epi64(addr_val), increasing);
                    get(mem_ptr, j, expected);
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
        let not_pattern = _mm512_xor_epi64(pattern, _mm512_set1_epi8(0xFFu8 as i8));
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
        let not_pattern = _mm512_xor_epi64(pattern, _mm512_set1_epi8(0xFFu8 as i8));
        set_all_up(mem, size, not_pattern);
        get_all_up(mem as *const u8, size, not_pattern);
    }
}

#[cfg(all(target_arch = "x86_64", target_feature = "avx512f"))]
pub unsafe fn avx512_checkerboard(mem: *mut u8, size: usize) {
    use rayon::prelude::*;
    let mem_usize = mem as usize;
    let chunk_size = size / CPUS;
    
    let pattern1 = _mm512_set1_epi8(0xAAu8 as i8);
    let pattern2 = _mm512_set1_epi8(0x55u8 as i8);
    
    (0..CPUS).into_par_iter().for_each(|i| {
        let mem_ptr = mem_usize as *mut u8;
        for j in (0..chunk_size).step_by(64) {
            let idx = j + i * chunk_size;
            let pattern = if ((idx / 64) % 2) != 0 { pattern1 } else { pattern2 };
            set(mem_ptr, idx, pattern);
        }
    });
    
    (0..CPUS).into_par_iter().for_each(|i| {
        let mem_ptr = mem_usize as *const u8;
        for j in (0..chunk_size).step_by(64) {
            let idx = j + i * chunk_size;
            let expected = if ((idx / 64) % 2) != 0 { pattern1 } else { pattern2 };
            get(mem_ptr, idx, expected);
        }
    });
    
    (0..CPUS).into_par_iter().for_each(|i| {
        let mem_ptr = mem_usize as *mut u8;
        for j in (0..chunk_size).step_by(64) {
            let idx = j + i * chunk_size;
            let pattern = if ((idx / 64) % 2) != 0 { pattern2 } else { pattern1 };
            set(mem_ptr, idx, pattern);
        }
    });
    
    (0..CPUS).into_par_iter().for_each(|i| {
        let mem_ptr = mem_usize as *const u8;
        for j in (0..chunk_size).step_by(64) {
            let idx = j + i * chunk_size;
            let expected = if ((idx / 64) % 2) != 0 { pattern2 } else { pattern1 };
            get(mem_ptr, idx, expected);
        }
    });
}

#[cfg(all(target_arch = "x86_64", target_feature = "avx512f"))]
pub unsafe fn avx512_address_line_test(mem: *mut u8, size: usize) {
    use rayon::prelude::*;
    let mem_usize = mem as usize;
    let chunk_size = size / CPUS;
    
    (0..CPUS).into_par_iter().for_each(|i| {
        let mem_ptr = mem_usize as *mut u8;
        for j in (0..chunk_size).step_by(64) {
            let idx = j + i * chunk_size;
            let addr_pattern = idx as u64;
            let pattern = _mm512_set1_epi64(addr_pattern as i64);
            set(mem_ptr, idx, pattern);
        }
    });
    
    (0..CPUS).into_par_iter().for_each(|i| {
        let mem_ptr = mem_usize as *const u8;
        for j in (0..chunk_size).step_by(64) {
            let idx = j + i * chunk_size;
            let addr_pattern = idx as u64;
            let expected = _mm512_set1_epi64(addr_pattern as i64);
            get(mem_ptr, idx, expected);
        }
    });
    
    (0..CPUS).into_par_iter().rev().for_each(|i| {
        let mem_ptr = mem_usize as *mut u8;
        let start = i * chunk_size;
        let end = start + chunk_size;
        for j in (start..end).rev().step_by(64) {
            if j + 64 <= end {
                let addr_pattern = !j as u64;
                let pattern = _mm512_set1_epi64(addr_pattern as i64);
                set(mem_ptr, j, pattern);
            }
        }
    });
    
    (0..CPUS).into_par_iter().rev().for_each(|i| {
        let mem_ptr = mem_usize as *const u8;
        let start = i * chunk_size;
        let end = start + chunk_size;
        for j in (start..end).rev().step_by(64) {
            if j + 64 <= end {
                let addr_pattern = !j as u64;
                let expected = _mm512_set1_epi64(addr_pattern as i64);
                get(mem_ptr, j, expected);
            }
        }
    });
    
    let mut shift = 1;
    while shift <= 16 {
        (0..CPUS).into_par_iter().for_each(|i| {
            let mem_ptr = mem_usize as *mut u8;
            for j in (0..chunk_size).step_by(64) {
                let idx = j + i * chunk_size;
                let addr_pattern = idx as u64 ^ ((idx as u64) << shift);
                let pattern = _mm512_set1_epi64(addr_pattern as i64);
                set(mem_ptr, idx, pattern);
            }
        });
        
        (0..CPUS).into_par_iter().for_each(|i| {
            let mem_ptr = mem_usize as *const u8;
            for j in (0..chunk_size).step_by(64) {
                let idx = j + i * chunk_size;
                let addr_pattern = idx as u64 ^ ((idx as u64) << shift);
                let expected = _mm512_set1_epi64(addr_pattern as i64);
                get(mem_ptr, idx, expected);
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
        let anti_pattern = _mm512_xor_epi64(pattern, _mm512_set1_epi8(0xFFu8 as i8));
        
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
        
        let inverse = _mm512_xor_epi64(pattern, _mm512_set1_epi8(0xFFu8 as i8));
        set_all_up(mem, size, inverse);
        get_all_up(mem as *const u8, size, inverse);
    }
    
    for word_idx in 0..4 {
        let base_pattern = 0xFFFFFFFFFFFFFFFFu64;
        let pattern_val = base_pattern ^ (0xFFFFu64 << (word_idx * 16));
        let pattern = _mm512_set1_epi64(pattern_val as i64);
        
        set_all_up(mem, size, pattern);
        get_all_up(mem as *const u8, size, pattern);
        
        let inverse = _mm512_xor_epi64(pattern, _mm512_set1_epi8(0xFFu8 as i8));
        set_all_up(mem, size, inverse);
        get_all_up(mem as *const u8, size, inverse);
    }
    
    for dword_idx in 0..2 {
        let base_pattern = 0xFFFFFFFFFFFFFFFFu64;
        let pattern_val = base_pattern ^ (0xFFFFFFFFu64 << (dword_idx * 32));
        let pattern = _mm512_set1_epi64(pattern_val as i64);
        
        set_all_up(mem, size, pattern);
        get_all_up(mem as *const u8, size, pattern);
        
        let inverse = _mm512_xor_epi64(pattern, _mm512_set1_epi8(0xFFu8 as i8));
        set_all_up(mem, size, inverse);
        get_all_up(mem as *const u8, size, inverse);
    }
}

//FIXME: remove stubs and/or error out when running in unsupported configuration
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
