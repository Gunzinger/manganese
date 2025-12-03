#[cfg(windows)]
mod windows {
    use winapi::um::sysinfoapi::{GlobalMemoryStatusEx, GetSystemInfo, MEMORYSTATUSEX, SYSTEM_INFO};
    use winapi::um::memoryapi::VirtualLock;
    use std::mem;
    use std::mem::zeroed;

    pub struct SysInfo {
        pub totalram: usize,
        pub freeram: usize,
        pub sharedram: usize,
        pub bufferram: usize,
        pub totalswap: usize,
        pub freeswap: usize,
        pub procs: u16,
        pub totalhigh: usize,
        pub freehigh: usize,
        pub mem_unit: u32,
    }

    pub fn sysinfo() -> SysInfo {
        let mut mem_status = MEMORYSTATUSEX {
            dwLength: mem::size_of::<MEMORYSTATUSEX>() as u32,
            dwMemoryLoad: 0,
            ullTotalPhys: 0,
            ullAvailPhys: 0,
            ullTotalPageFile: 0,
            ullAvailPageFile: 0,
            ullTotalVirtual: 0,
            ullAvailVirtual: 0,
            ullAvailExtendedVirtual: 0,
        };
        
        unsafe {
            GlobalMemoryStatusEx(&mut mem_status);
        }

        let mut sys_info: SYSTEM_INFO = unsafe { zeroed() };
        //let mut sys_info = SYSTEM_INFO::default();
        unsafe {
            GetSystemInfo(&mut sys_info);
        }
        
        SysInfo {
            totalram: mem_status.ullTotalPhys as usize,
            freeram: mem_status.ullAvailPhys as usize,
            sharedram: 0,
            bufferram: 0,
            totalswap: mem_status.ullTotalPageFile as usize,
            freeswap: mem_status.ullAvailPageFile as usize,
            procs: sys_info.dwNumberOfProcessors as u16,
            totalhigh: 0,
            freehigh: 0,
            mem_unit: 1,
        }
    }

    pub fn getpagesize() -> usize {
        unsafe {
            let mut sys_info: SYSTEM_INFO = mem::zeroed();
            GetSystemInfo(&mut sys_info);
            sys_info.dwPageSize as usize
        }
    }

    pub unsafe fn mlock(addr: *mut u8, len: usize) -> i32 {
        if VirtualLock(addr as *mut _, len) != 0 {
            0
        } else {
            -1
        }
    }

    pub unsafe fn aligned_alloc(alignment: usize, size: usize) -> *mut u8 {
        // VirtualAlloc provides page-aligned allocations (minimum 4KB, allocation granularity 64KB)
        // Since our alignment requirement is cpu_count * pagesize, VirtualAlloc's natural alignment suffices
        use winapi::um::memoryapi::VirtualAlloc;
        use winapi::um::winnt::{MEM_COMMIT, MEM_RESERVE, PAGE_READWRITE};
        
        let _ = alignment;  // VirtualAlloc is inherently aligned to page boundaries
        
        VirtualAlloc(
            std::ptr::null_mut(),
            size,
            MEM_COMMIT | MEM_RESERVE,
            PAGE_READWRITE,
        ) as *mut u8
    }

    pub unsafe fn aligned_free(ptr: *mut u8) {
        use winapi::um::memoryapi::VirtualFree;
        use winapi::um::winnt::MEM_RELEASE;
        
        if !ptr.is_null() {
            // VirtualFree with MEM_RELEASE requires the base address
            // For page-aligned allocations, we need to free the original address
            // Since we can't track the original with this API, we'll use a simpler approach:
            // Just allocate naturally aligned memory from VirtualAlloc
            VirtualFree(ptr as *mut _, 0, MEM_RELEASE);
        }
    }
}

#[cfg(not(windows))]
mod unix {
    #[cfg(target_os = "linux")]
    use libc::sysinfo as sysinfo_struct;

    pub struct SysInfo {
        pub totalram: usize,
        pub freeram: usize,
        pub sharedram: usize,
        pub bufferram: usize,
        pub totalswap: usize,
        pub freeswap: usize,
        pub procs: u16,
        pub totalhigh: usize,
        pub freehigh: usize,
        pub mem_unit: u32,
    }

    #[cfg(target_os = "linux")]
    pub fn sysinfo() -> SysInfo {
        let mut info = sysinfo_struct {
            uptime: 0,
            loads: [0; 3],
            totalram: 0,
            freeram: 0,
            sharedram: 0,
            bufferram: 0,
            totalswap: 0,
            freeswap: 0,
            procs: 0,
            totalhigh: 0,
            freehigh: 0,
            mem_unit: 0,
            pad: [0; 0],
            __reserved: [0; 8],
        };
        
        unsafe {
            libc::sysinfo(&mut info);
        }
        
        SysInfo {
            totalram: (info.totalram as usize) * (info.mem_unit as usize),
            freeram: (info.freeram as usize) * (info.mem_unit as usize),
            sharedram: (info.sharedram as usize) * (info.mem_unit as usize),
            bufferram: (info.bufferram as usize) * (info.mem_unit as usize),
            totalswap: (info.totalswap as usize) * (info.mem_unit as usize),
            freeswap: (info.freeswap as usize) * (info.mem_unit as usize),
            procs: info.procs as u16,
            totalhigh: (info.totalhigh as usize) * (info.mem_unit as usize),
            freehigh: (info.freehigh as usize) * (info.mem_unit as usize),
            mem_unit: info.mem_unit as u32,
        }
    }

    #[cfg(not(target_os = "linux"))]
    pub fn sysinfo() -> SysInfo {
        // Fallback for non-Linux Unix systems
        SysInfo {
            totalram: 0,
            freeram: 0,
            sharedram: 0,
            bufferram: 0,
            totalswap: 0,
            freeswap: 0,
            procs: num_cpus::get() as u16,
            totalhigh: 0,
            freehigh: 0,
            mem_unit: 1,
        }
    }

    pub fn getpagesize() -> usize {
        unsafe { libc::getpagesize() as usize }
    }

    pub unsafe fn mlock(addr: *mut u8, len: usize) -> i32 {
        libc::mlock(addr as *const _, len)
    }

    pub unsafe fn aligned_alloc(alignment: usize, size: usize) -> *mut u8 {
        libc::aligned_alloc(alignment, size) as *mut u8
    }

    pub unsafe fn aligned_free(ptr: *mut u8) {
        libc::free(ptr as *mut _);
    }
}

#[cfg(windows)]
pub use windows::*;

#[cfg(not(windows))]
pub use unix::*;

