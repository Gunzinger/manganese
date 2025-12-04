use std::{env, io};
use std::io::{IsTerminal, Write};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

mod hardware;
mod platform;
mod simd_xorshift;
mod tests_avx2;
mod tests_avx512;
mod tests;

use hardware::{hardware_cpu_count, hardware_instruction_set, hardware_is_needlessly_disabled, hardware_ram_speed, InstructionSet};
use platform::{getpagesize, mlock, sysinfo, aligned_alloc, aligned_free};
use tests::tests_init;

static ERRORS: AtomicU64 = AtomicU64::new(0);

fn main() {
    if ! io::stdout().is_terminal() {
        // if we dont detect that we are in a terminal, spawn a new one :)
        spawn_terminal();
    }

    let args: Vec<String> = env::args().collect();

    let input = if args.len() > 1 {
        // Arguments provided
        args[1..].join(" ")
    } else {
        // No arguments: prompt user
        eprintln!("usage: manganese [0%-99%]");
        eprintln!("where the input % is the amount of total installed ram that should be tested");
        print!("Please enter arguments: ");
        io::stdout().flush().unwrap(); // flush so the prompt shows
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        input.trim().to_string()
    };

    let fraction = input
        .trim_end_matches('%')
        .parse::<f64>()
        .unwrap_or(0.0) / 100.0;

    let cpu_count = hardware_cpu_count();
    let ram_speed = hardware_ram_speed(true);
    let actual_ram_speed = hardware_ram_speed(false);
    let isa = hardware_instruction_set();

    if matches!(isa, InstructionSet::SSE) {
        eprintln!("AVX2 or AVX-512 not available, aborting");
        std::process::exit(-1);
    }

    let sys = sysinfo();
    let total_alloc = (sys.totalram as f64 * fraction) as usize;
    let alignment = cpu_count * getpagesize();
    let total_alloc = total_alloc - (total_alloc % alignment);

    const BACKOFF: usize = 256 * 1024 * 1024;
    let mut mem: Option<*mut u8> = None;
    let mut size = 0;

    for i in 0..=(total_alloc / BACKOFF) {
        let alloc_size = total_alloc - i * BACKOFF;
        if alloc_size == 0 {
            break;
        }
        
        unsafe {
            //eprintln!("Trying to alloc memory: {}", alloc_size);
            let ptr = aligned_alloc(alignment, alloc_size);
            if ptr.is_null() {
                continue;
            }

            if mlock(ptr, alloc_size) == 0 {
                let hw_info = hardware::collect_system_info();
                eprintln!("Hardware information:\n{}", hw_info);
                eprintln!("Available Threads : {}", cpu_count);
                if ram_speed > 0 {
                    if actual_ram_speed > 0 && actual_ram_speed != ram_speed {
                        eprintln!("Memory Bandwidth  : {}MB/s (maximum, theoretical)",
                                  8 * actual_ram_speed * hw_info.populated_channels() as u64);
                    } else {
                        // runs at spec or actual speed field missing
                        eprintln!("Memory Bandwidth ?: {}MB/s (maximum, theoretical)",
                                  8 * ram_speed * hw_info.populated_channels() as u64);
                    }
                }
                eprintln!(
                    "Locked Memory     : {}MB of {}MB ({:.0}%)",
                    alloc_size / (1024 * 1024),
                    sys.totalram / (1024 * 1024),
                    100.0 * alloc_size as f64 / sys.totalram as f64
                );
                eprintln!("Chunk Alignment   : {}K", alignment / 1024);
                match isa {
                    InstructionSet::AVX512 => eprintln!("Instruction Set   : AVX-512"),
                    InstructionSet::AVX2 => {
                        if hardware_is_needlessly_disabled() {
                            eprintln!("Instruction Set   : AVX2 (lol)");
                        } else {
                            eprintln!("Instruction Set   : AVX2");
                        }
                    }
                    _ => {}
                }
                eprintln!();
                
                mem = Some(ptr);
                size = alloc_size;
                break;
            } else {
                eprintln!("Failed to mlock memory, try root (linux) or granting SeLockMemoryPrivilege (windows)!");
                aligned_free(ptr);
            }
        }
    }

    if mem.is_none() {
        eprintln!("can't lock any memory; try increasing memlock ulimit or running as root");
        std::process::exit(-1);
    }

    let mem_ptr = mem.unwrap();
    let tests = tests_init(cpu_count, &ERRORS, isa);

    loop {
        let start = Instant::now();

        for test in &tests {
            println!("Running: {}", test.name);
            unsafe {
                (test.run)(mem_ptr, size);
            }
        }

        let elapsed = start.elapsed();
        let total_time = elapsed.as_secs_f64();
        
        let total_passes: usize = tests.iter()
            .map(|t| t.passes * t.iters)
            .sum();
        
        let bandwidth = (total_passes as f64 * (size as f64 / (1024.0 * 1024.0))) / total_time;
        eprintln!("Tests completed in {:.2} sec [{:.0}MB/s]", total_time, bandwidth);
        
        let errors = ERRORS.load(Ordering::Relaxed);
        if errors > 0 {
            eprintln!("\x1b[1;91m{} errors detected\x1b[0m", errors);
        }
    }
}

fn spawn_terminal() {
    let exe_path = env::current_exe().unwrap();
    let exe_str = exe_path.to_str().unwrap();

    #[cfg(target_os = "windows")]
    {
        // windows: spawn powershell
        Command::new("powershell")
            .args(&["-NoExit", "-Command", &format!("& '{}'", exe_str)])
            .spawn()
            .expect("Failed to spawn terminal");
    }

    #[cfg(target_os = "macos")]
    {
        // macOS: use AppleScript to open Terminal.app
        Command::new("osascript")
            .args(&[
                "-e",
                &format!("tell application \"Terminal\" to do script \"{}\"", exe_str),
            ])
            .spawn()
            .expect("Failed to spawn terminal");
    }

    #[cfg(target_os = "linux")]
    {
        // Linux: try common terminals (gnome-terminal, konsole, xterm)
        let terminals = ["gnome-terminal", "konsole", "xterm"];
        let mut spawned = false;

        for term in &terminals {
            if Command::new(term).args(&["-e", exe_str]).spawn().is_ok() {
                spawned = true;
                break;
            }
        }

        if !spawned {
            eprintln!("Could not spawn a terminal. Please run this CLI from a terminal manually.");
        }
    }
}
