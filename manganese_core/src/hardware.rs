// No imports needed here - cpuid handled via module
use log::{error};

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
        error!("Failed to get system firmware table (1) RSMB: {}", size);
        return 0;
    }
    //error!("got system firmware table (1) RSMB: {}, ACPI: {}, FIRM: {}", size, size_ACPI, size_FIRM);

    // Step 2: allocate buffer
    let mut buffer = vec![0u8; size as usize];

    // Step 3: retrieve table
    let ret = unsafe { GetSystemFirmwareTable(provider, 0, Some(&mut buffer[..])) };
    if ret != size {
        error!("Failed to get system firmware table (3)");
        return 0;
    }

    // Step 4: parse Type 17 entries
    let mut offset = 0usize;
    let mut max_speed = 0u16;

    while offset + 4 <= buffer.len() {
        let entry_type = buffer[offset];
        let length = buffer[offset + 1] as usize;

        //error!("RSMBinfo @ {} (/{}): {} / {}", offset, buffer.len(), entry_type, length);
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
// let the ai shenanigans begin, humans see SMBIOS ref spec:
// https://www.dmtf.org/sites/default/files/standards/documents/DSP0134_3.7.0.pdf
// ---
// Robust SMBIOS parser (Linux + Windows) with CPU, Board, Memory parsing.
// Uses smb_next_structure to walk the SMBIOS blob and reliable string lookups.

use std::collections::{BTreeMap, HashMap};
use std::fmt;

#[derive(Debug, Default)]
pub struct SystemInfo {
    pub cpu: Option<CpuInfo>,
    pub board: Option<BoardInfo>,
    pub memory_devices: Vec<MemoryInfo>, // includes recorded slots; populated flag indicates actual module
    /// Type 16 NumberOfDevices (if present and >0)
    pub memory_array_slots: Option<u8>,
    pub hide_serials: bool,
}

#[derive(Debug, Default)]
pub struct CpuInfo {
    pub manufacturer: String,
    pub name: String,
    pub socket: String,

    pub cores: u32,
    pub threads: u32,

    pub l1_kb: u32,
    pub l2_kb: u32,
    pub l3_kb: u32,

    // cache handles from Type 4 -> Type 7 mapping
    pub l1_handle: u16,
    pub l2_handle: u16,
    pub l3_handle: u16,
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
    pub size_mb: u32,
    pub locator: String,
    pub slot_index: Option<u8>,      // trailing digit in locator if any
    pub channel_index: Option<usize>,// assigned channel 0-based
    pub channel_name: Option<String>,
    pub populated: bool,
}

impl fmt::Display for SystemInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(cpu) = &self.cpu {
            writeln!(f, "CPU: {}, Socket {}, {}", cpu.name, cpu.socket, cpu.manufacturer)?;
            // broken readouts
            //writeln!(f, "  Family: {:X}  Model: {:X}  Stepping: {:X}  ExtFam: {:X}  ExtModel: {:X}",
            //         cpu.family, cpu.model, cpu.stepping, cpu.ext_family, cpu.ext_model)?;
            writeln!(f, "  Cores: {}, Threads: {}, L1: {}KB, L2: {}KB, L3: {}KB",
                     cpu.cores, cpu.threads,
                     cpu.l1_kb, cpu.l2_kb, cpu.l3_kb)?;
        } else {
            writeln!(f, "CPU: <unknown>")?;
        }

        if let Some(board) = &self.board {
            if board.version.to_ascii_lowercase() == "Default String".to_ascii_lowercase() {
                if self.hide_serials {
                    writeln!(f, "Board: {} {}", board.manufacturer, board.product)?;
                } else {
                    writeln!(f, "Board: {} {}, Serial: {}", board.manufacturer, board.product, board.serial)?;
                }
            } else {
                if self.hide_serials {
                    writeln!(f, "Board: {} {}, Version: {}", board.manufacturer, board.product, board.version)?;
                } else {
                    writeln!(f, "Board: {} {}, Version: {}, Serial: {}", board.manufacturer, board.product, board.version, board.serial)?;
                }
            }
        }

        if let Some(n) = self.memory_array_slots {
            if n > 0 {
                writeln!(f, "Memory Array slots (Type 16): {}", n)?;
            }
        }

        // show channels, but skip unpopulated slots
        let channels = self.memory_channels();
        let pop_channels = self.populated_channels();
        if !channels.is_empty() {
            writeln!(f, "DRAM Channels observed: {} (populated: {})", channels.len(), pop_channels)?;
            if channels.len() > pop_channels {
                writeln!(f, " WARNING: Not all memory channels seem to be populated, this will degrade performance!\n  Detected {} out of {}",
                         channels.len(), pop_channels)?;
            }
            for (ch, slots) in channels {
                let populated_slots: Vec<&MemoryInfo> = slots.iter().filter(|s| s.populated).cloned().collect();
                if populated_slots.is_empty() { continue; }
                writeln!(f, " {}:", ch)?;
                for (i, m) in populated_slots.iter().enumerate() {
                    writeln!(f, "  Slot {}: {} MB @ {}MT/s (spec at {}MT/s), Locator: {}",
                             i+1, m.size_mb, m.configured_speed, m.speed, m.locator)?;
                    if !m.manufacturer.is_empty() {
                        if self.hide_serials {
                            writeln!(f, "   Manufacturer: {}, Part: {}", m.manufacturer, m.part_number)?;
                        } else {
                            writeln!(f, "   Manufacturer: {}, Part: {}, Serial: {}", m.manufacturer, m.part_number, m.serial)?;
                        }
                    }
                }
            }
        } else {
            writeln!(f, "Memory: <none discovered>")?;
        }

        Ok(())
    }
}

////////////////////
// basic readers
////////////////////

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

/// Find start index of next SMBIOS structure (skip formatted area + strings)
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

/// Read a SMBIOS string (1-based index) for structure starting at struct_start in buffer
fn get_smbios_string(buf: &[u8], struct_start: usize, index: u8) -> Option<String> {
    if index == 0 { return None; }
    let struct_len = *buf.get(struct_start + 1)? as usize;
    let mut p = struct_start + struct_len;
    if p >= buf.len() { return None; }
    let mut cur = 1u8;
    while p < buf.len() {
        let end = match buf[p..].iter().position(|&c| c == 0) {
            Some(pos) => p + pos,
            None => return None,
        };
        if cur == index {
            let s = String::from_utf8_lossy(&buf[p..end]).to_string();
            return Some(s.trim().trim_end_matches(char::from(0)).to_string());
        }
        cur = cur.saturating_add(1);
        p = end + 1;
        if p < buf.len() && buf[p] == 0 {
            break;
        }
    }
    None
}

////////////////////
// structure parsers
////////////////////

fn parse_type2_board(buf: &[u8], offset: usize) -> Option<BoardInfo> {
    let struct_len = *buf.get(offset + 1)? as usize;
    if offset + struct_len > buf.len() { return None; }
    let man_idx = *buf.get(offset + 0x04).unwrap_or(&0);
    let prod_idx = *buf.get(offset + 0x05).unwrap_or(&0);
    let ver_idx = *buf.get(offset + 0x06).unwrap_or(&0);
    let ser_idx = *buf.get(offset + 0x07).unwrap_or(&0);
    Some(BoardInfo {
        manufacturer: get_smbios_string(buf, offset, man_idx).unwrap_or_default(),
        product: get_smbios_string(buf, offset, prod_idx).unwrap_or_default(),
        version: get_smbios_string(buf, offset, ver_idx).unwrap_or_default(),
        serial: get_smbios_string(buf, offset, ser_idx).unwrap_or_default(),
    })
}

fn parse_type4_cpu(buf: &[u8], offset: usize) -> Option<CpuInfo> {
    let struct_len = *buf.get(offset + 1)? as usize;
    if offset + struct_len > buf.len() { return None; }

    let man_idx = *buf.get(offset + 0x07).unwrap_or(&0);
    let name_idx = *buf.get(offset + 0x10).unwrap_or(&0);
    let socket_idx = *buf.get(offset + 0x04).unwrap_or(&0);

    /*FIXME: detection logic here is off, should be improved before printing it
    // family/model/stepping (bytes)
    let family = *buf.get(offset + 0x06).unwrap_or(&0) as u16;
    let model = *buf.get(offset + 0x07).unwrap_or(&0) as u16;
    let stepping = *buf.get(offset + 0x08).unwrap_or(&0) as u16;

    // extended fields
    let ext_model = *buf.get(offset + 0x27).unwrap_or(&0) as u16;
    let ext_family = *buf.get(offset + 0x28).unwrap_or(&0) as u16;
     */

    // cache handles (words) at 0x1A,0x1C,0x1E
    let l1_handle = le_u16_at(buf, offset + 0x1A);
    let l2_handle = le_u16_at(buf, offset + 0x1C);
    let l3_handle = le_u16_at(buf, offset + 0x1E);

    // cores/threads (SMBIOS may store at 0x23 and 0x25 as bytes or words depending on version)
    let cores = {
        let b = *buf.get(offset + 0x23).unwrap_or(&0);
        if b != 0 && b != 0xFF { b as u32 } else { 0 }
    };
    let threads = {
        let b = *buf.get(offset + 0x25).unwrap_or(&0);
        if b != 0 && b != 0xFF { b as u32 } else { 0 }
    };

    Some(CpuInfo {
        manufacturer: get_smbios_string(buf, offset, man_idx).unwrap_or_default(),
        name: get_smbios_string(buf, offset, name_idx).unwrap_or_default(),
        socket: get_smbios_string(buf, offset, socket_idx).unwrap_or_default(),
        cores,
        threads,
        l1_kb: 0,
        l2_kb: 0,
        l3_kb: 0,
        l1_handle,
        l2_handle,
        l3_handle,
    })
}

fn parse_type7_cache(buf: &[u8], offset: usize) -> Option<(u16, u32, u8, u16)> {
    // return (handle, size_kb, level, associativity)
    let struct_len = *buf.get(offset + 1)? as usize;
    if offset + struct_len > buf.len() { return None; }
    let handle = le_u16_at(buf, offset + 2);
    // installed size at offset 0x09 (word), bit15 = granularity
    let installed = le_u16_at(buf, offset + 0x09);
    let gran = (installed & 0x8000) != 0;
    let raw = (installed & 0x7FFF) as u32;
    let size_kb = if raw == 0 { 0 } else { if gran { raw * 64 } else { raw } };
    // cache level in Cache Configuration (offset 0x05) bits 2:0
    let cfg = le_u16_at(buf, offset + 0x05);
    let level = (cfg & 0x7) as u8;
    // associativity (offset 0x07) - keep as raw
    let assoc = le_u16_at(buf, offset + 0x07);
    Some((handle, size_kb, level, assoc))
}

fn parse_type16_array(buf: &[u8], offset: usize) -> Option<u8> {
    Some(*buf.get(offset + 0x0E).unwrap_or(&0))
}

fn parse_type17_memory(buf: &[u8], offset: usize) -> Option<MemoryInfo> {
    let struct_len = *buf.get(offset + 1)? as usize;
    if offset + struct_len > buf.len() { return None; }

    let size_word = le_u16_at(buf, offset + 0x0C);
    // 0 or 0xFFFF -> not present/unknown
    let size_mb = if size_word == 0 || size_word == 0xFFFF {
        0u32
    } else if size_word == 0x7FFF {
        // extended size at 0x1C..0x1F (DWORD)
        le_u32_at(buf, offset + 0x1C)
    } else {
        size_word as u32
    };

    let locator_idx = *buf.get(offset + 0x10).unwrap_or(&0);
    let manufacturer_idx = *buf.get(offset + 0x17).unwrap_or(&0);
    let serial_idx = *buf.get(offset + 0x18).unwrap_or(&0);
    let part_idx = *buf.get(offset + 0x1A).unwrap_or(&0);

    let speed = le_u16_at(buf, offset + 0x15);
    let configured = le_u16_at(buf, offset + 0x20);

    let locator = get_smbios_string(buf, offset, locator_idx).unwrap_or_default();
    let manufacturer = get_smbios_string(buf, offset, manufacturer_idx).unwrap_or_default();
    let part = get_smbios_string(buf, offset, part_idx).unwrap_or_default();
    let serial = get_smbios_string(buf, offset, serial_idx).unwrap_or_default();

    // slot index = trailing digits of locator if any
    let slot_index = parse_slot_index(&locator);

    let populated = size_mb > 0;

    Some(MemoryInfo {
        speed,
        configured_speed: configured,
        manufacturer,
        part_number: part,
        serial,
        size_mb,
        locator,
        slot_index,
        channel_index: None,
        channel_name: None,
        populated,
    })
}

fn parse_slot_index(locator: &str) -> Option<u8> {
    let s = locator.trim();
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

////////////////////
// channel assignment & cache application helpers
////////////////////

fn assign_memory_channels(sys: &mut SystemInfo) {
    // count occurrences per slot_index and per locator string
    let mut counts_by_slot: HashMap<u8, usize> = HashMap::new();
    let mut counts_by_name: HashMap<String, usize> = HashMap::new();

    for m in &sys.memory_devices {
        if let Some(idx) = m.slot_index {
            *counts_by_slot.entry(idx).or_insert(0) += 1;
        } else {
            *counts_by_name.entry(m.locator.clone()).or_insert(0) += 1;
        }
    }

    let mut max_channels = counts_by_slot.values().copied().max().unwrap_or(0);
    max_channels = max_channels.max(counts_by_name.values().copied().max().unwrap_or(0));
    if max_channels == 0 { max_channels = 1; }

    let mut seen_slot: HashMap<u8, usize> = HashMap::new();
    let mut seen_name: HashMap<String, usize> = HashMap::new();

    for m in sys.memory_devices.iter_mut() {
        let channel = if let Some(idx) = m.slot_index {
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
        m.channel_index = Some(channel);
        let ch_name = if max_channels <= 26 {
            let letter = (b'A' + (channel as u8)).min(b'Z') as char;
            format!("Channel {}", letter)
        } else {
            format!("Channel {}", channel)
        };
        m.channel_name = Some(ch_name);
    }
}

fn build_cache_map(buf: &[u8]) -> HashMap<u16, (u32, u8, u16)> {
    // handle -> (size_kb, level, associativity)
    let mut map = HashMap::new();
    let mut offset = 0usize;
    while offset + 4 <= buf.len() {
        let typ = buf[offset];
        let len = buf[offset + 1] as usize;
        if len == 0 { break; }
        if offset + len > buf.len() { break; }

        if typ == 7 {
            if let Some((handle, size_kb, level, assoc)) = parse_type7_cache(buf, offset) {
                map.insert(handle, (size_kb, level, assoc));
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

fn apply_cache_handles(sys: &mut SystemInfo, cache_map: &HashMap<u16,(u32,u8,u16)>) {
    if let Some(cpu) = sys.cpu.as_mut() {
        if cpu.l1_handle != 0 && cpu.l1_handle != 0xFFFF {
            if let Some(&(size, _level, _)) = cache_map.get(&cpu.l1_handle) {
                cpu.l1_kb = size; // best-effort; accurate split L1I/L1D requires examining Type7 L1 descriptors
            }
        }
        if cpu.l2_handle != 0 && cpu.l2_handle != 0xFFFF {
            if let Some(&(size, _level, _)) = cache_map.get(&cpu.l2_handle) {
                cpu.l2_kb = size;
            }
        }
        if cpu.l3_handle != 0 && cpu.l3_handle != 0xFFFF {
            if let Some(&(size, _level, _)) = cache_map.get(&cpu.l3_handle) {
                cpu.l3_kb = size;
            }
        }
    }
}

////////////////////
// platform-specific SMBIOS read & top-level collection
////////////////////

#[cfg(target_os = "linux")]
fn load_smbios_table() -> Option<Vec<u8>> {
    use glob::glob;
    use std::fs::File;
    use std::io::Read;
    let mut buf = Vec::new();
    for entry in glob("/sys/firmware/dmi/entries/*/raw").ok()? {
        if let Ok(path) = entry {
            if let Ok(mut f) = File::open(path) {
                let mut tmp = Vec::new();
                if f.read_to_end(&mut tmp).is_ok() {
                    buf.extend_from_slice(&tmp);
                }
            }
        }
    }
    if buf.is_empty() { None } else { Some(buf) }
}

#[cfg(target_os = "windows")]
fn load_smbios_table() -> Option<Vec<u8>> {
    use windows::Win32::System::SystemInformation::{GetSystemFirmwareTable, RSMB};
    let provider = RSMB;
    let size = unsafe { GetSystemFirmwareTable(provider, 0, None) };
    if size == 0 { return None; }
    let mut buffer = vec![0u8; size as usize];
    let ret = unsafe { GetSystemFirmwareTable(provider, 0, Some(&mut buffer[..])) };
    if ret != size { return None; }
    Some(buffer)
}

pub fn collect_system_info() -> SystemInfo {
    let mut sys = SystemInfo::default();
    let buf = match load_smbios_table() {
        Some(b) => b,
        None => return sys,
    };

    // Build cache map first
    let cache_map = build_cache_map(&buf);

    // Walk table using smb_next_structure to parse each structure reliably
    let mut offset = 0usize;
    while offset + 4 <= buf.len() {
        let typ = buf[offset];
        let len = buf[offset + 1] as usize;
        if len == 0 { break; }
        if offset + len > buf.len() { break; }

        match typ {
            2 => { // Baseboard
                if sys.board.is_none() {
                    if let Some(b) = parse_type2_board(&buf, offset) {
                        sys.board = Some(b);
                    }
                }
            }
            4 => { // Processor
                if sys.cpu.is_none() {
                    if let Some(c) = parse_type4_cpu(&buf, offset) {
                        sys.cpu = Some(c);
                    }
                }
            }
            7 => { /* already processed in cache_map */ }
            16 => {
                if sys.memory_array_slots.is_none() {
                    if let Some(n) = parse_type16_array(&buf, offset) {
                        if n > 0 { sys.memory_array_slots = Some(n); }
                    }
                }
            }
            17 => {
                if let Some(m) = parse_type17_memory(&buf, offset) {
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

    // Assign caches from cache_map to CPU
    apply_cache_handles(&mut sys, &cache_map);

    // Try to fill cores/threads if missing using available_parallelism fallback (platform-specific enhancements can be added)
    if let Some(cpu) = sys.cpu.as_mut() {
        if cpu.threads == 0 {
            if let Ok(n) = std::thread::available_parallelism() {
                cpu.threads = n.get() as u32;
            }
        }
        if cpu.cores == 0 && cpu.threads > 0 {
            cpu.cores = std::cmp::max(1, cpu.threads / 2);
        }
    }

    // Assign memory channels
    assign_memory_channels(&mut sys);

    sys
}

////////////////////
// SystemInfo helper methods (channels/populated)
////////////////////

impl SystemInfo {
    pub fn memory_channels(&self) -> BTreeMap<String, Vec<&MemoryInfo>> {
        let mut map: BTreeMap<String, Vec<&MemoryInfo>> = BTreeMap::new();
        for m in &self.memory_devices {
            let name = m.channel_name.clone().unwrap_or_else(|| "Channel 0".to_string());
            map.entry(name).or_default().push(m);
        }
        map
    }

    pub fn populated_channels(&self) -> usize {
        self.memory_channels().values().filter(|slots| slots.iter().any(|s| s.populated)).count()
    }

    #[allow(dead_code)]
    pub fn total_channels(&self) -> usize {
        self.memory_channels().len()
    }

    #[allow(dead_code)]
    pub fn populated_slots(&self) -> usize {
        self.memory_devices.iter().filter(|m| m.populated).count()
    }

    #[allow(dead_code)]
    pub fn total_slots(&self) -> usize {
        self.memory_devices.len()
    }
}

#[cfg(test)]
mod tests {
    use log::info;
    use super::*;
    #[test]
    fn smoke_collect() {
        let info = collect_system_info();
        info!("{:#?}", info);
        info!("{}", info);
    }
}
