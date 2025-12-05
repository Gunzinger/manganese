mod hardware;
mod platform;
mod simd_xorshift;
mod tests;
mod tests_avx2;
mod tests_avx512;

use std::sync::atomic::{AtomicUsize, AtomicBool, Ordering};
use std::time::Instant;
use log::info;

pub static ERRORS: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Clone, Copy)]
pub enum RamSpec {
    Percent(f64, bool), // fraction, is_total
    Bytes(usize),
}

pub fn parse_ram_spec(input: &str, free_ram: usize, total_ram: usize) -> Option<RamSpec> {
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
pub fn run_tests(ram_bytes: usize, show_serials: bool, stop_signal: &AtomicBool) {
    if show_serials {
        info!("Hardware information: <SMBIOS INFO HERE>");
    }
    info!("Testing {} bytes of RAM...", ram_bytes);

    let start = Instant::now();
    while !stop_signal.load(Ordering::Relaxed) {
        std::thread::sleep(std::time::Duration::from_millis(500));
        let elapsed = start.elapsed().as_secs_f64();
        info!("Elapsed {:.2}s", elapsed);
    }
    info!("Test stopped after {:.2}s", start.elapsed().as_secs_f64());
}
