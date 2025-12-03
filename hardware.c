#ifndef _WIN32
#define _GNU_SOURCE // for sched_getaffinity
#endif

#include "assert.h"
#include "stdint.h"
#include "stdio.h"
#include "stdlib.h"
#include "stdbool.h"

#ifdef PLATFORM_WINDOWS
#include <windows.h>
#include <intrin.h>
#ifdef _OPENMP
#include <omp.h>
#endif
// CPUID feature bits (from CPUID leaf 7, EBX register)
#define bit_AVX2     (1 << 5)   // Bit 5: AVX2
#define bit_AVX512F  (1 << 16)  // Bit 16: AVX-512 Foundation
#define bit_AVX512BW (1 << 30)  // Bit 30: AVX-512 Byte and Word Instructions
#else
#include "cpuid.h"
#include "glob.h"
#include "sys/stat.h"
#include "unistd.h"
#include "omp.h"
#include "sched.h"
// CPUID feature bits (from CPUID leaf 7, EBX register)
#define bit_AVX2     (1 << 5)   // Bit 5: AVX2
#define bit_AVX512F  (1 << 16)  // Bit 16: AVX-512 Foundation
#define bit_AVX512BW (1 << 30)  // Bit 30: AVX-512 Byte and Word Instructions
#endif

#include "platform.h"
#include "hardware.h"

const uint64_t HARDWARE_HAS_SSE = 0x00;
const uint64_t HARDWARE_HAS_AVX2 = 0x01;
const uint64_t HARDWARE_HAS_AVX512 = 0x02;

uint64_t hardware_is_needlessly_disabled() {
#ifdef PLATFORM_WINDOWS
  int cpuInfo[4] = {0};
  __cpuid(cpuInfo, 0x01);
  int a = cpuInfo[0], b = cpuInfo[1], c = cpuInfo[2], d = cpuInfo[3];
  while(a == 0) {
    __cpuid(cpuInfo, 0x01);
    a = cpuInfo[0];
    b = cpuInfo[1];
    c = cpuInfo[2];
    d = cpuInfo[3];
  }
#else
  int a = 0, b = 0, c = 0, d = 0;
  while(a == 0) {
    __cpuid(0x01, a, b, c, d);
  }
#endif
  const int family = a >> 8 & (int)0x0F;
  const int model = (a >> 4 & (int)0x0F) | (a >> 12) & (int)0xF0;
  const int stepping = a & 0x0F;
  return (family == 6 && model == 151);
}

uint64_t hardware_instruction_set() {
#ifdef PLATFORM_WINDOWS
  int cpuInfo[4] = {0};
  __cpuid(cpuInfo, 0x07);
  int a = cpuInfo[0], b = cpuInfo[1], c = cpuInfo[2], d = cpuInfo[3];
  while(b == 0) {
    __cpuid(cpuInfo, 0x07);
    a = cpuInfo[0];
    b = cpuInfo[1];
    c = cpuInfo[2];
    d = cpuInfo[3];
  }
#else
  int a = 0, b = 0, c = 0, d = 0;
  while(b == 0) {
    __cpuid(0x07, a, b, c, d);
  }
#endif

  if(b & bit_AVX512F && b & bit_AVX512BW) {
    return HARDWARE_HAS_AVX512;
  } else if((b & bit_AVX2)) {
    return HARDWARE_HAS_AVX2;
  } else {
    return HARDWARE_HAS_SSE;
    exit(-1);
  }
}

uint64_t hardware_ram_speed(bool configured) {
#ifdef PLATFORM_WINDOWS
  // Windows doesn't have DMI tables accessible the same way
  // Return 0 to indicate speed is unknown
  (void)configured;
  return (uint64_t) 0u;
#else
  glob_t dmiglob;
  uint16_t ram_speed;
  switch (glob("/sys/firmware/dmi/entries/17-*/raw", 0, NULL, &dmiglob)) {
    case GLOB_NOSPACE:
    case GLOB_ABORTED:
    case GLOB_NOMATCH:
      return (uint64_t) 0u;
  }
  for(size_t i = 0; i < dmiglob.gl_pathc; i++) {
    FILE* file = fopen(dmiglob.gl_pathv[i], "r");
    if(file == NULL) {
      return 0;
    }
    struct stat size;
    fseek(file, configured ? 0x20 : 0x15, SEEK_SET);
    while(!fread((void*) &ram_speed, sizeof(uint16_t), 1, file));
    fclose(file);
    if(ram_speed) {
      break;
    }
  }

  return (uint64_t) ram_speed;
#endif
}

uint64_t hardware_cpu_count() {
#ifdef PLATFORM_WINDOWS
  SYSTEM_INFO si;
  GetSystemInfo(&si);
  #ifdef _OPENMP
  int omp_thread_count;
  #pragma omp parallel
  {
    #pragma omp single
    omp_thread_count = omp_get_num_threads();
  }
  if((DWORD)omp_thread_count < si.dwNumberOfProcessors) {
    omp_set_num_threads(si.dwNumberOfProcessors);
    return si.dwNumberOfProcessors;
  } else {
    return omp_thread_count;
  }
  #else
  return si.dwNumberOfProcessors;
  #endif
#else
  cpu_set_t cpuset;
  size_t omp_thread_count;

  sched_getaffinity((pid_t)getpid(), sizeof(cpu_set_t), &cpuset);

  #ifndef _OPENMP
  return CPU_COUNT(&cpuset);
  #endif

  #pragma omp parallel
  {
    #pragma omp single
    omp_thread_count = omp_get_num_threads();
  }

  if(CPU_COUNT(&cpuset) < omp_thread_count){
    omp_set_num_threads(CPU_COUNT(&cpuset));
    return CPU_COUNT(&cpuset);
  } else {
    return omp_thread_count;
  }
#endif
}
