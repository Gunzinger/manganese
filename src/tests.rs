use std::sync::atomic::AtomicU64;

use crate::hardware::InstructionSet;
use crate::tests_avx2::*;
use crate::tests_avx512::*;

pub struct Test {
    pub name: &'static str,
    pub passes: usize,
    pub iters: usize,
    pub run: unsafe fn(*mut u8, usize),
}

pub fn tests_init(cpus: usize, errors: &'static AtomicU64, isa: InstructionSet) -> Vec<Test> {
    match isa {
        InstructionSet::AVX512 => {
            unsafe { avx512_tests_init(cpus, errors); }
            vec![
                Test { name: "Basic Tests", passes: 4, iters: 6, run: avx512_basic_tests },
                //FIXME: segfaults Test: Test { name: "March", passes: 17, iters: 2, run: avx512_march },
                Test { name: "Random Inversions", passes: 4, iters: 16, run: avx512_random_inversions },
                Test { name: "Moving Inversions <<64", passes: 4, iters: 64, run: avx512_moving_inversions_left_64 },
                Test { name: "Moving Inversions 32>>", passes: 4, iters: 32, run: avx512_moving_inversions_right_32 },
                Test { name: "Moving Inversions <<16", passes: 4, iters: 16, run: avx512_moving_inversions_left_16 },
                Test { name: "Moving Inversions 8>>", passes: 4, iters: 8, run: avx512_moving_inversions_right_8 },
                Test { name: "Moving Inversions <<4", passes: 4, iters: 4, run: avx512_moving_inversions_left_4 },
                Test { name: "Moving Saturations 16>>", passes: 8, iters: 16, run: avx512_moving_saturations_right_16 },
                Test { name: "Moving Saturations <<8", passes: 8, iters: 8, run: avx512_moving_saturations_left_8 },
                //FIXME: segfaults Test: Test { name: "Addressing", passes: 4, iters: 16, run: avx512_addressing },
                Test { name: "SGEMM", passes: 1, iters: 32, run: avx512_sgemm },
                Test { name: "Walking-1", passes: 4, iters: 64, run: avx512_walking_1 },
                Test { name: "Walking-0", passes: 4, iters: 64, run: avx512_walking_0 },
                Test { name: "Checkerboard", passes: 4, iters: 1, run: avx512_checkerboard },
                //FIXME: segfaults Test: Test { name: "Address Line Test", passes: 2, iters: 1, run: avx512_address_line_test },
                Test { name: "Anti-Patterns", passes: 8, iters: 34, run: avx512_anti_patterns },
                Test { name: "Inverse Data Patterns", passes: 4, iters: 1, run: avx512_inverse_data_patterns },
            ]
        }
        InstructionSet::AVX2 => {
            unsafe { avx2_tests_init(cpus, errors); }
            vec![
                Test { name: "Basic Tests", passes: 4, iters: 6, run: avx2_basic_tests },
                //FIXME: segfaultsTest: Test { name: "March", passes: 17, iters: 2, run: avx2_march },
                Test { name: "Random Inversions", passes: 4, iters: 16, run: avx2_random_inversions },
                Test { name: "Moving Inversions <<64", passes: 4, iters: 64, run: avx2_moving_inversions_left_64 },
                Test { name: "Moving Inversions 32>>", passes: 4, iters: 32, run: avx2_moving_inversions_right_32 },
                Test { name: "Moving Inversions <<16", passes: 4, iters: 16, run: avx2_moving_inversions_left_16 },
                Test { name: "Moving Inversions 8>>", passes: 4, iters: 8, run: avx2_moving_inversions_right_8 },
                Test { name: "Moving Inversions <<4", passes: 4, iters: 4, run: avx2_moving_inversions_left_4 },
                Test { name: "Moving Saturations 16>>", passes: 8, iters: 16, run: avx2_moving_saturations_right_16 },
                Test { name: "Moving Saturations <<8", passes: 8, iters: 8, run: avx2_moving_saturations_left_8 },
                //FIXME: segfaults Test { name: "Addressing", passes: 2, iters: 16, run: avx2_addressing },
                Test { name: "SGEMM", passes: 1, iters: 32, run: avx2_sgemm },
                Test { name: "Walking-1", passes: 4, iters: 64, run: avx2_walking_1 },
                Test { name: "Walking-0", passes: 4, iters: 64, run: avx2_walking_0 },
                Test { name: "Checkerboard", passes: 4, iters: 1, run: avx2_checkerboard },
                //FIXME: segfaults Test { name: "Address Line Test", passes: 2, iters: 1, run: avx2_address_line_test },
                Test { name: "Anti-Patterns", passes: 8, iters: 34, run: avx2_anti_patterns },
                Test { name: "Inverse Data Patterns", passes: 4, iters: 1, run: avx2_inverse_data_patterns },
            ]
        }
        _ => vec![],
    }
}

