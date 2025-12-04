// src/gui.rs
use std::sync::{atomic::{AtomicBool, Ordering}, Arc};
use std::thread;

use eframe::{egui, run_native, NativeOptions};
use egui::{CentralPanel, TextEdit};

use manganese_core::{parse_ram_spec, run_tests, RamSpec};
use sysinfo::{RefreshKind, System};

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
}

impl Default for GuiApp {
    fn default() -> Self {
        Self {
            ram_input: "".to_owned(),
            hide_serials: false,
            running: false,
            stop_flag: Arc::new(AtomicBool::new(false)),
            status: "Idle".to_owned(),
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
                    // Parse input
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
                    let stop = self.stop_flag.clone();
                    let hide = self.hide_serials;

                    // Spawn background test
                    thread::spawn(move || {
                        run_tests(ram_bytes, !hide, &stop);
                    });
                }
            } else {
                if ui.button("Stop").clicked() {
                    self.stop_flag.store(true, Ordering::SeqCst);
                    self.status = "Stopping...".to_owned();
                }
            }

            ui.separator();
            ui.label("Status:");
            ui.label(&self.status);
        });

        // keep repainting to allow status updates
        ctx.request_repaint();
    }
}
