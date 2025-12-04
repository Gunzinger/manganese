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


// ---

// smb_info.rs
// Cross-platform (Linux + Windows) SMBIOS parser to extract:
// - CPU info (manufacturer/version/family/socket)
// - Baseboard (manufacturer/product/version/serial)
// - Memory devices (speed, configured speed, manufacturer, part, serial, size, locator)
// - Memory array / populated slots count
//
// Usage:
// ```ignore
// let info = smb_info::get_system_info();
// println!("{:#?}", info);
// ```

use std::fmt;

#[derive(Debug, Default)]
pub struct SystemInfo {
    pub cpu: Option<CpuInfo>,
    pub board: Option<BoardInfo>,
    pub memory_devices: Vec<MemoryInfo>,
    /// declared number of devices in Memory Array (SMBIOS Type 16; may be 0)
    pub memory_array_slots: Option<u8>,
}

#[derive(Debug, Default)]
pub struct CpuInfo {
    pub manufacturer: String,
    pub version: String,
    pub family: u8,
    pub socket: String,
}

#[derive(Debug, Default)]
pub struct BoardInfo {
    pub manufacturer: String,
    pub product: String,
    pub version: String,
    pub serial: String,
}

#[derive(Debug, Default)]
pub struct MemoryInfo {
    /// Speed field from Type 17 (in MHz)
    pub speed: u16,
    /// Configured Speed field from Type 17 (in MHz)
    pub configured_speed: u16,
    pub manufacturer: String,
    pub part_number: String,
    pub serial: String,
    /// Size in MB (best-effort)
    pub size_mb: u32,
    pub locator: String,
}

impl fmt::Display for SystemInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(cpu) = &self.cpu {
            writeln!(f, "CPU: {} ({}) family {}", cpu.manufacturer, cpu.version, cpu.family)?;
            writeln!(f, " Socket: {}", cpu.socket)?;
        } else {
            writeln!(f, "CPU: <unknown>")?;
        }

        if let Some(board) = &self.board {
            writeln!(f, "Board: {} / {} (v{}) SN: {}", board.manufacturer, board.product, board.version, board.serial)?;
        } else {
            writeln!(f, "Board: <unknown>")?;
        }

        writeln!(f, "Memory Array slots (Type 16): {:?}", self.memory_array_slots)?;

        let populated = self.memory_devices.iter().filter(|m| m.size_mb > 0 || !m.manufacturer.is_empty()).count();
        writeln!(f, "Memory devices populated: {}", populated)?;
        for (i, m) in self.memory_devices.iter().enumerate() {
            writeln!(f, " Slot {}: {}MB (speed {}MHz configured {})", i + 1, m.size_mb, m.speed, m.configured_speed)?;
            writeln!(f, "  Manufacturer: {}", m.manufacturer)?;
            writeln!(f, "  Part: {} Serial: {} Locator: {}", m.part_number, m.serial, m.locator)?;
        }
        Ok(())
    }
}

/////////////////////
// Common parsing helpers
/////////////////////

/// Return SMBIOS string referenced by `index` (1-based) for structure starting at `struct_start` in `buf`.
/// Safe: returns None if index == 0 or out-of-bounds.
fn get_smbios_string(buf: &[u8], struct_start: usize, index: u8) -> Option<String> {
    if index == 0 {
        return None;
    }
    // structure length is at offset + 1
    let struct_len = *buf.get(struct_start + 1)? as usize;
    let mut p = struct_start + struct_len;
    if p >= buf.len() {
        return None;
    }

    // iterate strings
    let mut cur = 1u8;
    while p < buf.len() {
        // find end of string
        let mut end = p;
        while end < buf.len() && buf[end] != 0 {
            end += 1;
        }
        // if this string is the requested one, return it
        if cur == index {
            let slice = &buf[p..end];
            // interpret as UTF-8 lossily
            return Some(String::from_utf8_lossy(slice).into_owned());
        }
        // advance to next string (skip the terminating NUL)
        cur = cur.saturating_add(1);
        p = end + 1;

        // check for double NUL -> end of strings
        if p < buf.len() && buf[p] == 0 {
            break;
        }
    }
    None
}

/// Advance `offset` to the next SMBIOS structure (skip formatted area and trailing string area).
/// Returns next offset or None if EOF or malformed.
fn smb_next_structure(buf: &[u8], offset: usize) -> Option<usize> {
    let len = *buf.get(offset + 1)? as usize;
    let mut next = offset + len;
    // walk until double nul
    while next + 1 < buf.len() {
        if buf[next] == 0 && buf[next + 1] == 0 {
            return Some(next + 2);
        }
        next += 1;
    }
    None
}

/// Safely read little-endian u16 at given index
fn le_u16_at(buf: &[u8], idx: usize) -> u16 {
    let a = *buf.get(idx).unwrap_or(&0);
    let b = *buf.get(idx + 1).unwrap_or(&0);
    u16::from_le_bytes([a, b])
}

/////////////////////
// Structure parsers (assume `offset` points to start of structure in `buf`)
/////////////////////

fn parse_type17(buf: &[u8], offset: usize) -> Option<MemoryInfo> {
    // formatted area length must be present
    let struct_len = *buf.get(offset + 1)? as usize;
    if offset + struct_len > buf.len() {
        return None;
    }

    // According to SMBIOS the following offsets are common:
    // 0x0C-0x0D = Size (word), 0x10 = Locator (string index)
    // 0x15-0x16 = Speed (u16), 0x17 = Manufacturer (string index)
    // 0x18 = SerialNumber (string index), 0x1A = PartNumber (string index)
    // Configured Clock Speed often at 0x20-0x21 for later versions
    let size_field = le_u16_at(buf, offset + 0x0C);
    let size_mb = if size_field == 0 || size_field == 0xFFFF { 0 } else { size_field as u32 };

    let locator_idx = *buf.get(offset + 0x10).unwrap_or(&0);
    let manufacturer_idx = *buf.get(offset + 0x17).unwrap_or(&0);
    let serial_idx = *buf.get(offset + 0x18).unwrap_or(&0);
    let part_idx = *buf.get(offset + 0x1A).unwrap_or(&0);

    let speed = le_u16_at(buf, offset + 0x15);
    let configured_speed = le_u16_at(buf, offset + 0x20);

    Some(MemoryInfo {
        speed,
        configured_speed,
        manufacturer: get_smbios_string(buf, offset, manufacturer_idx).unwrap_or_default(),
        part_number: get_smbios_string(buf, offset, part_idx).unwrap_or_default(),
        serial: get_smbios_string(buf, offset, serial_idx).unwrap_or_default(),
        size_mb,
        locator: get_smbios_string(buf, offset, locator_idx).unwrap_or_default(),
    })
}

fn parse_type4(buf: &[u8], offset: usize) -> Option<CpuInfo> {
    // Type 4 (Processor Info)
    // Offsets:
    // 0x04 = socket (string), 0x06 = family (byte), 0x07 = manufacturer (string ref),
    // 0x10 = version (string ref)
    let socket_idx = *buf.get(offset + 0x04).unwrap_or(&0);
    let family = *buf.get(offset + 0x06).unwrap_or(&0);
    let manufacturer_idx = *buf.get(offset + 0x07).unwrap_or(&0);
    let version_idx = *buf.get(offset + 0x10).unwrap_or(&0);

    Some(CpuInfo {
        manufacturer: get_smbios_string(buf, offset, manufacturer_idx).unwrap_or_default(),
        version: get_smbios_string(buf, offset, version_idx).unwrap_or_default(),
        family,
        socket: get_smbios_string(buf, offset, socket_idx).unwrap_or_default(),
    })
}

fn parse_type2(buf: &[u8], offset: usize) -> Option<BoardInfo> {
    // Type 2 (Baseboard)
    // 0x04 = Manufacturer, 0x05 = Product, 0x06 = Version, 0x07 = Serial
    let man = *buf.get(offset + 0x04).unwrap_or(&0);
    let prod = *buf.get(offset + 0x05).unwrap_or(&0);
    let ver = *buf.get(offset + 0x06).unwrap_or(&0);
    let ser = *buf.get(offset + 0x07).unwrap_or(&0);

    Some(BoardInfo {
        manufacturer: get_smbios_string(buf, offset, man).unwrap_or_default(),
        product: get_smbios_string(buf, offset, prod).unwrap_or_default(),
        version: get_smbios_string(buf, offset, ver).unwrap_or_default(),
        serial: get_smbios_string(buf, offset, ser).unwrap_or_default(),
    })
}

fn parse_type16(buf: &[u8], offset: usize) -> Option<u8> {
    // Type 16 (Physical Memory Array)
    // 0x0E = NumberOfDevices (1 byte)
    Some(*buf.get(offset + 0x0E).unwrap_or(&0))
}

/////////////////////
// Platform-specific code
/////////////////////

#[cfg(target_os = "linux")]
mod platform {
    use super::*;
    use glob::glob;
    use std::fs;
    use std::io::Read;

    /// Build a single buffer by concatenating the raw contents of all /sys/firmware/dmi/entries/*/raw files.
    /// This mirrors SMBIOS table parsing by reading all entries (each file is a single structure + strings).
    fn read_sys_dmi_entries() -> Option<Vec<u8>> {
        let mut buffer = Vec::new();
        // iterate all entry raw files
        let pattern = "/sys/firmware/dmi/entries/*/raw";
        let entries = glob(pattern).ok()?;
        for entry in entries.flatten() {
            if let Ok(mut f) = fs::File::open(&entry) {
                let mut tmp = Vec::new();
                if f.read_to_end(&mut tmp).is_ok() {
                    // Some kernels expose each structure as a file. Append and keep entries contiguous.
                    buffer.extend_from_slice(&tmp);
                }
            }
        }
        if buffer.is_empty() {
            None
        } else {
            Some(buffer)
        }
    }

    pub fn collect_system_info() -> SystemInfo {
        let mut sys = SystemInfo::default();
        let buf = match read_sys_dmi_entries() {
            Some(b) => b,
            None => return sys,
        };

        let mut offset = 0usize;
        while offset + 4 <= buf.len() {
            let typ = buf[offset];
            let len = buf[offset + 1] as usize;
            if len == 0 {
                break;
            }
            if offset + len > buf.len() {
                break;
            }

            match typ {
                17 => {
                    if let Some(m) = parse_type17(&buf, offset) {
                        sys.memory_devices.push(m);
                    }
                }
                4 => {
                    if sys.cpu.is_none() {
                        sys.cpu = parse_type4(&buf, offset);
                    }
                }
                2 => {
                    if sys.board.is_none() {
                        sys.board = parse_type2(&buf, offset);
                    }
                }
                16 => {
                    if sys.memory_array_slots.is_none() {
                        sys.memory_array_slots = parse_type16(&buf, offset);
                    }
                }
                _ => {}
            }

            // advance to next structure
            if let Some(next) = smb_next_structure(&buf, offset) {
                offset = next;
            } else {
                break;
            }
        }

        sys
    }
}

#[cfg(target_os = "windows")]
mod platform {
    use super::*;
    use windows::Win32::System::SystemInformation::{GetSystemFirmwareTable, RSMB};
    use std::mem;

    pub fn collect_system_info() -> SystemInfo {
        let mut sys = SystemInfo::default();

        unsafe {
            // Step 1: get required size
            let size = GetSystemFirmwareTable(RSMB, 0, None);
            if size == 0 {
                // failed
                return sys;
            }
            // allocate
            let mut buffer = vec![0u8; size as usize];
            let got = GetSystemFirmwareTable(RSMB, 0, Some(&mut buffer[..]));
            if got == 0 || got as usize > buffer.len() {
                return sys;
            }

            let mut offset = 0usize;
            while offset + 4 <= buffer.len() {
                let typ = buffer[offset];
                let len = buffer[offset + 1] as usize;
                if len == 0 {
                    break;
                }
                if offset + len > buffer.len() {
                    break;
                }

                match typ {
                    17 => {
                        if let Some(m) = parse_type17(&buffer, offset) {
                            sys.memory_devices.push(m);
                        }
                    }
                    4 => {
                        if sys.cpu.is_none() {
                            sys.cpu = parse_type4(&buffer, offset);
                        }
                    }
                    2 => {
                        if sys.board.is_none() {
                            sys.board = parse_type2(&buffer, offset);
                        }
                    }
                    16 => {
                        if sys.memory_array_slots.is_none() {
                            sys.memory_array_slots = parse_type16(&buffer, offset);
                        }
                    }
                    _ => {}
                }

                if let Some(next) = smb_next_structure(&buffer, offset) {
                    offset = next;
                } else {
                    break;
                }
            }
        }

        sys
    }
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
mod platform {
    use super::*;
    pub fn collect_system_info() -> SystemInfo {
        // unsupported platform: return empty SystemInfo
        SystemInfo::default()
    }
}

/////////////////////
// Public API
/////////////////////

/// Collects and returns system info (CPU, board, memory devices, memory array slot count).
pub fn get_system_info() -> SystemInfo {
    platform::collect_system_info()
}

/////////////////////
// Optional tests / example
/////////////////////

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke() {
        let info = get_system_info();
        // just ensure it doesn't crash
        println!("{:#?}", info);
    }
}

