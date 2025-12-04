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
// smb_info_final.rs
// Cross-platform SMBIOS parser with correct core/thread counts, cache parsing, channel inferencing,
// and display that omits unpopulated memory slots & empty Type 16 arrays.

use std::collections::{BTreeMap, HashMap};
use std::fmt;

#[derive(Debug, Default)]
pub struct SystemInfo {
    pub cpu: Option<CpuInfo>,
    pub board: Option<BoardInfo>,
    pub memory_devices: Vec<MemoryInfo>, // includes recorded slots (populated flag indicates presence)
    /// declared number of devices in Memory Array (SMBIOS Type 16; None if not present or zero)
    pub memory_array_slots: Option<u8>,
}

#[derive(Debug, Default)]
pub struct CpuInfo {
    pub manufacturer: String,
    pub version: String,
    pub family: u8,
    pub socket: String,
    pub core_count: u32,
    pub thread_count: u32,
    pub l1_cache_kb: u32,
    pub l2_cache_kb: u32,
    pub l3_cache_kb: u32,
    // internal cache handles from Type 4 (0x1A/0x1C/0x1E)
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
    pub speed: u16,
    pub configured_speed: u16,
    pub manufacturer: String,
    pub part_number: String,
    pub serial: String,
    /// Size in MB (0 -> not populated / unknown)
    pub size_mb: u32,
    pub locator: String,
    /// trailing numeric index from locator if present e.g. "DIMM 0" -> Some(0)
    pub slot_index: Option<u8>,
    /// zero-based channel index assigned by algorithm
    pub channel_index: Option<usize>,
    /// friendly channel name (e.g. "Channel A")
    pub channel_name: Option<String>,
    /// whether slot is populated (size or manufacturer indicates)
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

        // Only show memory_array_slots if present and > 0
        if let Some(n) = self.memory_array_slots {
            if n > 0 {
                writeln!(f, "Memory Array slots (Type 16): {}", n)?;
            }
        }

        // channels and slots - do not print unpopulated slots; only show populated DIMMs
        let channels = self.memory_channels();
        let populated_channels = self.populated_channels();
        if !channels.is_empty() {
            writeln!(f, "Channels observed: {} (populated: {})", channels.len(), populated_channels)?;
            for (ch_name, slots) in channels {
                // show only populated slots for output
                let populated_slots: Vec<&MemoryInfo> = slots.iter().filter(|s| s.populated).cloned().collect();
                if populated_slots.is_empty() {
                    // skip printing empty channels entirely
                    continue;
                }
                writeln!(f, " {}:", ch_name)?;
                for (i, m) in populated_slots.iter().enumerate() {
                    writeln!(
                        f,
                        "  Slot {}: {}MB @ {}MHz (configured {}MHz) Locator: {}",
                        i + 1,
                        m.size_mb,
                        m.speed,
                        m.configured_speed,
                        m.locator
                    )?;
                    if !m.manufacturer.is_empty() {
                        writeln!(f, "   Manufacturer: {}  Part: {}  Serial: {}", m.manufacturer, m.part_number, m.serial)?;
                    }
                }
            }
        } else {
            writeln!(f, "Memory: <none discovered>")?;
        }

        Ok(())
    }
}

/////////////////////////////////////
// small helpers (string cleaning, ub readers)
/////////////////////////////////////

fn clean_opt_string(s: Option<String>) -> String {
    s.unwrap_or_default().trim().trim_end_matches(char::from(0)).to_string()
}

fn le_u16_at(buf: &[u8], idx: usize) -> u16 {
    let a = *buf.get(idx).unwrap_or(&0);
    let b = *buf.get(idx + 1).unwrap_or(&0);
    u16::from_le_bytes([a, b])
}

fn le_u32_at(buf: &[u8], idx: usize) -> u32 {
    let a = *buf.get(idx).unwrap_or(&0);
    let b = *buf.get(idx + 1).unwrap_or(&0);
    let c = *buf.get(idx + 2).unwrap_or(&0);
    let d = *buf.get(idx + 3).unwrap_or(&0);
    u32::from_le_bytes([a, b, c, d])
}

/// read SMBIOS string referenced by index (1-based) from structure at struct_start
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

/// advance to next structure (skip formatted area + strings)
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

/////////////////////////////////////
// parsers for individual SMBIOS types
/////////////////////////////////////

fn parse_type17(buf: &[u8], offset: usize) -> Option<MemoryInfo> {
    let struct_len = *buf.get(offset + 1)? as usize;
    if offset + struct_len > buf.len() { return None; }

    // size at 0x0C (word). 0 or 0xFFFF => not present/unknown
    let size_word = le_u16_at(buf, offset + 0x0C);
    let size_mb = if size_word == 0 || size_word == 0xFFFF { 0 } else { size_word as u32 };

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

/// try to parse trailing numeric slot index from locator string, e.g. "DIMM 0" -> Some(0), "A1" -> Some(1)
fn parse_slot_index(locator: &str) -> Option<u8> {
    let s = locator.trim();
    // collect trailing digits
    let mut rev = String::new();
    for ch in s.chars().rev() {
        if ch.is_ascii_digit() {
            rev.push(ch);
        } else {
            break;
        }
    }
    if rev.is_empty() { return None; }
    let digits: String = rev.chars().rev().collect();
    digits.parse::<u8>().ok()
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

/// parse Processor (Type 4). Note: core/thread counts are bytes at offsets 0x23 and 0x25 (per SMBIOS spec).
/// L1/L2/L3 cache handles are words at offsets 0x1A,0x1C,0x1E respectively.
fn parse_type4(buf: &[u8], offset: usize) -> Option<CpuInfo> {
    let struct_len = *buf.get(offset + 1)? as usize;
    if offset + struct_len > buf.len() { return None; }

    let socket_idx = *buf.get(offset + 0x04).unwrap_or(&0);
    let family = *buf.get(offset + 0x06).unwrap_or(&0);
    let manufacturer_idx = *buf.get(offset + 0x07).unwrap_or(&0);
    let version_idx = *buf.get(offset + 0x10).unwrap_or(&0);

    // cache handles at 0x1A/0x1C/0x1E (word)
    let l1_handle = le_u16_at(buf, offset + 0x1A);
    let l2_handle = le_u16_at(buf, offset + 0x1C);
    let l3_handle = le_u16_at(buf, offset + 0x1E);

    // core/thread counts are bytes at offsets 0x23 and 0x25 (SMBIOS 2.5+). If 0 => unknown -> fallback later.
    let core_count_b = *buf.get(offset + 0x23).unwrap_or(&0);
    let thread_count_b = *buf.get(offset + 0x25).unwrap_or(&0);

    Some(CpuInfo {
        manufacturer: clean_opt_string(get_smbios_string(buf, offset, manufacturer_idx)),
        version: clean_opt_string(get_smbios_string(buf, offset, version_idx)),
        family,
        socket: clean_opt_string(get_smbios_string(buf, offset, socket_idx)),
        core_count: if core_count_b == 0 || core_count_b == 0xFF { 0 } else { core_count_b as u32 },
        thread_count: if thread_count_b == 0 || thread_count_b == 0xFF { 0 } else { thread_count_b as u32 },
        l1_cache_kb: 0,
        l2_cache_kb: 0,
        l3_cache_kb: 0,
        l1_handle,
        l2_handle,
        l3_handle,
    })
}

/// parse Type 16 (Memory Array) NumberOfDevices at offset 0x0E (byte)
fn parse_type16(buf: &[u8], offset: usize) -> Option<u8> {
    Some(*buf.get(offset + 0x0E).unwrap_or(&0))
}

/// parse Type 7 (Cache Information)
/// return (structure_handle, installed_kb, cache_level)
/// - handle is word at offset+2
/// - installed size word at offset+9 (word). bit15 = granularity (0 => 1KB units, 1 => 64KB units)
/// - cache level is in Cache Configuration bits 2:0 (located at offset+5 bits 2:0 per spec)
fn parse_type7(buf: &[u8], offset: usize) -> Option<(u16, u32, u8)> {
    let struct_len = *buf.get(offset + 1)? as usize;
    if offset + struct_len > buf.len() { return None; }

    let handle = le_u16_at(buf, offset + 2);
    // Cache Configuration at offset 0x05 (word) — bits 2:0 indicate cache level (1..8)
    let cache_config = le_u16_at(buf, offset + 0x05);
    let cache_level = (cache_config & 0x7) as u8; // bits 2:0

    // Installed Size at offset 0x09 (word) for SMBIOS 2.x; for 3.x it may be extended, but we handle the common word.
    let installed_word = le_u16_at(buf, offset + 0x09);

    if installed_word == 0 {
        // no cache installed -> size 0
        return Some((handle, 0, cache_level));
    }

    let granularity = (installed_word & 0x8000) != 0;
    let raw = (installed_word & 0x7FFF) as u32;
    // If granularity == 0 => units are kilobytes; if granularity == 1 => units are 64 KB blocks
    let size_kb = if granularity { raw * 64 } else { raw };

    Some((handle, size_kb, cache_level))
}

///////////////////////////////////////////
// Shared helpers (DRY): channel assignment + cache mapping
///////////////////////////////////////////

/// Assign channel indices & names to the memory devices inside SystemInfo.
/// Algorithm:
/// - For each slot_index value (the trailing number parsed from the locator), count how many times it occurs.
/// - The maximum occurrences across slot indices determines number of channels.
/// - For each MemoryInfo with slot_index = N, assign it to channel = occurrence_index (0-based),
///   i.e. the 1st DIMM N -> channel 0, 2nd DIMM N -> channel 1, etc.
/// - For locators without numeric indices, group by locator string and apply the same N-th-occurrence rule.
fn assign_memory_channels(sys: &mut SystemInfo) {
    let mut counts_by_slot: HashMap<u8, usize> = HashMap::new();
    let mut counts_by_name: HashMap<String, usize> = HashMap::new();

    // first pass: counts
    for m in &sys.memory_devices {
        if let Some(idx) = m.slot_index {
            *counts_by_slot.entry(idx).or_insert(0) += 1;
        } else {
            *counts_by_name.entry(m.locator.clone()).or_insert(0) += 1;
        }
    }

    // compute maximum channels
    let mut max_channels = counts_by_slot.values().copied().max().unwrap_or(0);
    max_channels = max_channels.max(counts_by_name.values().copied().max().unwrap_or(0));
    if max_channels == 0 {
        max_channels = 1;
    }

    // second pass: assign occurrence order
    let mut seen_slot: HashMap<u8, usize> = HashMap::new();
    let mut seen_name: HashMap<String, usize> = HashMap::new();

    for m in sys.memory_devices.iter_mut() {
        let ch = if let Some(idx) = m.slot_index {
            let occ = seen_slot.entry(idx).or_insert(0);
            let ch = *occ;
            *occ += 1;
            ch
        } else {
            let key = m.locator.clone();
            let occ = seen_name.entry(key.clone()).or_insert(0);
            let ch = *occ;
            *occ += 1;
            ch
        };
        m.channel_index = Some(ch);
        let name = if max_channels <= 26 {
            let letter = (b'A' + (ch as u8)).min(b'Z') as char;
            format!("Channel {}", letter)
        } else {
            format!("Channel {}", ch)
        };
        m.channel_name = Some(name);
    }
}

/// Build map of cache handle -> (size_kb, level)
fn build_cache_map(buf: &[u8]) -> HashMap<u16, (u32, u8)> {
    let mut offset = 0usize;
    let mut map = HashMap::new();
    while offset + 4 <= buf.len() {
        let typ = buf[offset];
        let len = buf[offset + 1] as usize;
        if len == 0 { break; }
        if offset + len > buf.len() { break; }

        if typ == 7 {
            if let Some((handle, size_kb, level)) = parse_type7(buf, offset) {
                map.insert(handle, (size_kb, level));
            }
        }

        if let Some(next) = smb_next_structure(buf, offset) {
            offset = next;
        } else {
            break;
        }
    }
    map
}

/// Map cache sizes from cache_map into sys.cpu by matching cache handles (Type4 -> Type7).
fn apply_cache_handles(sys: &mut SystemInfo, cache_map: &HashMap<u16, (u32, u8)>) {
    if let Some(cpu) = sys.cpu.as_mut() {
        if cpu.l1_handle != 0 && cpu.l1_handle != 0xFFFF {
            if let Some(&(size_kb, _lv)) = cache_map.get(&cpu.l1_handle) {
                cpu.l1_cache_kb = size_kb;
            }
        }
        if cpu.l2_handle != 0 && cpu.l2_handle != 0xFFFF {
            if let Some(&(size_kb, _lv)) = cache_map.get(&cpu.l2_handle) {
                cpu.l2_cache_kb = size_kb;
            }
        }
        if cpu.l3_handle != 0 && cpu.l3_handle != 0xFFFF {
            if let Some(&(size_kb, _lv)) = cache_map.get(&cpu.l3_handle) {
                cpu.l3_cache_kb = size_kb;
            }
        }
    }
}

/// If CPU core/thread counts are missing, try OS fallbacks (Linux: /proc/cpuinfo; generic: available_parallelism).
fn cpu_counts_fallback(cpu: &mut CpuInfo) {
    if cpu.core_count == 0 || cpu.thread_count == 0 {
        // Try platform-generic thread count fallback
        if cpu.thread_count == 0 {
            if let Ok(n) = std::thread::available_parallelism() {
                cpu.thread_count = n.get() as u32;
            }
        }
        // On Linux prefer /proc/cpuinfo to estimate core count (best-effort)
        #[cfg(target_os = "linux")]
        {
            use std::collections::HashSet;
            use std::fs;
            if cpu.core_count == 0 {
                if let Ok(s) = fs::read_to_string("/proc/cpuinfo") {
                    let mut physical_cores: HashSet<(String, String)> = HashSet::new();
                    let mut cur_phys = String::new();
                    let mut cur_core = String::new();
                    for line in s.lines() {
                        if line.starts_with("physical id") {
                            if let Some(pos) = line.find(':') {
                                cur_phys = line[pos+1..].trim().to_string();
                            }
                        } else if line.starts_with("core id") {
                            if let Some(pos) = line.find(':') {
                                cur_core = line[pos+1..].trim().to_string();
                                physical_cores.insert((cur_phys.clone(), cur_core.clone()));
                            }
                        }
                    }
                    if !physical_cores.is_empty() {
                        cpu.core_count = physical_cores.len() as u32;
                    } else if cpu.thread_count > 0 {
                        // fallback assume SMT=2 when thread_count >=2
                        cpu.core_count = std::cmp::max(1, cpu.thread_count / 2);
                    }
                }
            }
        }
    }
}

///////////////////////////////////////////
// Platform-specific SMBIOS reading & tie together
///////////////////////////////////////////

#[cfg(target_os = "linux")]
mod platform {
    use super::*;
    use glob::glob;
    use std::fs::File;
    use std::io::Read;

    fn read_all_dmi_entries() -> Option<Vec<u8>> {
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
        if buffer.is_empty() { None } else { Some(buffer) }
    }

    pub fn collect_system_info() -> SystemInfo {
        let mut sys = SystemInfo::default();
        let buf = match read_all_dmi_entries() { Some(b) => b, None => return sys };

        // build cache map first (Type 7)
        let cache_map = build_cache_map(&buf);

        let mut offset = 0usize;
        while offset + 4 <= buf.len() {
            let typ = buf[offset];
            let len = buf[offset + 1] as usize;
            if len == 0 { break; }
            if offset + len > buf.len() { break; }

            match typ {
                2 => {
                    if sys.board.is_none() {
                        sys.board = parse_type2(&buf, offset);
                    }
                }
                4 => {
                    if sys.cpu.is_none() {
                        sys.cpu = parse_type4(&buf, offset);
                    }
                }
                16 => {
                    // If NumberOfDevices == 0 => treat as not present per request
                    if sys.memory_array_slots.is_none() {
                        if let Some(n) = parse_type16(&buf, offset) {
                            if n > 0 { sys.memory_array_slots = Some(n); }
                        }
                    }
                }
                17 => {
                    if let Some(m) = parse_type17(&buf, offset) {
                        sys.memory_devices.push(m);
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

        // apply cache sizes to CPU
        apply_cache_handles(&mut sys, &cache_map);

        // CPU fallback counts
        if let Some(cpu) = sys.cpu.as_mut() {
            cpu_counts_fallback(cpu);
        }

        // assign memory channels
        assign_memory_channels(&mut sys);

        sys
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

            // build cache_map
            let cache_map = build_cache_map(&buffer);

            let mut offset = 0usize;
            while offset + 4 <= buffer.len() {
                let typ = buffer[offset];
                let len = buffer[offset + 1] as usize;
                if len == 0 { break; }
                if offset + len > buffer.len() { break; }

                match typ {
                    2 => {
                        if sys.board.is_none() {
                            sys.board = parse_type2(&buffer, offset);
                        }
                    }
                    4 => {
                        if sys.cpu.is_none() {
                            sys.cpu = parse_type4(&buffer, offset);
                        }
                    }
                    16 => {
                        if sys.memory_array_slots.is_none() {
                            if let Some(n) = parse_type16(&buffer, offset) {
                                if n > 0 { sys.memory_array_slots = Some(n); }
                            }
                        }
                    }
                    17 => {
                        if let Some(m) = parse_type17(&buffer, offset) {
                            sys.memory_devices.push(m);
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

            // apply caches
            apply_cache_handles(&mut sys, &cache_map);

            // fallback CPU counts (use available_parallelism if needed)
            if let Some(cpu) = sys.cpu.as_mut() {
                cpu_counts_fallback(cpu);
            }

            // assign channels
            assign_memory_channels(&mut sys);
        }
        sys
    }
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
mod platform {
    use super::*;
    pub fn collect_system_info() -> SystemInfo { SystemInfo::default() }
}

/////////////////////
// Public API and helpers (non-printing)
/////////////////////

pub fn get_system_info() -> SystemInfo {
    platform::collect_system_info()
}

impl SystemInfo {
    /// Channel map: ChannelName -> slots in insertion order
    pub fn memory_channels(&self) -> BTreeMap<String, Vec<&MemoryInfo>> {
        let mut map: BTreeMap<String, Vec<&MemoryInfo>> = BTreeMap::new();
        for m in &self.memory_devices {
            if let Some(name) = &m.channel_name {
                map.entry(name.clone()).or_default().push(m);
            } else {
                map.entry("Channel 0".to_string()).or_default().push(m);
            }
        }
        map
    }

    /// Number of channels with at least one populated DIMM
    pub fn populated_channels(&self) -> usize {
        self.memory_channels()
            .values()
            .filter(|slots| slots.iter().any(|s| s.populated))
            .count()
    }

    /// total channels observed
    pub fn total_channels(&self) -> usize {
        self.memory_channels().len()
    }

    pub fn populated_slots(&self) -> usize {
        self.memory_devices.iter().filter(|m| m.populated).count()
    }

    pub fn total_slots(&self) -> usize { self.memory_devices.len() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn smoke() {
        let info = get_system_info();
        // only print populated slots in the Display impl per user request
        println!("{}", info);
    }
}
