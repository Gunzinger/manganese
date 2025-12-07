use std::collections::HashMap;
use std::sync::atomic::AtomicU64;
use log::error;
use crate::hardware::InstructionSet;
use crate::tests_avx2::*;
use crate::tests_avx512::*;

#[derive(Clone)]
pub struct TestDefinition {
    pub name: &'static str,
    pub passes: usize,
    pub iters: usize,
    pub run: unsafe fn(*mut u8, usize),
    pub loops: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TestKind {
    BasicTests,
    RandomInversions,
    MovingInversionsLeft64,
    MovingInversionsRight32,
    MovingInversionsLeft16,
    MovingInversionsRight8,
    MovingInversionsLeft4,
    MovingSaturationsRight16,
    MovingSaturationsLeft8,
    Walking1,
    Walking0,
    Checkerboard,
    AntiPatterns,
    InverseDataPatterns,
}

impl TestKind {
    pub fn parse(s: &str) -> Option<Self> {
        use TestKind::*;
        Some(match s {
            "basic_tests" => BasicTests,
            "random_inversions" => RandomInversions,
            "moving_inversions_left_64" => MovingInversionsLeft64,
            "moving_inversions_right_32" => MovingInversionsRight32,
            "moving_inversions_left_16" => MovingInversionsLeft16,
            "moving_inversions_right_8"  => MovingInversionsRight8,
            "moving_inversions_left_4"  => MovingInversionsLeft4,
            "moving_saturations_right_16" => MovingSaturationsRight16,
            "moving_saturations_left_8" => MovingSaturationsLeft8,
            "walking1" => Walking1,
            "walking0" => Walking0,
            "checkerboard" => Checkerboard,
            "anti_patterns" => AntiPatterns,
            "inverse_data_patterns" => InverseDataPatterns,
            _ => return None,
        })
    }
}

pub fn avx2_definitions() -> HashMap<TestKind, TestDefinition> {
    //FIXME: segfaultsTest: Test { name: "March", passes: 17, iters: 2, run: avx2_march },
    //FIXME: segfaults Test { name: "Addressing", passes: 2, iters: 16, run: avx2_addressing },
    //FIXME: no openBLAS / other BLAS framework integration Test { name: "SGEMM", passes: 1, iters: 32, run: avx2_sgemm },
    //FIXME: segfaults Test { name: "Address Line Test", passes: 2, iters: 1, run: avx2_address_line_test },
    use TestKind::*;
    HashMap::from([
        (BasicTests, TestDefinition {
            name: "basic_tests",
            passes: 4,
            iters: 6,
            run: avx2_basic_tests,
            loops: 1,
        }),
        (RandomInversions, TestDefinition {
            name: "random_inversions",
            passes: 4,
            iters: 16,
            run: avx2_random_inversions,
            loops: 1,
        }),
        (MovingInversionsLeft64, TestDefinition {
            name: "moving_inversions_left_64",
            passes: 4,
            iters: 64,
            run: avx2_moving_inversions_left_64,
            loops: 1,
        }),
        (MovingInversionsRight32, TestDefinition {
            name: "moving_inversions_right_32",
            passes: 4,
            iters: 32,
            run: avx2_moving_inversions_right_32,
            loops: 1,
        }),
        (MovingInversionsLeft16, TestDefinition {
            name: "moving_inversions_left_16",
            passes: 4,
            iters: 16,
            run: avx2_moving_inversions_left_16,
            loops: 1,
        }),
        (MovingInversionsRight8, TestDefinition {
            name: "moving_inversions_right_8",
            passes: 4,
            iters: 8,
            run: avx2_moving_inversions_right_8,
            loops: 1,
        }),
        (MovingInversionsLeft4, TestDefinition {
            name: "moving_inversions_left_4",
            passes: 4,
            iters: 4,
            run: avx2_moving_inversions_left_4,
            loops: 1,
        }),
        (MovingSaturationsRight16, TestDefinition {
            name: "moving_saturations_right_16",
            passes: 8,
            iters: 16,
            run: avx2_moving_saturations_right_16,
            loops: 1,
        }),
        (MovingSaturationsLeft8, TestDefinition {
            name: "moving_saturations_left_8",
            passes: 8,
            iters: 8,
            run: avx2_moving_saturations_left_8,
            loops: 1,
        }),
        (Walking1, TestDefinition {
            name: "walking1",
            passes: 4,
            iters: 64,
            run: avx2_walking_1,
            loops: 1,
        }),
        (Walking0, TestDefinition {
            name: "walking0",
            passes: 4,
            iters: 64,
            run: avx2_walking_0,
            loops: 1,
        }),
        (Checkerboard, TestDefinition {
            name: "checkerboard",
            passes: 4,
            iters: 1,
            run: avx2_checkerboard,
            loops: 8,
        }),
        (AntiPatterns, TestDefinition {
            name: "anti_patterns",
            passes: 8,
            iters: 34,
            run: avx2_anti_patterns,
            loops: 1,
        }),
        (InverseDataPatterns, TestDefinition {
             name: "inverse_data_patterns",
             passes: 4,
             iters: 14,
             run: avx2_inverse_data_patterns,
             loops: 1,
         }),
    ])
}

pub fn avx512_definitions() -> HashMap<TestKind, TestDefinition> {
    //FIXME: segfaults Test: Test { name: "March", passes: 17, iters: 2, run: avx512_march },
    //FIXME: segfaults Test: Test { name: "Addressing", passes: 4, iters: 16, run: avx512_addressing },
    //FIXME: no openBLAS / other BLAS framework integration Test { name: "SGEMM", passes: 1, iters: 32, run: avx512_sgemm },
    //FIXME: segfaults Test: Test { name: "Address Line Test", passes: 2, iters: 1, run: avx512_address_line_test },
    use TestKind::*;
    HashMap::from([
        (BasicTests, TestDefinition {
            name: "basic_tests",
            passes: 4,
            iters: 6,
            run: avx512_basic_tests,
            loops: 1,
        }),
        (RandomInversions, TestDefinition {
            name: "random_inversions",
            passes: 4,
            iters: 16,
            run: avx512_random_inversions,
            loops: 1,
        }),
        (MovingInversionsLeft64, TestDefinition {
            name: "moving_inversions_left_64",
            passes: 4,
            iters: 64,
            run: avx512_moving_inversions_left_64,
            loops: 1,
        }),
        (MovingInversionsRight32, TestDefinition {
            name: "moving_inversions_right_32",
            passes: 4,
            iters: 32,
            run: avx512_moving_inversions_right_32,
            loops: 1,
        }),
        (MovingInversionsLeft16, TestDefinition {
            name: "moving_inversions_left_16",
            passes: 4,
            iters: 16,
            run: avx512_moving_inversions_left_16,
            loops: 1,
        }),
        (MovingInversionsRight8, TestDefinition {
            name: "moving_inversions_right_8",
            passes: 4,
            iters: 8,
            run: avx512_moving_inversions_right_8,
            loops: 1,
        }),
        (MovingInversionsLeft4, TestDefinition {
            name: "moving_inversions_left_4",
            passes: 4,
            iters: 4,
            run: avx512_moving_inversions_left_4,
            loops: 1,
        }),
        (MovingSaturationsRight16, TestDefinition {
            name: "moving_saturations_right_16",
            passes: 8,
            iters: 16,
            run: avx512_moving_saturations_right_16,
            loops: 1,
        }),
        (MovingSaturationsLeft8, TestDefinition {
            name: "moving_saturations_left_8",
            passes: 8,
            iters: 8,
            run: avx512_moving_saturations_left_8,
            loops: 1,
        }),
        (Walking1, TestDefinition {
            name: "walking1",
            passes: 4,
            iters: 64,
            run: avx512_walking_1,
            loops: 1,
        }),
        (Walking0, TestDefinition {
            name: "walking0",
            passes: 4,
            iters: 64,
            run: avx512_walking_0,
            loops: 1,
        }),
        (Checkerboard, TestDefinition {
            name: "checkerboard",
            passes: 4,
            iters: 1,
            run: avx512_checkerboard,
            loops: 8,
        }),
        (AntiPatterns, TestDefinition {
            name: "anti_patterns",
            passes: 8,
            iters: 34,
            run: avx512_anti_patterns,
            loops: 1,
        }),
        (InverseDataPatterns, TestDefinition {
            name: "inverse_data_patterns",
            passes: 4,
            iters: 14,
            run: avx512_inverse_data_patterns,
            loops: 1,
        }),
    ])
}

#[allow(dead_code)]
pub fn get_test_definitions_for_isa(isa: InstructionSet) -> HashMap<TestKind, TestDefinition> {
    match isa {
        InstructionSet::AVX512 => {
            avx512_definitions()
        }
        InstructionSet::AVX2 => {
            avx2_definitions()
        },
        InstructionSet::SSE => HashMap::new(),
    }
}

pub fn tests_init(cpus: usize, errors: &'static AtomicU64, isa: InstructionSet) {
    match isa {
        InstructionSet::AVX512 => {
            unsafe { avx512_tests_init(cpus, errors); }
        }
        InstructionSet::AVX2 => {
            unsafe { avx2_tests_init(cpus, errors); }
        },
        InstructionSet::SSE => error!("Unsupported instruction set: SSE"),
    }
}

