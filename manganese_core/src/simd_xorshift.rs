#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

#[cfg(target_arch = "x86_64")]
pub struct AvxXorshift128PlusKey {
    pub part1: __m256i,
    pub part2: __m256i,
}

#[cfg(target_arch = "x86_64")]
pub struct Avx512Xorshift128PlusKey {
    pub part1: __m512i,
    pub part2: __m512i,
}

#[cfg(target_arch = "x86_64")]
fn xorshift128plus_onkeys(s0: &mut u64, s1: &mut u64) {
    let s1_val = *s0;
    let s0_val = *s1;
    *s0 = s0_val;
    *s1 = s1_val ^ (s1_val << 23) ^ s0_val ^ (s1_val >> 18) ^ (s0_val >> 5);
}

#[cfg(target_arch = "x86_64")]
fn xorshift128plus_jump_onkeys(in1: u64, in2: u64, output1: &mut u64, output2: &mut u64) {
    const JUMP: [u64; 2] = [0x8a5cd789635d2dff, 0x121fd2155c472f96];
    let mut s0 = 0u64;
    let mut s1 = 0u64;
    let mut in1 = in1;
    let mut in2 = in2;
    
    for jump_val in &JUMP {
        for b in 0..64 {
            if (jump_val & (1u64 << b)) != 0 {
                s0 ^= in1;
                s1 ^= in2;
            }
            xorshift128plus_onkeys(&mut in1, &mut in2);
        }
    }
    
    *output1 = s0;
    *output2 = s1;
}

#[cfg(target_arch = "x86_64")]
pub unsafe fn avx_xorshift128plus_init(key1: u64, key2: u64, key: &mut AvxXorshift128PlusKey) {
    let mut s0 = [0u64; 4];
    let mut s1 = [0u64; 4];
    
    s0[0] = key1;
    s1[0] = key2;
    
    xorshift128plus_jump_onkeys(s0[0], s1[0], &mut s0[1], &mut s1[1]);
    xorshift128plus_jump_onkeys(s0[1], s1[1], &mut s0[2], &mut s1[2]);
    xorshift128plus_jump_onkeys(s0[2], s1[2], &mut s0[3], &mut s1[3]);
    
    key.part1 = _mm256_loadu_si256(s0.as_ptr() as *const __m256i);
    key.part2 = _mm256_loadu_si256(s1.as_ptr() as *const __m256i);
}

#[cfg(target_arch = "x86_64")]
pub unsafe fn avx_xorshift128plus(key: &mut AvxXorshift128PlusKey) -> __m256i {
    let _s1 = key.part1;
    let s0 = key.part2;
    key.part1 = key.part2;
    
    let s1_new = _mm256_xor_si256(key.part2, _mm256_slli_epi64(key.part2, 23));
    key.part2 = _mm256_xor_si256(
        _mm256_xor_si256(_mm256_xor_si256(s1_new, s0), _mm256_srli_epi64(s1_new, 18)),
        _mm256_srli_epi64(s0, 5),
    );
    
    _mm256_add_epi64(key.part2, s0)
}

#[cfg(all(target_arch = "x86_64", target_feature = "avx512f"))]
pub unsafe fn avx512_xorshift128plus_init(key1: u64, key2: u64, key: &mut Avx512Xorshift128PlusKey) {
    let mut s0 = [0u64; 8];
    let mut s1 = [0u64; 8];
    
    s0[0] = key1;
    s1[0] = key2;
    
    xorshift128plus_jump_onkeys(s0[0], s1[0], &mut s0[1], &mut s1[1]);
    xorshift128plus_jump_onkeys(s0[1], s1[1], &mut s0[2], &mut s1[2]);
    xorshift128plus_jump_onkeys(s0[2], s1[2], &mut s0[3], &mut s1[3]);
    xorshift128plus_jump_onkeys(s0[3], s1[3], &mut s0[4], &mut s1[4]);
    xorshift128plus_jump_onkeys(s0[4], s1[4], &mut s0[5], &mut s1[5]);
    xorshift128plus_jump_onkeys(s0[5], s1[5], &mut s0[6], &mut s1[6]);
    xorshift128plus_jump_onkeys(s0[6], s1[6], &mut s0[7], &mut s1[7]);
    
    key.part1 = _mm512_loadu_si512(s0.as_ptr() as *const __m512i);
    key.part2 = _mm512_loadu_si512(s1.as_ptr() as *const __m512i);
}

#[cfg(all(target_arch = "x86_64", target_feature = "avx512f"))]
pub unsafe fn avx512_xorshift128plus(key: &mut Avx512Xorshift128PlusKey) -> __m512i {
    let s0 = key.part2;
    key.part1 = key.part2;
    
    let s1_new = _mm512_xor_si512(key.part2, _mm512_slli_epi64::<23>(key.part2));
    key.part2 = _mm512_xor_si512(
        _mm512_xor_si512(_mm512_xor_si512(s1_new, s0), _mm512_srli_epi64::<18>(s1_new)),
        _mm512_srli_epi64::<5>(s0),
    );
    
    _mm512_add_epi64(key.part2, s0)
}

