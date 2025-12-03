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

