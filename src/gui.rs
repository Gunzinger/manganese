// src/gui.rs
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    Mutex,
};
use std::thread;

use eframe::{egui, run_native, NativeOptions};
use egui::{CentralPanel, TextEdit, ScrollArea};

use sysinfo::{System, RefreshKind, MemoryRefreshKind};
use manganese_core::{parse_ram_spec, RamSpec, run_tests};

pub fn launch_gui() -> eframe::Result<()> {
    let native_options = NativeOptions::default();
    run_native(
        "Manganese RAM Tester",
        native_options,
        Box::new(|_cc| Ok(Box::new(GuiApp::default()))),
    )
}

struct GuiApp {
    ram_input: String,
    hide_serials: bool,
    running: bool,
    stop_flag: Arc<AtomicBool>,
    status: String,
    log: Arc<Mutex<Vec<String>>>,
    allocated_bytes: Option<usize>,
}

impl Default for GuiApp {
    fn default() -> Self {
        Self {
            ram_input: "".to_owned(),
            hide_serials: false,
            running: false,
            stop_flag: Arc::new(AtomicBool::new(false)),
            status: "Idle".to_owned(),
            log: Arc::new(Mutex::new(Vec::new())),
            allocated_bytes: None,
        }
    }
}

impl eframe::App for GuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        CentralPanel::default().show(ctx, |ui| {
            ui.heading("Manganese RAM Tester");

            ui.horizontal(|ui| {
                ui.label("RAM to test:");
                ui.add(
                    TextEdit::singleline(&mut self.ram_input)
                        .hint_text("e.g. 4GiB, 50%, 10%t"),
                );
            });

            ui.checkbox(&mut self.hide_serials, "Hide serials");

            if !self.running {
                if ui.button("Start").clicked() {
                    // Parse input & compute ram_bytes
                    let mut sys = System::new_with_specifics(
                        RefreshKind::everything(),
                    );
                    sys.refresh_memory();
                    let total = sys.total_memory() as usize;
                    let avail = sys.available_memory() as usize;

                    let ram_bytes = match parse_ram_spec(&self.ram_input, avail, total) {
                        Some(RamSpec::Bytes(b)) => b,
                        Some(RamSpec::Percent(fr, true)) => (total as f64 * fr) as usize,
                        Some(RamSpec::Percent(fr, false)) => (avail as f64 * fr) as usize,
                        None => {
                            self.status = format!("Invalid RAM spec: {}", self.ram_input);
                            return;
                        }
                    };

                    self.running = true;
                    self.stop_flag.store(false, Ordering::SeqCst);
                    self.status = "Running...".to_owned();
                    self.allocated_bytes = Some(ram_bytes);

                    let stop = self.stop_flag.clone();
                    let hide_serials = self.hide_serials;
                    let log_buf = self.log.clone();

                    // Clear old log
                    {
                        let mut log = log_buf.lock().unwrap();
                        log.clear();
                    }

                    // Spawn background test thread
                    thread::spawn(move || {
                        // Here: you need to adapt run_tests to accept a log callback
                        run_tests(ram_bytes, !hide_serials, &stop);

                        // For example: after finishing
                        let mut log = log_buf.lock().unwrap();
                        log.push(format!("Test finished.\n"));
                    });
                }
            } else {
                if ui.button("Stop").clicked() {
                    self.stop_flag.store(true, Ordering::SeqCst);
                    self.status = "Stopping...".to_owned();
                }
            }

            ui.separator();

            if let Some(bytes) = self.allocated_bytes {
                ui.label(format!("Requested allocation: {} MiB", bytes / (1024 * 1024)));
            }

            ui.label(format!("Status: {}", self.status));

            ui.separator();
            ui.label("Log:");
            ScrollArea::vertical().show(ui, |ui| {
                let log = self.log.lock().unwrap();
                for line in log.iter() {
                    ui.label(line);
                }
            });
        });

        ctx.request_repaint();
    }
}
