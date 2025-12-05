#![cfg_attr(feature = "gui", windows_subsystem = "windows")]

use clap::Parser;
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool};
#[cfg(not(feature = "gui"))]
use std::io::IsTerminal;
#[cfg(not(feature = "gui"))]
use std::env;
#[cfg(not(feature = "gui"))]
use std::process::Command;

use sysinfo::{System, RefreshKind};

use manganese_core::{parse_ram_spec, RamSpec, run_tests};

use simplelog::{SimpleLogger, ConfigBuilder};
use log::{error, info, LevelFilter as LogLevelFilter};

fn init_cli_logger() {
    let config = ConfigBuilder::new()
        .build();
    SimpleLogger::init(LogLevelFilter::Info, config).unwrap();
}

#[cfg(feature = "gui")]
mod gui;

#[derive(Parser)]
struct Args {
    ram: Option<String>,
    #[arg(long)]
    hide_serials: bool,
    headless: bool,
}

fn main() {
    let args = Args::parse();

    // GUI fallback if enabled and no terminal is present
    #[cfg(feature = "gui")]
    if !args.headless {
        gui::launch_gui().expect("ERROR: gui crashed");
        return;
    }

    // CLI-only fallback
    #[cfg(not(feature = "gui"))]
    if ! io::stdout().is_terminal() {
        spawn_terminal();
    }

    run_cli(args);
}

fn run_cli(args: Args) {
    // Refresh memory using sysinfo 0.37 API
    let mut sys = System::new_with_specifics(
        RefreshKind::everything()
    );

    sys.refresh_memory();

    init_cli_logger();

    info!("manganese v{} 🎉", env!("CARGO_PKG_VERSION"));

    let ram_input = args.ram.unwrap_or_else(|| {
        error!("usage: manganese [0%-99%|4GiB|8%t|300MiB]");
        error!("where the input is an SI size, % of free RAM, or %t of total RAM.");

        info!(
            "Total RAM: {}MiB, available: {}MiB ({:.2}%)",
            sys.total_memory() / 1024 / 1024,
            sys.available_memory() / 1024 / 1024,
            (sys.available_memory() as f64 / sys.total_memory() as f64) * 100.
        );

        print!("Please enter arguments: ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        input.trim().to_string()
    });

    // Parse RAM specification
    let ram_bytes = match parse_ram_spec(
        &ram_input,
        sys.available_memory() as usize,
        sys.total_memory() as usize,
    ) {
        Some(RamSpec::Bytes(b)) => b,
        Some(RamSpec::Percent(frac, true)) => (sys.total_memory() as f64 * frac) as usize,
        Some(RamSpec::Percent(frac, false)) => (sys.available_memory() as f64 * frac) as usize,
        None => {
            error!("Invalid RAM specification: \"{}\"", ram_input);
            std::process::exit(1);
        }
    };

    let stop_signal = AtomicBool::new(false);

    run_tests(ram_bytes, args.hide_serials, &stop_signal);
}

#[cfg(not(feature = "gui"))]
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
            error!("Could not spawn a terminal. Please run this CLI from a terminal manually.");
        }
    }
}
