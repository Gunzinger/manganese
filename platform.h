#ifndef PLATFORM_H
#define PLATFORM_H

#ifdef _WIN32
#define PLATFORM_WINDOWS
#include <windows.h>
#include "mman.h"  // mman-win32
#else
#define PLATFORM_UNIX
#include <sys/mman.h>
#include <unistd.h>
#include <sys/sysinfo.h>
#endif

// Platform-specific page size
#ifdef PLATFORM_WINDOWS
static inline size_t platform_getpagesize(void) {
    SYSTEM_INFO si;
    GetSystemInfo(&si);
    return si.dwPageSize;
}
#define getpagesize() platform_getpagesize()
#else
// Use standard getpagesize() on Unix
#include <unistd.h>
#endif

// Platform-specific memory locking
#ifdef PLATFORM_WINDOWS
static inline int platform_mlock(void *addr, size_t len) {
    return VirtualLock(addr, len) ? 0 : -1;
}
#define mlock(addr, len) platform_mlock(addr, len)
#else
// Use standard mlock() on Unix
#endif

// Platform-specific aligned_alloc
#ifdef PLATFORM_WINDOWS
#include <malloc.h>
static inline void* platform_aligned_alloc(size_t alignment, size_t size) {
    return _aligned_malloc(size, alignment);
}
#define aligned_alloc(alignment, size) platform_aligned_alloc(alignment, size)
static inline void platform_aligned_free(void* ptr) {
    _aligned_free(ptr);
}
#define aligned_free(ptr) platform_aligned_free(ptr)
#else
// Use standard aligned_alloc() on Unix (C11)
#define aligned_free(ptr) free(ptr)
#endif

// Platform-specific clock definitions
#ifdef PLATFORM_WINDOWS
#include <time.h>
// Define timespec if not available (older MinGW versions)
#ifndef _TIMESPEC_DEFINED
#define _TIMESPEC_DEFINED
struct timespec {
    time_t tv_sec;
    long tv_nsec;
};
#endif
// CLOCK_MONOTONIC_RAW is not available on Windows, use CLOCK_MONOTONIC equivalent
// For Windows, we'll use QueryPerformanceCounter in clock_gettime wrapper
#ifndef CLOCK_MONOTONIC
#define CLOCK_MONOTONIC 1
#endif
#ifndef CLOCK_MONOTONIC_RAW
#define CLOCK_MONOTONIC_RAW CLOCK_MONOTONIC
#endif

// Windows implementation of clock_gettime for CLOCK_MONOTONIC
static inline int platform_clock_gettime(int clock_id, struct timespec *tp) {
    (void)clock_id; // Ignore clock_id on Windows, always use high-res timer
    LARGE_INTEGER frequency, counter;
    QueryPerformanceFrequency(&frequency);
    QueryPerformanceCounter(&counter);
    
    tp->tv_sec = counter.QuadPart / frequency.QuadPart;
    tp->tv_nsec = (long)(((counter.QuadPart % frequency.QuadPart) * 1000000000LL) / frequency.QuadPart);
    return 0;
}
#define clock_gettime(clock_id, tp) platform_clock_gettime(clock_id, tp)
#else
// Use standard clock_gettime() on Unix
#include <time.h>
#endif

// Platform-specific sysinfo
#ifdef PLATFORM_WINDOWS
struct sysinfo {
    unsigned long totalram;
    unsigned long freeram;
    unsigned long sharedram;
    unsigned long bufferram;
    unsigned long totalswap;
    unsigned long freeswap;
    unsigned short procs;
    unsigned long totalhigh;
    unsigned long freehigh;
    unsigned int mem_unit;
    char _f[20-2*sizeof(long)-sizeof(int)];
};

static inline int sysinfo(struct sysinfo *info) {
    MEMORYSTATUSEX memInfo;
    memInfo.dwLength = sizeof(MEMORYSTATUSEX);
    GlobalMemoryStatusEx(&memInfo);
    
    SYSTEM_INFO si;
    GetSystemInfo(&si);
    
    info->totalram = (unsigned long)memInfo.ullTotalPhys;
    info->freeram = (unsigned long)memInfo.ullAvailPhys;
    info->sharedram = 0;
    info->bufferram = 0;
    info->totalswap = (unsigned long)memInfo.ullTotalPageFile;
    info->freeswap = (unsigned long)memInfo.ullAvailPageFile;
    info->procs = (unsigned short)si.dwNumberOfProcessors;
    info->totalhigh = 0;
    info->freehigh = 0;
    info->mem_unit = 1;
    
    return 0;
}
#else
// Use standard sysinfo() on Unix
#include <sys/sysinfo.h>
#endif

#endif // PLATFORM_H

