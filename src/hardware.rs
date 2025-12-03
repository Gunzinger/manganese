// No imports needed here - cpuid handled via module

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstructionSet {
    SSE,
    AVX2,
    AVX512,
}

// CPUID feature bit definitions
// CPUID leaf 0x07, subleaf 0, EBX register
const BIT_AVX2: u32 = 1 << 5;       // Bit 5: AVX2 (NOT in leaf 0x01!)
const BIT_AVX512F: u32 = 1 << 16;   // Bit 16: AVX-512 Foundation
const BIT_AVX512BW: u32 = 1 << 30;  // Bit 30: AVX-512 Byte and Word

pub fn hardware_is_needlessly_disabled() -> bool {
    #[cfg(target_arch = "x86_64")]
    {
        unsafe {
            let mut cpu_info = [0u32; 4];
            cpuid::cpuid_count(0x01, 0, &mut cpu_info);
            
            let family = (cpu_info[0] >> 8) & 0x0F;
            let model = ((cpu_info[0] >> 4) & 0x0F) | ((cpu_info[0] >> 12) & 0xF0);
            
            family == 6 && model == 151
        }
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        false
    }
}

pub fn hardware_instruction_set() -> InstructionSet {
    #[cfg(target_arch = "x86_64")]
    {
        unsafe {
            // Query CPUID leaf 0x07 (Extended Features), subleaf 0
            // This leaf contains AVX2 and AVX-512 feature bits
            // Note: AVX (original) is in leaf 0x01, but AVX2 is in leaf 0x07!
            let mut cpu_info = [0u32; 4];
            cpuid::cpuid_count(0x07, 0, &mut cpu_info);
            
            let ebx = cpu_info[1];  // EBX contains the feature flags
            
            // Check for AVX-512 first (requires both Foundation and Byte/Word)
            if (ebx & BIT_AVX512F) != 0 && (ebx & BIT_AVX512BW) != 0 {
                InstructionSet::AVX512
            } else if (ebx & BIT_AVX2) != 0 {
                // AVX2 is in CPUID.07H:EBX[bit 5], not in CPUID.01H!
                InstructionSet::AVX2
            } else {
                InstructionSet::SSE
            }
        }
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        InstructionSet::SSE
    }
}

#[cfg(target_os = "linux")]
pub fn hardware_ram_speed(configured: bool) -> u64 {
    use std::fs;
    use std::io::{Read, Seek, SeekFrom};
    use glob::glob;

    let glob_pattern = "/sys/firmware/dmi/entries/17-*/raw";
    let entries = match glob(glob_pattern) {
        Ok(entries) => entries,
        Err(_) => return 0,
    };

    for entry in entries.flatten() {
        if let Ok(mut file) = fs::File::open(&entry) {
            let offset = if configured { 0x20 } else { 0x15 };
            if file.seek(SeekFrom::Start(offset as u64)).is_ok() {
                let mut buf = [0u8; 2];
                if file.read_exact(&mut buf).is_ok() {
                    let ram_speed = u16::from_le_bytes(buf);
                    if ram_speed > 0 {
                        return ram_speed as u64;
                    }
                }
            }
        }
    }

    0
}

#[cfg(target_os = "windows")]
pub fn hardware_ram_speed(configured: bool) -> u64 {
    use windows::Win32::System::SystemInformation::{GetSystemFirmwareTable, RSMB};

    // 'RSMB' signature
    let provider = RSMB;
    //let provider_ACPI = ACPI;

    // Step 1: get required size
    let size = unsafe { GetSystemFirmwareTable(provider, 0, None) };
    //let size_ACPI = unsafe { GetSystemFirmwareTable(provider_ACPI, 0, None) };
    if size == 0 {
        eprintln!("Failed to get system firmware table (1) RSMB: {}", size);
        return 0;
    }
    //eprintln!("got system firmware table (1) RSMB: {}, ACPI: {}, FIRM: {}", size, size_ACPI, size_FIRM);

    // Step 2: allocate buffer
    let mut buffer = vec![0u8; size as usize];

    // Step 3: retrieve table
    let ret = unsafe { GetSystemFirmwareTable(provider, 0, Some(&mut buffer[..])) };
    if ret != size {
        eprintln!("Failed to get system firmware table (3)");
        return 0;
    }

    // Step 4: parse Type 17 entries
    let mut offset = 0usize;
    let mut max_speed = 0u16;

    while offset + 4 <= buffer.len() {
        let entry_type = buffer[offset];
        let length = buffer[offset + 1] as usize;

        //eprintln!("RSMBinfo @ {} (/{}): {} / {}", offset, buffer.len(), entry_type, length);
        // see e.g. smbioslib https://github.com/jrgerber/smbios-lib/blob/942b892559b88e921f986f05b00641594c518d73/src/structs/types/memory_device.rs#L176

        if length < 4 {
            if length == 0 {
                break;
            }
            // Move to next entry: SMBIOS entries are followed by double-null terminated strings
            let mut next = offset + length;
            while next + 1 < buffer.len() && !(buffer[next] == 0 && buffer[next + 1] == 0) {
                next += 1;
            }
            offset = next + 2; // skip double null
            continue;
        }
        if offset + length > buffer.len() {
            break;
        }

        // Type 17 = Memory Device
        if entry_type == 17 {
            if configured {
                if length > 0x16 {
                    let speed = u16::from_le_bytes([buffer[offset + 0x15], buffer[offset + 0x16]]);
                    if speed > 0 {
                        max_speed = max_speed.max(speed);
                    }
                }
            } else {
                if length > 0x21 {
                    let speed = u16::from_le_bytes([buffer[offset + 0x20], buffer[offset + 0x21]]);
                    if speed > 0 {
                        max_speed = max_speed.max(speed);
                    }
                }
            }
        }

        // Move to next entry: SMBIOS entries are followed by double-null terminated strings
        let mut next = offset + length;
        while next + 1 < buffer.len() && !(buffer[next] == 0 && buffer[next + 1] == 0) {
            next += 1;
        }
        offset = next + 2; // skip double null
    }

    max_speed as u64
}


#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn hardware_ram_speed(_configured: bool) -> u64 {
    0
}

pub fn hardware_cpu_count() -> usize {
    #[cfg(windows)]
    {
        use winapi::um::sysinfoapi::{GetSystemInfo, SYSTEM_INFO};
        use std::mem;
        
        let mut sys_info: SYSTEM_INFO = unsafe { mem::zeroed() };
        unsafe {
            GetSystemInfo(&mut sys_info);
        }
        
        let num_procs = sys_info.dwNumberOfProcessors as usize;
        
        // Configure rayon to use all processors
        rayon::ThreadPoolBuilder::new()
            .num_threads(num_procs)
            .build_global()
            .ok();
        num_procs
    }
    #[cfg(target_os = "linux")]
    {
        use libc::{sched_getaffinity, cpu_set_t};
        use std::mem;
        
        let mut cpuset: cpu_set_t = unsafe { mem::zeroed() };
        let cpu_count = unsafe {
            if sched_getaffinity(0, mem::size_of::<cpu_set_t>(), &mut cpuset) == 0 {
                // Use libc's CPU_COUNT macro equivalent
                let mut count = 0;
                let bits_ptr = &cpuset as *const cpu_set_t as *const u64;
                let num_u64s = mem::size_of::<cpu_set_t>() / 8;
                for i in 0..num_u64s {
                    count += (*bits_ptr.add(i)).count_ones() as usize;
                }
                count
            } else {
                num_cpus::get()
            }
        };
        
        rayon::ThreadPoolBuilder::new()
            .num_threads(cpu_count)
            .build_global()
            .ok();
        cpu_count
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        let cpu_count = num_cpus::get();
        rayon::ThreadPoolBuilder::new()
            .num_threads(cpu_count)
            .build_global()
            .ok();
        cpu_count
    }
}

#[cfg(target_arch = "x86_64")]
mod cpuid {
    pub unsafe fn cpuid_count(leaf: u32, subleaf: u32, regs: &mut [u32; 4]) {
        #[cfg(target_arch = "x86_64")]
        {
            use std::arch::x86_64::__cpuid_count;
            let result = __cpuid_count(leaf, subleaf);
            regs[0] = result.eax;
            regs[1] = result.ebx;
            regs[2] = result.ecx;
            regs[3] = result.edx;
        }
    }
}

#[cfg(not(target_arch = "x86_64"))]
mod cpuid {
    pub unsafe fn cpuid_count(_leaf: u32, _subleaf: u32, regs: &mut [u32; 4]) {
        regs[0] = 0;
        regs[1] = 0;
        regs[2] = 0;
        regs[3] = 0;
    }
}

