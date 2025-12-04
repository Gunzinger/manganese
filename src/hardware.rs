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
// smb_info_extended.rs
// Cross-platform (Linux + Windows) SMBIOS parser with improved channel detection,
// empty-slot handling, CPU trimming and cache/core/thread info, and helpers.

use std::collections::{BTreeMap, HashMap};
use std::fmt;

#[derive(Debug, Default)]
pub struct SystemInfo {
    pub cpu: Option<CpuInfo>,
    pub board: Option<BoardInfo>,
    pub memory_devices: Vec<MemoryInfo>, // includes empty slots (populated == false)
    /// declared number of devices in Memory Array (SMBIOS Type 16; may be 0)
    pub memory_array_slots: Option<u8>,
}

#[derive(Debug, Default)]
pub struct CpuInfo {
    pub manufacturer: String,
    pub version: String,
    pub family: u8,
    pub socket: String,
    pub core_count: u16,
    pub thread_count: u16,
    pub l1_cache_kb: u32,
    pub l2_cache_kb: u32,
    pub l3_cache_kb: u32,

    // internal: handles referenced in Type 4 for caches
    l1_handle: u16,
    l2_handle: u16,
    l3_handle: u16,
}

#[derive(Debug, Default)]
pub struct BoardInfo {
    pub manufacturer: String,
    pub product: String,
    pub version: String,
    pub serial: String,
}

#[derive(Debug, Default, Clone)]
pub struct MemoryInfo {
    /// Speed field from Type 17 (in MHz)
    pub speed: u16,
    /// Configured Speed field from Type 17 (in MHz)
    pub configured_speed: u16,
    pub manufacturer: String,
    pub part_number: String,
    pub serial: String,
    /// Size in MB (0 means empty/unpopulated or unknown)
    pub size_mb: u32,
    pub locator: String,
    /// slot index parsed from locator when available (e.g., "DIMM 0" -> 0)
    pub slot_index: Option<u8>,
    /// assigned channel index (0-based). This is computed after all devices parsed.
    pub channel_index: Option<usize>,
    /// human channel name (computed, e.g., "Channel A" or "Channel 0")
    pub channel_name: Option<String>,
    /// whether the slot is populated (size_mb > 0 or a non-empty manufacturer)
    pub populated: bool,
}

impl fmt::Display for SystemInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(cpu) = &self.cpu {
            writeln!(f, "CPU: {} ({}) family {}", cpu.manufacturer, cpu.version, cpu.family)?;
            writeln!(f, " Socket: {}", cpu.socket)?;
            writeln!(f, " Cores: {}  Threads: {}", cpu.core_count, cpu.thread_count)?;
            writeln!(f, " L1: {} KB  L2: {} KB  L3: {} KB", cpu.l1_cache_kb, cpu.l2_cache_kb, cpu.l3_cache_kb)?;
        } else {
            writeln!(f, "CPU: <unknown>")?;
        }

        if let Some(board) = &self.board {
            writeln!(f, "Board: {} / {} (v{}) SN: {}", board.manufacturer, board.product, board.version, board.serial)?;
        } else {
            writeln!(f, "Board: <unknown>")?;
        }

        //writeln!(f, "Memory Array slots (Type 16): {:?}", self.memory_array_slots)?;

        let populated = self.memory_devices.iter().filter(|m| m.populated).count();
        writeln!(f, "Memory devices recorded: {} (populated: {})", self.memory_devices.len(), populated)?;

        // show channels
        let channels = self.memory_channels();
        writeln!(f, "Channels observed: {}", channels.len())?;
        for (ch_name, slots) in channels {
            writeln!(f, " {}:", ch_name)?;
            for (i, m) in slots.iter().enumerate() {
                writeln!(f, "  Slot {}: {}MB {}MHz (configured {}MHz) Locator: {} Populated: {}",
                         i+1, m.size_mb, m.speed, m.configured_speed, m.locator, m.populated)?;
                if !m.manufacturer.is_empty() {
                    writeln!(f, "   Manufacturer: {} Part: {} Serial: {}", m.manufacturer, m.part_number, m.serial)?;
                }
            }
        }

        Ok(())
    }
}

/////////////////////
// Helpers
/////////////////////

fn clean_opt_string(s: Option<String>) -> String {
    s.unwrap_or_default()
        .trim()
        .trim_end_matches(char::from(0))
        .to_string()
}

/// Return SMBIOS string referenced by `index` (1-based) for structure starting at `struct_start` in `buf`.
fn get_smbios_string(buf: &[u8], struct_start: usize, index: u8) -> Option<String> {
    if index == 0 {
        return None;
    }
    let struct_len = *buf.get(struct_start + 1)? as usize;
    let mut p = struct_start + struct_len;
    if p >= buf.len() {
        return None;
    }

    let mut cur = 1u8;
    while p < buf.len() {
        let mut end = p;
        while end < buf.len() && buf[end] != 0 {
            end += 1;
        }
        if cur == index {
            return Some(String::from_utf8_lossy(&buf[p..end]).into_owned());
        }
        cur = cur.saturating_add(1);
        p = end + 1;
        if p < buf.len() && buf[p] == 0 {
            break;
        }
    }
    None
}

fn smb_next_structure(buf: &[u8], offset: usize) -> Option<usize> {
    let len = *buf.get(offset + 1)? as usize;
    let mut next = offset + len;
    while next + 1 < buf.len() {
        if buf[next] == 0 && buf[next + 1] == 0 {
            return Some(next + 2);
        }
        next += 1;
    }
    None
}

fn le_u16_at(buf: &[u8], idx: usize) -> u16 {
    let a = *buf.get(idx).unwrap_or(&0);
    let b = *buf.get(idx + 1).unwrap_or(&0);
    u16::from_le_bytes([a, b])
}

/////////////////////
// Parsers for structures
/////////////////////

fn parse_type17(buf: &[u8], offset: usize) -> Option<MemoryInfo> {
    let struct_len = *buf.get(offset + 1)? as usize;
    if offset + struct_len > buf.len() {
        return None;
    }

    let size_field = le_u16_at(buf, offset + 0x0C);
    // SMBIOS uses 0 or 0xFFFF to indicate no module installed or unknown.
    let size_mb = if size_field == 0 || size_field == 0xFFFF { 0 } else { size_field as u32 };

    let locator_idx = *buf.get(offset + 0x10).unwrap_or(&0);
    let manufacturer_idx = *buf.get(offset + 0x17).unwrap_or(&0);
    let serial_idx = *buf.get(offset + 0x18).unwrap_or(&0);
    let part_idx = *buf.get(offset + 0x1A).unwrap_or(&0);

    let speed = le_u16_at(buf, offset + 0x15);
    let configured_speed = le_u16_at(buf, offset + 0x20);

    let locator_raw = clean_opt_string(get_smbios_string(buf, offset, locator_idx));
    let manufacturer = clean_opt_string(get_smbios_string(buf, offset, manufacturer_idx));
    let part_number = clean_opt_string(get_smbios_string(buf, offset, part_idx));
    let serial = clean_opt_string(get_smbios_string(buf, offset, serial_idx));

    let slot_index = parse_slot_index(&locator_raw);

    Some(MemoryInfo {
        speed,
        configured_speed,
        manufacturer,
        part_number,
        serial,
        size_mb,
        locator: locator_raw,
        slot_index,
        channel_index: None,
        channel_name: None,
        populated: size_mb > 0,
    })
}

/// tries to pull a trailing decimal number from locator, e.g. "DIMM 0" -> Some(0), "A1" -> Some(1)
fn parse_slot_index(locator: &str) -> Option<u8> {
    // find trailing digits
    let s = locator.trim();
    let mut digits_rev = String::new();
    for ch in s.chars().rev() {
        if ch.is_ascii_digit() {
            digits_rev.push(ch);
        } else {
            break;
        }
    }
    if digits_rev.is_empty() {
        // try single-letter like A1 -> take trailing char if digit
        return None;
    }
    let digits: String = digits_rev.chars().rev().collect();
    digits.parse::<u8>().ok()
}

fn parse_type4(buf: &[u8], offset: usize) -> Option<CpuInfo> {
    // Offsets:
    // 0x04 = socket (string), 0x06 = family (byte), 0x07 = manufacturer (string ref),
    // 0x10 = version (string ref)
    let socket_idx = *buf.get(offset + 0x04).unwrap_or(&0);
    let family = *buf.get(offset + 0x06).unwrap_or(&0);
    let manufacturer_idx = *buf.get(offset + 0x07).unwrap_or(&0);
    let version_idx = *buf.get(offset + 0x10).unwrap_or(&0);

    // cache handle references: word values at offsets commonly 0x12, 0x14, 0x16
    let l1_handle = le_u16_at(buf, offset + 0x12);
    let l2_handle = le_u16_at(buf, offset + 0x14);
    let l3_handle = le_u16_at(buf, offset + 0x16);

    // core/thread counts may be present at 0x23 and 0x25 (SMBIOS 2.7+), read if available
    let core_count = le_u16_at(buf, offset + 0x23);
    let thread_count = le_u16_at(buf, offset + 0x25);

    Some(CpuInfo {
        manufacturer: clean_opt_string(get_smbios_string(buf, offset, manufacturer_idx)),
        version: clean_opt_string(get_smbios_string(buf, offset, version_idx)),
        family,
        socket: clean_opt_string(get_smbios_string(buf, offset, socket_idx)),
        core_count,
        thread_count,
        l1_cache_kb: 0,
        l2_cache_kb: 0,
        l3_cache_kb: 0,
        l1_handle,
        l2_handle,
        l3_handle,
    })
}

fn parse_type2(buf: &[u8], offset: usize) -> Option<BoardInfo> {
    let man = *buf.get(offset + 0x04).unwrap_or(&0);
    let prod = *buf.get(offset + 0x05).unwrap_or(&0);
    let ver = *buf.get(offset + 0x06).unwrap_or(&0);
    let ser = *buf.get(offset + 0x07).unwrap_or(&0);

    Some(BoardInfo {
        manufacturer: clean_opt_string(get_smbios_string(buf, offset, man)),
        product: clean_opt_string(get_smbios_string(buf, offset, prod)),
        version: clean_opt_string(get_smbios_string(buf, offset, ver)),
        serial: clean_opt_string(get_smbios_string(buf, offset, ser)),
    })
}

fn parse_type16(buf: &[u8], offset: usize) -> Option<u8> {
    Some(*buf.get(offset + 0x0E).unwrap_or(&0))
}

/// parse Type 7 (Cache Information) and return (structure_handle, installed_size_kb)
fn parse_type7(buf: &[u8], offset: usize) -> Option<(u16, u32)> {
    // structure handle is at offset+2
    let handle = le_u16_at(buf, offset + 2);
    // Installed Size is often stored at offset 0x05..0x06 as a word
    let installed_kb = le_u16_at(buf, offset + 0x05) as u32;
    // Some SMBIOS variants use special encodings; we return what we get (best-effort)
    Some((handle, installed_kb))
}

/////////////////////
// Platform-specific SMBIOS consumption
/////////////////////

#[cfg(target_os = "linux")]
mod platform {
    use super::*;
    use glob::glob;
    use std::fs::File;
    use std::io::Read;

    fn read_sys_dmi_entries() -> Option<Vec<u8>> {
        let mut buffer = Vec::new();
        let pattern = "/sys/firmware/dmi/entries/*/raw";
        let entries = glob(pattern).ok()?;
        for entry in entries.flatten() {
            if let Ok(mut f) = File::open(&entry) {
                let mut tmp = Vec::new();
                if f.read_to_end(&mut tmp).is_ok() {
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

    fn fallback_cpu_counts(cpu: &mut CpuInfo) {
        // If SMBIOS didn't provide core/thread counts, try /proc/cpuinfo for a decent fallback.
        use std::fs;
        if cpu.core_count == 0 || cpu.thread_count == 0 {
            if let Ok(contents) = fs::read_to_string("/proc/cpuinfo") {
                let mut processor_count = 0u16;
                let mut cores_per_physical: Option<u16> = None;
                let mut current_physical = None::<String>;
                let mut core_ids: HashMap<String, std::collections::HashSet<String>> = HashMap::new();

                for line in contents.lines() {
                    if line.starts_with("processor") {
                        processor_count += 1;
                    }
                    if let Some(pos) = line.find(':') {
                        let key = line[..pos].trim();
                        let val = line[pos+1..].trim().to_string();
                        if key == "physical id" {
                            current_physical = Some(val.clone());
                        } else if key == "cpu cores" {
                            if let Ok(v) = val.parse::<u16>() {
                                cores_per_physical = Some(v);
                            }
                        } else if key == "core id" {
                            if let Some(pid) = current_physical.clone() {
                                core_ids.entry(pid).or_default().insert(val);
                            }
                        }
                    }
                }

                if cpu.thread_count == 0 {
                    cpu.thread_count = processor_count;
                }
                if cpu.core_count == 0 {
                    if let Some(v) = cores_per_physical {
                        cpu.core_count = v;
                    } else {
                        // try using core_ids
                        let mut cores_total = 0u16;
                        for set in core_ids.values() {
                            cores_total = cores_total.saturating_add(set.len() as u16);
                        }
                        if cores_total > 0 {
                            cpu.core_count = cores_total;
                        } else if cpu.thread_count > 0 {
                            // fall back: assume 1 thread per core (best-effort)
                            cpu.core_count = cpu.thread_count;
                        }
                    }
                }
            } else {
                // fallback to logical concurrency for threads
                if cpu.thread_count == 0 {
                    if let Ok(n) = std::thread::available_parallelism() {
                        cpu.thread_count = n.get() as u16;
                    }
                }
            }
        }
    }

    pub fn collect_system_info() -> SystemInfo {
        let mut sys = SystemInfo::default();
        let buf = match read_sys_dmi_entries() {
            Some(b) => b,
            None => return sys,
        };

        // temporary cache map: handle -> size_kb
        let mut cache_map: HashMap<u16, u32> = HashMap::new();

        let mut offset = 0usize;
        while offset + 4 <= buf.len() {
            let typ = buf[offset];
            let len = buf[offset + 1] as usize;
            if len == 0 { break; }
            if offset + len > buf.len() { break; }

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
                7 => { // cache info
                    if let Some((handle, size_kb)) = parse_type7(&buf, offset) {
                        cache_map.insert(handle, size_kb);
                    }
                }
                _ => {}
            }

            if let Some(next) = smb_next_structure(&buf, offset) {
                offset = next;
            } else {
                break;
            }
        }

        // assign caches to cpu if handles found
        if let Some(cpu) = sys.cpu.as_mut() {
            if cpu.l1_handle != 0 {
                if let Some(&s) = cache_map.get(&cpu.l1_handle) {
                    cpu.l1_cache_kb = s;
                }
            }
            if cpu.l2_handle != 0 {
                if let Some(&s) = cache_map.get(&cpu.l2_handle) {
                    cpu.l2_cache_kb = s;
                }
            }
            if cpu.l3_handle != 0 {
                if let Some(&s) = cache_map.get(&cpu.l3_handle) {
                    cpu.l3_cache_kb = s;
                }
            }

            // if core/thread counts are missing, try fallback
            fallback_cpu_counts(cpu);
        }

        // Assign channels from memory_devices using the improved algorithm:
        assign_memory_channels(&mut sys);

        sys
    }

    /// assign channel indices and human-readable names
    fn assign_memory_channels(sys: &mut SystemInfo) {
        // occ_counter per slot_index -> how many seen so far
        let mut occ_counter: HashMap<u8, usize> = HashMap::new();
        // Also track the case where slot_index is None (use locator string hashing)
        let mut unnamed_counter: HashMap<String, usize> = HashMap::new();

        // first pass: count occurrences per slot index to determine channel count
        for m in &sys.memory_devices {
            if let Some(idx) = m.slot_index {
                *occ_counter.entry(idx).or_insert(0) += 1;
            } else {
                let key = m.locator.clone();
                *unnamed_counter.entry(key).or_insert(0) += 1;
            }
        }

        // channel count is max of occurrences for any slot_index or unnamed highest
        let mut max_channels = 0usize;
        for &v in occ_counter.values() { if v > max_channels { max_channels = v; } }
        for &v in unnamed_counter.values() { if v > max_channels { max_channels = v; } }
        if max_channels == 0 {
            max_channels = 1; // at least one channel by default
        }

        // second pass: assign each device a channel index based on the occurrence index within its slot_index
        let mut seen_counter: HashMap<u8, usize> = HashMap::new();
        let mut seen_unnamed: HashMap<String, usize> = HashMap::new();

        for m in sys.memory_devices.iter_mut() {
            if let Some(idx) = m.slot_index {
                let occ = seen_counter.entry(idx).or_insert(0);
                let ch = *occ;
                *occ += 1;
                m.channel_index = Some(ch);
            } else {
                let key = m.locator.clone();
                let occ = seen_unnamed.entry(key.clone()).or_insert(0);
                let ch = *occ;
                *occ += 1;
                m.channel_index = Some(ch);
            }
            // compute a friendly name (A, B, C... if channels <= 26)
            if let Some(ch_idx) = m.channel_index {
                let ch_name = if max_channels <= 26 {
                    // map 0->A, 1->B ...
                    let letter = (b'A' + (ch_idx as u8)).min(b'Z') as char;
                    format!("Channel {}", letter)
                } else {
                    format!("Channel {}", ch_idx)
                };
                m.channel_name = Some(ch_name);
            }
        }

        // Ensure every channel has at least a name for empty slots: fill None -> default name
        for m in sys.memory_devices.iter_mut() {
            if m.channel_name.is_none() {
                m.channel_index = Some(0);
                m.channel_name = Some("Channel A".to_string());
            }
        }
    }
}

#[cfg(target_os = "windows")]
mod platform {
    use super::*;
    use windows::Win32::System::SystemInformation::{GetSystemFirmwareTable, RSMB};

    pub fn collect_system_info() -> SystemInfo {
        let mut sys = SystemInfo::default();

        unsafe {
            let size = GetSystemFirmwareTable(RSMB, 0, None);
            if size == 0 { return sys; }
            let mut buffer = vec![0u8; size as usize];
            let got = GetSystemFirmwareTable(RSMB, 0, Some(&mut buffer[..]));
            if got == 0 || (got as usize) > buffer.len() { return sys; }

            let mut cache_map: HashMap<u16, u32> = HashMap::new();

            let mut offset = 0usize;
            while offset + 4 <= buffer.len() {
                let typ = buffer[offset];
                let len = buffer[offset + 1] as usize;
                if len == 0 { break; }
                if offset + len > buffer.len() { break; }

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
                    7 => {
                        if let Some((handle, size_kb)) = parse_type7(&buffer, offset) {
                            cache_map.insert(handle, size_kb);
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

            // assign caches
            if let Some(cpu) = sys.cpu.as_mut() {
                if cpu.l1_handle != 0 {
                    if let Some(&s) = cache_map.get(&cpu.l1_handle) {
                        cpu.l1_cache_kb = s;
                    }
                }
                if cpu.l2_handle != 0 {
                    if let Some(&s) = cache_map.get(&cpu.l2_handle) {
                        cpu.l2_cache_kb = s;
                    }
                }
                if cpu.l3_handle != 0 {
                    if let Some(&s) = cache_map.get(&cpu.l3_handle) {
                        cpu.l3_cache_kb = s;
                    }
                }

                // fallback for thread/core: try available_parallelism
                if cpu.thread_count == 0 {
                    if let Ok(n) = std::thread::available_parallelism() {
                        cpu.thread_count = n.get() as u16;
                    }
                }
                // leave core_count as-is if unknown; Windows fallback requires additional APIs.
            }

            // assign channels
            assign_memory_channels(&mut sys);
        }

        sys
    }

    fn assign_memory_channels(sys: &mut SystemInfo) {
        // same algorithm as linux
        let mut occ_counter: HashMap<u8, usize> = HashMap::new();
        let mut unnamed_counter: HashMap<String, usize> = HashMap::new();

        for m in &sys.memory_devices {
            if let Some(idx) = m.slot_index {
                *occ_counter.entry(idx).or_insert(0) += 1;
            } else {
                let key = m.locator.clone();
                *unnamed_counter.entry(key).or_insert(0) += 1;
            }
        }

        let mut max_channels = 0usize;
        for &v in occ_counter.values() { if v > max_channels { max_channels = v; } }
        for &v in unnamed_counter.values() { if v > max_channels { max_channels = v; } }
        if max_channels == 0 { max_channels = 1; }

        let mut seen_counter: HashMap<u8, usize> = HashMap::new();
        let mut seen_unnamed: HashMap<String, usize> = HashMap::new();

        for m in sys.memory_devices.iter_mut() {
            if let Some(idx) = m.slot_index {
                let occ = seen_counter.entry(idx).or_insert(0);
                let ch = *occ;
                *occ += 1;
                m.channel_index = Some(ch);
            } else {
                let key = m.locator.clone();
                let occ = seen_unnamed.entry(key.clone()).or_insert(0);
                let ch = *occ;
                *occ += 1;
                m.channel_index = Some(ch);
            }
            if let Some(ch_idx) = m.channel_index {
                let ch_name = if max_channels <= 26 {
                    let letter = (b'A' + (ch_idx as u8)).min(b'Z') as char;
                    format!("Channel {}", letter)
                } else {
                    format!("Channel {}", ch_idx)
                };
                m.channel_name = Some(ch_name);
            }
        }
        for m in sys.memory_devices.iter_mut() {
            if m.channel_name.is_none() {
                m.channel_index = Some(0);
                m.channel_name = Some("Channel A".to_string());
            }
        }
    }
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
mod platform {
    use super::*;
    pub fn collect_system_info() -> SystemInfo {
        SystemInfo::default()
    }
}

/////////////////////
// Public APIs (helpers)
/////////////////////

pub fn get_system_info() -> SystemInfo {
    platform::collect_system_info()
}

impl SystemInfo {
    /// Map of ChannelName -> vector of references to MemoryInfo (preserves insertion order per channel).
    pub fn memory_channels(&self) -> BTreeMap<String, Vec<&MemoryInfo>> {
        let mut map: BTreeMap<String, Vec<&MemoryInfo>> = BTreeMap::new();
        for m in &self.memory_devices {
            let name = m.channel_name.clone().unwrap_or_else(|| "Channel 0".to_string());
            map.entry(name).or_default().push(m);
        }
        map
    }

    /// Number of channels that have at least one populated DIMM
    pub fn populated_channels(&self) -> usize {
        self.memory_channels()
            .values()
            .filter(|slots| slots.iter().any(|s| s.populated))
            .count()
    }

    /// Total channels observed (populated or not)
    pub fn total_channels(&self) -> usize {
        self.memory_channels().len()
    }

    /// Number of populated DIMM slots
    pub fn populated_slots(&self) -> usize {
        self.memory_devices.iter().filter(|m| m.populated).count()
    }

    /// Number of total slots recorded
    pub fn total_slots(&self) -> usize {
        self.memory_devices.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke() {
        let info = get_system_info();
        println!("{:#?}", info);
        println!("{}", info);
    }
}

