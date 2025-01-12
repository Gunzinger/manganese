#include "assert.h"
#include "stdatomic.h"
#include "stdint.h"
#include "stdio.h"
#include "sys/sysinfo.h"

#include "immintrin.h"
#include "OpenBLAS/cblas.h"

#include "SIMDxorshift/include/simdxorshift128plus.h"

static uint64_t CPUS;
static _Atomic(uint64_t)* ERRORS;
static avx512_xorshift128plus_key_t rng;

#define FOR_EACH_BLOCK_UP                             \
  _Pragma("omp parallel for schedule(static)")        \
  for(ssize_t i = 0; i < CPUS; i++)                   \
    for(ssize_t j = 0; j < (size / CPUS); j += 64)

#define FOR_EACH_BLOCK_DOWN                               \
  _Pragma("omp parallel for schedule(static)")            \
  for(ssize_t i = CPUS - 1; i >= 0; i--)                  \
    for(ssize_t j = (size / CPUS) - 64; j >= 0; j -= 64)

#define BLOCK_IDX (j + i * (size / CPUS))

void avx512_tests_init(size_t cpus, _Atomic(uint64_t)* errors) {
  CPUS = cpus;
  ERRORS = errors;
  unsigned long long r1 = 0, r2 = 0;
  while(r1 == 0 && r2 == 0) {
    assert(_rdrand64_step(&r1));
    assert(_rdrand64_step(&r2));
  }
  avx512_xorshift128plus_init(r1, r2, &rng);
}

static inline void get(const char* const restrict mem, const size_t idx, const __m512i expected) {
  const __m512i actual = _mm512_load_si512(&mem[idx]);
  _mm_lfence();
  const __mmask64 result = _mm512_cmp_epu8_mask(expected, actual, _MM_CMPINT_NE);

  if(__builtin_expect(result != 0, 0)) {
    const uint64_t error_total = _mm_popcnt_u64(result);
    fprintf(stderr, "%ld errors detected at offset 0x%016lx [error mask: 0x%016llx]\n", error_total, idx, result);
    atomic_fetch_add(ERRORS, error_total);
  }
}

static inline void get_all_up(const char* const restrict mem, const size_t size, const __m512i expected) {
  FOR_EACH_BLOCK_UP {
    get(mem, BLOCK_IDX, expected);
  }
}

static inline void get_all_down(const char* const restrict mem, const size_t size, const __m512i expected) {
  FOR_EACH_BLOCK_DOWN {
    get(mem, BLOCK_IDX, expected);
  }
}

static inline void set(char* const restrict mem, const size_t idx, const __m512i val) {
  _mm512_stream_si512((__m512i*)&mem[idx], val);
}

static inline void set_all_up(char* const restrict mem, const size_t size, const __m512i val) {
  FOR_EACH_BLOCK_UP {
    set(mem, BLOCK_IDX, val);
  }
}

static inline void set_all_down(char* const restrict mem, const size_t size, const __m512i val) {
  FOR_EACH_BLOCK_DOWN {
    set(mem, BLOCK_IDX, val);
  }
}

void avx512_basic_tests(void* const restrict mem, const size_t size) {
    const uint8_t patterns[] = { 0x00, 0xFF, 0x0F, 0xF0, 0x55, 0xAA, };
    for(size_t i = 0; i < sizeof(patterns) / sizeof(uint8_t); i++) {
      const __m512i pattern = _mm512_set1_epi8(patterns[i]);
      set_all_up(mem, size, pattern);
      get_all_up(mem, size, pattern);
      set_all_down(mem, size, pattern);
      get_all_down(mem, size, pattern);
    }
}

void avx512_march(void* const restrict mem, const size_t size) {
  for(size_t _ = 0; _ < 2; _++) {
    const __m512i ones = _mm512_set1_epi8(0xFF);
    const __m512i zeroes = _mm512_set1_epi8(0x00);
    FOR_EACH_BLOCK_DOWN {
      set(mem, j + i * (size / CPUS), zeroes);
    }
    FOR_EACH_BLOCK_UP {
      get(mem, BLOCK_IDX, zeroes);
      set(mem, BLOCK_IDX, ones);
      get(mem, BLOCK_IDX, ones);
      set(mem, BLOCK_IDX, zeroes);
      get(mem, BLOCK_IDX, zeroes);
      set(mem, BLOCK_IDX, ones);
    }
    FOR_EACH_BLOCK_UP {
      get(mem, BLOCK_IDX, ones);
      set(mem, BLOCK_IDX, zeroes);
      set(mem, BLOCK_IDX, ones);
    }
    FOR_EACH_BLOCK_DOWN {
      get(mem, BLOCK_IDX, ones);
      set(mem, BLOCK_IDX, zeroes);
      set(mem, BLOCK_IDX, ones);
      set(mem, BLOCK_IDX, zeroes);
    }
    FOR_EACH_BLOCK_DOWN {
      get(mem, BLOCK_IDX, zeroes);
      set(mem, BLOCK_IDX, ones);
      set(mem, BLOCK_IDX, zeroes);
    }
  }
}

void avx512_random_inversions(void* const restrict mem, const size_t size) {
  for(size_t i = 0; i < 16; i++) {
    const __m512i pattern = avx512_xorshift128plus(&rng);
    set_all_up(mem, size, pattern);
    get_all_up(mem, size, pattern);
    const __m512i not_pattern = _mm512_xor_epi64(pattern, _mm512_set1_epi8(0xFF));
    set_all_up(mem, size, not_pattern);
    get_all_up(mem, size, not_pattern);
  }
}

static void moving_inversions_template(void* const restrict mem, const size_t size, const size_t iters, __m512i (*shift)(__m512i, unsigned int), const __m512i initial) {
  for(size_t i = 0; i < iters; i++) {
    const __m512i pattern = shift(initial, i);
    set_all_up(mem, size, pattern);
    get_all_up(mem, size, pattern);
    const __m512i not_pattern = _mm512_xor_epi64(pattern, _mm512_set1_epi8(0xFF));
    set_all_up(mem, size, not_pattern);
    get_all_up(mem, size, not_pattern);
  }
}

void avx512_moving_inversions_left_64(void* const restrict mem, const size_t size) {
  return moving_inversions_template(mem, size, 64, _mm512_slli_epi64, _mm512_set1_epi64(0x0000000000000001));
}

void avx512_moving_inversions_right_32(void* const restrict mem, const size_t size) {
  return moving_inversions_template(mem, size, 32, _mm512_srli_epi64, _mm512_set1_epi32(0x80000000));
}

void avx512_moving_inversions_left_16(void* const restrict mem, const size_t size) {
  return moving_inversions_template(mem, size, 16, _mm512_slli_epi64, _mm512_set1_epi16(0x0001));
}

void avx512_moving_inversions_right_8(void* const restrict mem, const size_t size) {
  return moving_inversions_template(mem, size, 8, _mm512_srli_epi64, _mm512_set1_epi8(0x80));
}

void avx512_moving_inversions_left_4(void* const restrict mem, const size_t size) {
  return moving_inversions_template(mem, size, 4, _mm512_slli_epi64, _mm512_set1_epi8(0x11));
}

void avx512_moving_saturations_right_16(void* const restrict mem, const size_t size) {
    for(size_t i = 0; i < 16; i++) {
      const __m512i pattern = _mm512_srli_epi16(_mm512_set1_epi16(0x8000), i);
      set_all_up(mem, size, pattern);
      get_all_up(mem, size, pattern);
      const __m512i zeroes = _mm512_set1_epi8(0x00);
      set_all_up(mem, size, zeroes);
      get_all_up(mem, size, zeroes);
      set_all_up(mem, size, pattern);
      get_all_up(mem, size, pattern);
      const __m512i ones = _mm512_set1_epi8(0xFF);
      set_all_up(mem, size, ones);
      get_all_up(mem, size, ones);
    }
}

void avx512_moving_saturations_left_8(void* const restrict mem, const size_t size) {
    for(size_t i = 0; i < 8; i++) {
      const __m512i pattern = _mm512_srli_epi16(_mm512_set1_epi16(0x01), i);
      set_all_up(mem, size, pattern);
      get_all_up(mem, size, pattern);
      const __m512i zeroes = _mm512_set1_epi8(0x00);
      set_all_up(mem, size, zeroes);
      get_all_up(mem, size, zeroes);
      set_all_up(mem, size, pattern);
      get_all_up(mem, size, pattern);
      const __m512i ones = _mm512_set1_epi8(0xFF);
      set_all_up(mem, size, ones);
      get_all_up(mem, size, ones);
    }
}

void avx512_addressing(void* const restrict mem, const size_t size) {
  for(size_t _ = 0; _ < 16; _++) {
    __m512i increasing = _mm512_set_epi64(56, 48, 40, 32, 24, 16, 8, 0);

    FOR_EACH_BLOCK_UP {
      set(mem, BLOCK_IDX, _mm512_add_epi64(_mm512_set1_epi64(BLOCK_IDX), increasing));
    }
    FOR_EACH_BLOCK_UP {
      get(mem, BLOCK_IDX, _mm512_add_epi64(_mm512_set1_epi64(BLOCK_IDX), increasing));
    }
    FOR_EACH_BLOCK_DOWN {
      set(mem, BLOCK_IDX, _mm512_add_epi64(_mm512_set1_epi64(BLOCK_IDX), increasing));
    }
    FOR_EACH_BLOCK_DOWN {
      get(mem, BLOCK_IDX, _mm512_add_epi64(_mm512_set1_epi64(BLOCK_IDX), increasing));
    }
  }
}

void avx512_sgemm(char* const restrict mem, const size_t size) {
  const __m512 zeroes = _mm512_set1_ps(0.0f);
  set_all_down(mem, size, (__m512i) zeroes);
  for(ssize_t _ = 0; _ < 32; _++) {
    _Pragma("omp parallel for schedule(static)")
    for(ssize_t i = 0; i < CPUS; i++) {
      for(ssize_t j = 64 * 64 * 4 * 2; j < (size / CPUS); j += 64 * 64 * 4) {
        float* const a = (float*) &mem[BLOCK_IDX - 64 * 64 * 4 * 2];
        float* const b = (float*) &mem[BLOCK_IDX - 64 * 64 * 4 * 1];
        float* const c = (float*) &mem[BLOCK_IDX - 64 * 64 * 4 * 0];
        cblas_sgemm(CblasRowMajor, CblasNoTrans, CblasNoTrans, 64, 64, 64, 1.0, a, 64, b, 64, 0.0, c, 64);
        for(ssize_t k = 0; k < 64 * 64 * 4; k += 64) {
          _mm_clflushopt(&mem[BLOCK_IDX + k]);
        }
        _mm_sfence();
      }
    }
  }
  get_all_up(mem, size, (__m512i) zeroes);
}
