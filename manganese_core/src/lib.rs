mod hardware;
mod platform;
mod simd_xorshift;
mod tests;
mod tests_avx2;
mod tests_avx512;
mod config;

use std::sync::atomic::{AtomicBool, Ordering, AtomicU64};
use std::time::Instant;
use log::{error, info, warn};
use crate::config::{build_tests_from_config, load_custom_config};
pub use crate::hardware::{hardware_cpu_count, hardware_instruction_set, hardware_is_needlessly_disabled, hardware_ram_speed, InstructionSet};
pub use crate::platform::{aligned_alloc, aligned_free, getpagesize, mlock, sysinfo};
use crate::tests::{tests_init};

pub static ERRORS: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy)]
pub enum RamSpec {
    Percent(f64, bool), // fraction, is_total
    Bytes(usize),
}

pub fn parse_ram_spec(input: &str) -> Option<RamSpec> {
    let input = input.trim().to_uppercase();

    if input.ends_with("T") && input.contains('%') { // % of total
        let num = input.trim_end_matches("%T").parse::<f64>().ok()?;
        Some(RamSpec::Percent(num / 100.0, true))
    } else if input.ends_with('%') { // % of free
        let num = input.trim_end_matches('%').parse::<f64>().ok()?;
        Some(RamSpec::Percent(num / 100.0, false))
    } else { // SI suffix
        let multipliers = [
            ("KI", 1024),
            ("K", 1000),
            ("MI", 1024*1024),
            ("M", 1000*1000),
            ("GI", 1024*1024*1024),
            ("G", 1000*1000*1000),
        ];

        let mut number_str = input.as_str();
        let mut multiplier = 1;

        for (suffix, mult) in &multipliers {
            if let Some(s) = input.strip_suffix(suffix) {
                number_str = s;
                multiplier = *mult;
                break;
            }
        }

        let bytes = number_str.parse::<f64>().ok()? * (multiplier as f64);
        Some(RamSpec::Bytes(bytes as usize))
    }
}

// Placeholder for memory allocation and test loop
pub fn run_tests(ram_bytes: usize, hide_serials: bool, stop_signal: &AtomicBool) {
    let sys = sysinfo();
    let mut smbios_info = hardware::collect_system_info();
    smbios_info.hide_serials = hide_serials;

    let cpu_count = hardware_cpu_count();
    let ram_speed = hardware_ram_speed(true);
    let actual_ram_speed = hardware_ram_speed(false);
    let isa = hardware_instruction_set();

    if matches!(isa, InstructionSet::SSE) {
        error!("AVX2 or AVX-512 not available, aborting");
        std::process::exit(-1);
    }

    info!("Hardware information:\n{}", smbios_info);
    info!("Available Threads : {}", cpu_count);
    if ram_speed > 0 {
        if actual_ram_speed > 0 && actual_ram_speed != ram_speed {
            info!("Memory Bandwidth  : {}MB/s (maximum, theoretical)",
                     8 * actual_ram_speed * smbios_info.populated_channels() as u64);
        } else {
            // runs at spec or actual speed field missing
            info!("Memory Bandwidth ?: {}MB/s (maximum, theoretical)",
                     8 * ram_speed * smbios_info.populated_channels() as u64);
        }
    }

    let alignment = cpu_count * getpagesize();
    let ram_bytes = ram_bytes - (ram_bytes % alignment);

    const BACKOFF: usize = 256 * 1024 * 1024;
    let mut mem: Option<*mut u8> = None;
    let mut size = 0;

    for i in 0..=(ram_bytes / BACKOFF) {
        if stop_signal.load(Ordering::SeqCst) {
            break;
        }
        let alloc_size = ram_bytes - i * BACKOFF;
        if alloc_size == 0 {
            break;
        }

        unsafe {
            //error!("Trying to alloc memory: {}", alloc_size);
            let ptr = aligned_alloc(alignment, alloc_size);
            if ptr.is_null() {
                continue;
            }

            if mlock(ptr, alloc_size) == 0 {
                info!(
                    "Locked Memory     : {}MiB of {}MiB ({:.0}%)",
                    alloc_size / (1024 * 1024),
                    sys.totalram / (1024 * 1024),
                    100.0 * alloc_size as f64 / sys.totalram as f64
                );
                info!("Chunk Alignment   : {}K", alignment / 1024);
                match isa {
                    InstructionSet::AVX512 => info!("Instruction Set   : AVX-512"),
                    InstructionSet::AVX2 => {
                        if hardware_is_needlessly_disabled() {
                            info!("Instruction Set   : AVX2 (lol)");
                        } else {
                            info!("Instruction Set   : AVX2");
                        }
                    }
                    _ => {}
                }

                mem = Some(ptr);
                size = alloc_size;
                break;
            } else {
                error!("Failed to mlock memory, try root (linux) or granting SeLockMemoryPrivilege (windows)!");
                aligned_free(ptr);
            }
        }
    }

    if mem.is_none() {
        error!("can't lock any memory; try increasing memlock ulimit or running as root");
        std::process::exit(-1);
    }

    let mem_ptr = mem.unwrap();
    let entries = load_custom_config("manganese.conf").unwrap_or_else(|_| {
        warn!("config file manganese.conf not found! using defaults...");
        vec![]
    });
    let test_config = build_tests_from_config(&entries, isa);
    tests_init(cpu_count, &ERRORS, isa);
    info!("Testing {:.2}MiB bytes of RAM...", ram_bytes as f64 / (1024. * 1024.));
    let start = Instant::now();
    loop {
        let loop_start = Instant::now();
        let mut test_start: Instant;
        for test in &test_config {
            // check if we should stop before starting the next test
            if stop_signal.load(Ordering::SeqCst) {
                break;
            }
            if test.loops > 1 {
                info!("Running: {} ({}x)", test.name, test.loops);
            } else if test.loops == 0 {
                info!("Skipping: {}", test.name);
            } else {
                info!("Running: {}", test.name);
            }

            test_start = Instant::now();
            let mut bandwidth: f64;
            for i in 1..(test.loops+1) {
                if stop_signal.load(Ordering::SeqCst) {
                    break;
                }
                unsafe {
                    (test.run)(mem_ptr, size);
                }
                if i < test.loops {
                    bandwidth = (test.passes * test.iters * i) as f64 * (size as f64 / (1000. * 1000.)) / test_start.elapsed().as_secs_f64();
                    info!("... {} ({}/{}) [avg. BW {:.0}MB/s] ...",
                        test.name,
                        i, test.loops,
                        bandwidth);
                }
            }
            bandwidth = (test.passes * test.iters * test.loops) as f64 * (size as f64 / (1000. * 1000.)) / test_start.elapsed().as_secs_f64();
            info!("{} completed in {:.2} sec [avg. BW {:.0}MB/s]", test.name, test_start.elapsed().as_secs_f64(), bandwidth);
        }

        let errors = ERRORS.load(Ordering::Relaxed);
        if errors > 0 {
            error!("\x1b[1;91m{} errors detected\x1b[0m", errors);
        }

        // if we break in the loop, we need ot break the outer one too
        if stop_signal.load(Ordering::SeqCst) {
            break;
        }

        let elapsed = loop_start.elapsed();
        let total_time = elapsed.as_secs_f64();

        let total_passes: usize = test_config.iter()
            .map(|t| t.passes * t.iters * t.loops)
            .sum();

        let bandwidth = (total_passes as f64 * (size as f64 / (1000.0 * 1000.0))) / total_time;
        info!("Tests completed in {:.2} sec [{:.0}MB/s]", total_time, bandwidth);
    }
    info!("Test stopped after {:.2}s", start.elapsed().as_secs_f64());
}
