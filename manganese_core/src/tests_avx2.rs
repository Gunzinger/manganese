#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;
use std::sync::atomic::AtomicU64;
use log::error;
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
    
    if result != 0 {
        error!("errors detected at offset 0x{:016x}", idx);
        (*ERRORS).fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
}

#[cfg(target_arch = "x86_64")]
unsafe fn get_all_up(mem: *const u8, size: usize, expected: __m256i) {
    use rayon::prelude::*;
    let mem_usize = mem as usize;  // Convert to usize for thread safety
    
    (0..CPUS).into_par_iter().for_each(|i| {
        let mem_ptr = mem_usize as *const u8;
        let chunk_size = size / CPUS;
        for j in (0..chunk_size).step_by(32) {
            let idx = j + i * chunk_size;
            get(mem_ptr, idx, expected);
        }
    });
}

#[cfg(target_arch = "x86_64")]
unsafe fn get_all_down(mem: *const u8, size: usize, expected: __m256i) {
    use rayon::prelude::*;
    let mem_usize = mem as usize;
    
    let chunk_size = size / CPUS;
    (0..CPUS).into_par_iter().rev().for_each(|i| {
        let mem_ptr = mem_usize as *const u8;
        let start = i * chunk_size;
        let end = start + chunk_size;
        // Iterate from end-32 down to start, stepping by 32
        let mut j = ((end - start) / 32) * 32 + start;  // Last aligned position
        while j >= start + 32 {
            j -= 32;
            get(mem_ptr, j, expected);
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
    let mem_usize = mem as usize;
    
    (0..CPUS).into_par_iter().for_each(|i| {
        let mem_ptr = mem_usize as *mut u8;
        let chunk_size = size / CPUS;
        for j in (0..chunk_size).step_by(32) {
            let idx = j + i * chunk_size;
            set(mem_ptr, idx, val);
        }
    });
}

#[cfg(target_arch = "x86_64")]
unsafe fn set_all_down(mem: *mut u8, size: usize, val: __m256i) {
    use rayon::prelude::*;
    let mem_usize = mem as usize;
    
    let chunk_size = size / CPUS;
    (0..CPUS).into_par_iter().rev().for_each(|i| {
        let mem_ptr = mem_usize as *mut u8;
        let start = i * chunk_size;
        let end = start + chunk_size;
        // Iterate from end-32 down to start, stepping by 32
        let mut j = ((end - start) / 32) * 32 + start;  // Last aligned position
        while j >= start + 32 {
            j -= 32;
            set(mem_ptr, j, val);
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
    use rayon::prelude::*;
    let mem_usize = mem as usize;
    
    for _ in 0..2 {
        let ones = _mm256_set1_epi8(0xFFu8 as i8);
        let zeroes = _mm256_set1_epi8(0x00u8 as i8);
        let chunk_size = size / CPUS;
        
        // Down: set zeroes
        (0..CPUS).into_par_iter().rev().for_each(|i| {
            let mem_ptr = mem_usize as *mut u8;
            let start = i * chunk_size;
            let end = start + chunk_size;
            for j in (start..end).rev().step_by(32) {
                if j + 32 <= end {
                    set(mem_ptr, j, zeroes);
                }
            }
        });
        
        // Up: get zeroes, set ones, get ones, set zeroes, get zeroes, set ones
        (0..CPUS).into_par_iter().for_each(|i| {
            let mem_ptr = mem_usize as *mut u8;
            for j in (0..chunk_size).step_by(32) {
                let idx = j + i * chunk_size;
                get(mem_ptr as *const u8, idx, zeroes);
                set(mem_ptr, idx, ones);
                get(mem_ptr as *const u8, idx, ones);
                set(mem_ptr, idx, zeroes);
                get(mem_ptr as *const u8, idx, zeroes);
                set(mem_ptr, idx, ones);
            }
        });
        
        // Up: get ones, set zeroes, set ones
        (0..CPUS).into_par_iter().for_each(|i| {
            let mem_ptr = mem_usize as *mut u8;
            for j in (0..chunk_size).step_by(32) {
                let idx = j + i * chunk_size;
                get(mem_ptr as *const u8, idx, ones);
                set(mem_ptr, idx, zeroes);
                set(mem_ptr, idx, ones);
            }
        });
        
        // Down: get ones, set zeroes, set ones, set zeroes
        (0..CPUS).into_par_iter().rev().for_each(|i| {
            let mem_ptr = mem_usize as *mut u8;
            let start = i * chunk_size;
            let end = start + chunk_size;
            for j in (start..end).rev().step_by(32) {
                if j + 32 <= end {
                    get(mem_ptr as *const u8, j, ones);
                    set(mem_ptr, j, zeroes);
                    set(mem_ptr, j, ones);
                    set(mem_ptr, j, zeroes);
                }
            }
        });
        
        // Down: get zeroes, set ones, set zeroes
        (0..CPUS).into_par_iter().rev().for_each(|i| {
            let mem_ptr = mem_usize as *mut u8;
            let start = i * chunk_size;
            let end = start + chunk_size;
            for j in (start..end).rev().step_by(32) {
                if j + 32 <= end {
                    get(mem_ptr as *const u8, j, zeroes);
                    set(mem_ptr, j, ones);
                    set(mem_ptr, j, zeroes);
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
        let not_pattern = _mm256_xor_si256(pattern, _mm256_set1_epi8(0xFFu8 as i8));
        set_all_up(mem, size, not_pattern);
        get_all_up(mem as *const u8, size, not_pattern);
    }
}

#[cfg(target_arch = "x86_64")]
pub unsafe fn avx2_moving_inversions_left_64(mem: *mut u8, size: usize) {
    macro_rules! do_shift {
        ($i:expr) => {{
            let pattern = _mm256_slli_epi64::<$i>(_mm256_set1_epi64x(0x0000000000000001));
            set_all_up(mem, size, pattern);
            get_all_up(mem as *const u8, size, pattern);
            let not_pattern = _mm256_xor_si256(pattern, _mm256_set1_epi8(0xFFu8 as i8));
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

#[cfg(target_arch = "x86_64")]
pub unsafe fn avx2_moving_inversions_right_32(mem: *mut u8, size: usize) {
    macro_rules! do_shift {
        ($i:expr) => {{
            let pattern = _mm256_srli_epi64::<$i>(_mm256_set1_epi32(0x80000000u32 as i32));
            set_all_up(mem, size, pattern);
            get_all_up(mem as *const u8, size, pattern);
            let not_pattern = _mm256_xor_si256(pattern, _mm256_set1_epi8(0xFFu8 as i8));
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

#[cfg(target_arch = "x86_64")]
pub unsafe fn avx2_moving_inversions_left_16(mem: *mut u8, size: usize) {
    macro_rules! do_shift {
        ($i:expr) => {{
            let pattern = _mm256_slli_epi64::<$i>(_mm256_set1_epi16(0x0001));
            set_all_up(mem, size, pattern);
            get_all_up(mem as *const u8, size, pattern);
            let not_pattern = _mm256_xor_si256(pattern, _mm256_set1_epi8(0xFFu8 as i8));
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

#[cfg(target_arch = "x86_64")]
pub unsafe fn avx2_moving_inversions_right_8(mem: *mut u8, size: usize) {
    macro_rules! do_shift {
        ($i:expr) => {{
            let pattern = _mm256_srli_epi64::<$i>(_mm256_set1_epi8(0x80u8 as i8));
            set_all_up(mem, size, pattern);
            get_all_up(mem as *const u8, size, pattern);
            let not_pattern = _mm256_xor_si256(pattern, _mm256_set1_epi8(0xFFu8 as i8));
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

#[cfg(target_arch = "x86_64")]
pub unsafe fn avx2_moving_inversions_left_4(mem: *mut u8, size: usize) {
    macro_rules! do_shift {
        ($i:expr) => {{
            let pattern = _mm256_slli_epi64::<$i>(_mm256_set1_epi8(0x11u8 as i8));
            set_all_up(mem, size, pattern);
            get_all_up(mem as *const u8, size, pattern);
            let not_pattern = _mm256_xor_si256(pattern, _mm256_set1_epi8(0xFFu8 as i8));
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

#[cfg(target_arch = "x86_64")]
pub unsafe fn avx2_moving_saturations_right_16(mem: *mut u8, size: usize) {
    macro_rules! do_test {
        ($i:expr) => {{
            let pattern = _mm256_srli_epi16::<$i>(_mm256_set1_epi16(0x8000u16 as i16));
            set_all_up(mem, size, pattern);
            get_all_up(mem as *const u8, size, pattern);
            let zeroes = _mm256_set1_epi8(0x00u8 as i8);
            set_all_up(mem, size, zeroes);
            get_all_up(mem as *const u8, size, zeroes);
            set_all_up(mem, size, pattern);
            get_all_up(mem as *const u8, size, pattern);
            let ones = _mm256_set1_epi8(0xFFu8 as i8);
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

#[cfg(target_arch = "x86_64")]
pub unsafe fn avx2_moving_saturations_left_8(mem: *mut u8, size: usize) {
    macro_rules! do_test {
        ($i:expr) => {{
            let pattern = _mm256_srli_epi16::<$i>(_mm256_set1_epi16(0x01));
            set_all_up(mem, size, pattern);
            get_all_up(mem as *const u8, size, pattern);
            let zeroes = _mm256_set1_epi8(0x00u8 as i8);
            set_all_up(mem, size, zeroes);
            get_all_up(mem as *const u8, size, zeroes);
            set_all_up(mem, size, pattern);
            get_all_up(mem as *const u8, size, pattern);
            let ones = _mm256_set1_epi8(0xFFu8 as i8);
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

#[cfg(target_arch = "x86_64")]
pub unsafe fn avx2_addressing(mem: *mut u8, size: usize) {
    use rayon::prelude::*;
    let mem_usize = mem as usize;
    let chunk_size = size / CPUS;
    
    for _ in 0..16 {
        let increasing = _mm256_set_epi64x(24, 16, 8, 0);
        
        (0..CPUS).into_par_iter().for_each(|i| {
            let mem_ptr = mem_usize as *mut u8;
            for j in (0..chunk_size).step_by(32) {
                let idx = j + i * chunk_size;
                let addr_val = idx as i64;
                let pattern = _mm256_add_epi64(_mm256_set1_epi64x(addr_val), increasing);
                set(mem_ptr, idx, pattern);
            }
        });
        
        (0..CPUS).into_par_iter().for_each(|i| {
            let mem_ptr = mem_usize as *const u8;
            for j in (0..chunk_size).step_by(32) {
                let idx = j + i * chunk_size;
                let addr_val = idx as i64;
                let expected = _mm256_add_epi64(_mm256_set1_epi64x(addr_val), increasing);
                get(mem_ptr, idx, expected);
            }
        });
        
        (0..CPUS).into_par_iter().rev().for_each(|i| {
            let mem_ptr = mem_usize as *mut u8;
            let start = i * chunk_size;
            let end = start + chunk_size;
            for j in (start..end).rev().step_by(32) {
                if j + 32 <= end {
                    let addr_val = j as i64;
                    let pattern = _mm256_add_epi64(_mm256_set1_epi64x(addr_val), increasing);
                    set(mem_ptr, j, pattern);
                }
            }
        });
        
        (0..CPUS).into_par_iter().rev().for_each(|i| {
            let mem_ptr = mem_usize as *const u8;
            let start = i * chunk_size;
            let end = start + chunk_size;
            for j in (start..end).rev().step_by(32) {
                if j + 32 <= end {
                    let addr_val = j as i64;
                    let expected = _mm256_add_epi64(_mm256_set1_epi64x(addr_val), increasing);
                    get(mem_ptr, j, expected);
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
        let not_pattern = _mm256_xor_si256(pattern, _mm256_set1_epi8(0xFFu8 as i8));
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
        let not_pattern = _mm256_xor_si256(pattern, _mm256_set1_epi8(0xFFu8 as i8));
        set_all_up(mem, size, not_pattern);
        get_all_up(mem as *const u8, size, not_pattern);
    }
}

#[cfg(target_arch = "x86_64")]
pub unsafe fn avx2_checkerboard(mem: *mut u8, size: usize) {
    use rayon::prelude::*;
    let mem_usize = mem as usize;
    let chunk_size = size / CPUS;
    
    let pattern1 = _mm256_set1_epi8(0xAAu8 as i8);
    let pattern2 = _mm256_set1_epi8(0x55u8 as i8);
    
    (0..CPUS).into_par_iter().for_each(|i| {
        let mem_ptr = mem_usize as *mut u8;
        for j in (0..chunk_size).step_by(32) {
            let idx = j + i * chunk_size;
            let pattern = if ((idx / 32) % 2) != 0 { pattern1 } else { pattern2 };
            set(mem_ptr, idx, pattern);
        }
    });
    
    (0..CPUS).into_par_iter().for_each(|i| {
        let mem_ptr = mem_usize as *const u8;
        for j in (0..chunk_size).step_by(32) {
            let idx = j + i * chunk_size;
            let expected = if ((idx / 32) % 2) != 0 { pattern1 } else { pattern2 };
            get(mem_ptr, idx, expected);
        }
    });
    
    (0..CPUS).into_par_iter().for_each(|i| {
        let mem_ptr = mem_usize as *mut u8;
        for j in (0..chunk_size).step_by(32) {
            let idx = j + i * chunk_size;
            let pattern = if ((idx / 32) % 2) != 0 { pattern2 } else { pattern1 };
            set(mem_ptr, idx, pattern);
        }
    });
    
    (0..CPUS).into_par_iter().for_each(|i| {
        let mem_ptr = mem_usize as *const u8;
        for j in (0..chunk_size).step_by(32) {
            let idx = j + i * chunk_size;
            let expected = if ((idx / 32) % 2) != 0 { pattern2 } else { pattern1 };
            get(mem_ptr, idx, expected);
        }
    });
}

#[cfg(target_arch = "x86_64")]
pub unsafe fn avx2_address_line_test(mem: *mut u8, size: usize) {
    use rayon::prelude::*;
    let mem_usize = mem as usize;
    let chunk_size = size / CPUS;
    
    (0..CPUS).into_par_iter().for_each(|i| {
        let mem_ptr = mem_usize as *mut u8;
        for j in (0..chunk_size).step_by(32) {
            let idx = j + i * chunk_size;
            let addr_pattern = idx as u64;
            let pattern = _mm256_set1_epi64x(addr_pattern as i64);
            set(mem_ptr, idx, pattern);
        }
    });
    
    (0..CPUS).into_par_iter().for_each(|i| {
        let mem_ptr = mem_usize as *const u8;
        for j in (0..chunk_size).step_by(32) {
            let idx = j + i * chunk_size;
            let addr_pattern = idx as u64;
            let expected = _mm256_set1_epi64x(addr_pattern as i64);
            get(mem_ptr, idx, expected);
        }
    });
    
    (0..CPUS).into_par_iter().rev().for_each(|i| {
        let mem_ptr = mem_usize as *mut u8;
        let start = i * chunk_size;
        let end = start + chunk_size;
        for j in (start..end).rev().step_by(32) {
            if j + 32 <= end {
                let addr_pattern = !j as u64;
                let pattern = _mm256_set1_epi64x(addr_pattern as i64);
                set(mem_ptr, j, pattern);
            }
        }
    });
    
    (0..CPUS).into_par_iter().rev().for_each(|i| {
        let mem_ptr = mem_usize as *const u8;
        let start = i * chunk_size;
        let end = start + chunk_size;
        for j in (start..end).rev().step_by(32) {
            if j + 32 <= end {
                let addr_pattern = !j as u64;
                let expected = _mm256_set1_epi64x(addr_pattern as i64);
                get(mem_ptr, j, expected);
            }
        }
    });
    
    let mut shift = 1;
    while shift <= 16 {
        (0..CPUS).into_par_iter().for_each(|i| {
            let mem_ptr = mem_usize as *mut u8;
            for j in (0..chunk_size).step_by(32) {
                let idx = j + i * chunk_size;
                let addr_pattern = idx as u64 ^ ((idx as u64) << shift);
                let pattern = _mm256_set1_epi64x(addr_pattern as i64);
                set(mem_ptr, idx, pattern);
            }
        });
        
        (0..CPUS).into_par_iter().for_each(|i| {
            let mem_ptr = mem_usize as *const u8;
            for j in (0..chunk_size).step_by(32) {
                let idx = j + i * chunk_size;
                let addr_pattern = idx as u64 ^ ((idx as u64) << shift);
                let expected = _mm256_set1_epi64x(addr_pattern as i64);
                get(mem_ptr, idx, expected);
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
        let anti_pattern = _mm256_xor_si256(pattern, _mm256_set1_epi8(0xFFu8 as i8));
        
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
        
        let inverse = _mm256_xor_si256(pattern, _mm256_set1_epi8(0xFFu8 as i8));
        set_all_up(mem, size, inverse);
        get_all_up(mem as *const u8, size, inverse);
    }
    
    for word_idx in 0..4 {
        let base_pattern = 0xFFFFFFFFFFFFFFFFu64;
        let pattern_val = base_pattern ^ (0xFFFFu64 << (word_idx * 16));
        let pattern = _mm256_set1_epi64x(pattern_val as i64);
        
        set_all_up(mem, size, pattern);
        get_all_up(mem as *const u8, size, pattern);
        
        let inverse = _mm256_xor_si256(pattern, _mm256_set1_epi8(0xFFu8 as i8));
        set_all_up(mem, size, inverse);
        get_all_up(mem as *const u8, size, inverse);
    }
    
    for dword_idx in 0..2 {
        let base_pattern = 0xFFFFFFFFFFFFFFFFu64;
        let pattern_val = base_pattern ^ (0xFFFFFFFFu64 << (dword_idx * 32));
        let pattern = _mm256_set1_epi64x(pattern_val as i64);
        
        set_all_up(mem, size, pattern);
        get_all_up(mem as *const u8, size, pattern);
        
        let inverse = _mm256_xor_si256(pattern, _mm256_set1_epi8(0xFFu8 as i8));
        set_all_up(mem, size, inverse);
        get_all_up(mem as *const u8, size, inverse);
    }
}

//FIXME: remove stubs and/or error out when running in unsupported configuration
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

