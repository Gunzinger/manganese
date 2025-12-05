// src/gui.rs
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
    Mutex,
};
use std::thread;

use eframe::{egui, run_native, NativeOptions};
use egui::{CentralPanel, ScrollArea, TextEdit, Context, FontDefinitions, FontFamily, ViewportBuilder, Color32};

use manganese_core::{parse_ram_spec, run_tests, RamSpec};
use sysinfo::{RefreshKind, System};

use log::{LevelFilter, Log, Metadata, Record, SetLoggerError};

struct GuiLogger {
    buffer: Arc<Mutex<String>>,
}

impl Log for GuiLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= LevelFilter::Info
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let mut buf = self.buffer.lock().unwrap();
            buf.push_str(&format!("[{}] {}\n", record.level(), record.args()));
        }
    }

    fn flush(&self) {}
}

fn init_gui_logger(buffer: Arc<Mutex<String>>) -> Result<(), SetLoggerError> {
    log::set_boxed_logger(Box::new(GuiLogger { buffer }))
        .map(|()| log::set_max_level(LevelFilter::Info))
}

pub fn launch_gui() -> eframe::Result<()> {
    let native_options = NativeOptions {
        viewport: ViewportBuilder::default()
            .with_title(format!("Manganese RAM Tester {} 🎉", env!("CARGO_PKG_VERSION")).to_owned())
            .with_inner_size(egui::Vec2::new(600.0, 800.0))
            .with_position(egui::Pos2::new(100.0, 100.0)),
        multisampling: 0, // reduce GPU load
        persist_window: true, // saves window position :)
        vsync: true, // sync redraw to monitor refresh
        ..Default::default()
    };
    run_native(
        format!("Manganese RAM Tester {} 🎉", env!("CARGO_PKG_VERSION")).as_str(),
        native_options,
        Box::new(|cc|{
            let app = Box::new(GuiApp::default());
            //apply_monospace_fonts(&cc.egui_ctx);
            Ok(app)
        }),
    )
}

struct GuiApp {
    ram_input: String,
    hide_serials: bool,
    running: bool,
    stop_flag: Arc<AtomicBool>,
    status: String,
    test_handle: Option<thread::JoinHandle<()>>,
    log_buffer: Arc<Mutex<String>>,
}

impl Default for GuiApp {
    fn default() -> Self {
        let buffer = Arc::new(Mutex::new(String::new()));
        init_gui_logger(buffer.clone()).unwrap();

        Self {
            ram_input: "".to_owned(),
            hide_serials: false,
            running: false,
            stop_flag: Arc::new(AtomicBool::new(false)),
            status: "Idle".to_owned(),
            test_handle: None,
            log_buffer: buffer,
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
                        .hint_text("e.g. 4GiB, 50%, 10%t")
                        .desired_width(200.0),
                );
                ui.spacing();
                ui.checkbox(&mut self.hide_serials, "Hide serial numbers");
            });

            if !self.running {
                if ui.add(egui::Button::new("Start").fill(Color32::DARK_GREEN)).clicked() {
                    // compute ram_bytes
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
                    self.status = "Running...".to_string();

                    // Clear previous log
                    {
                        let mut log = self.log_buffer.lock().unwrap();
                        log.clear();
                    }
                    let stop_clone = self.stop_flag.clone();
                    let hide_serials = self.hide_serials;

                    self.test_handle = Option::from(thread::spawn(move || {
                        // run the tests (existing code, no change required)
                        run_tests(ram_bytes, hide_serials, &stop_clone);
                    }));
                }
            } else {
                if ui.add(egui::Button::new("Stop").fill(Color32::DARK_RED)).clicked() {
                    self.stop_flag.store(true, Ordering::SeqCst);
                    self.status = "Stopping...".to_string();
                    // after stop, we expect run_tests to exit — the thread will drop guard & capture output
                    self.test_handle.take().unwrap().join().unwrap();
                    self.running = false; // allow start button again
                    self.status = "Idle".to_owned();
                }
            }

            ui.separator();
            ui.label(format!("Status: {}", self.status));

            ui.separator();
            ui.label("Console output:");
            ScrollArea::vertical()
                .auto_shrink([false; 2])
                .stick_to_bottom(true) // sticky-bottom behavior
                .show(ui, |ui| {
                    let log = self.log_buffer.lock().unwrap();
                    let text = log.as_str();
                    // Use a label to display the log
                    ui.label(text);
                });

            // Reset running status if stop flag is cleared and thread finished
            //if self.running && self.stop_flag.load(Ordering::SeqCst) == false {
                // Optimistically check: if thread has finished, mark as stopped
                // For better detection, you could join a handle (requires storing it)
                // Here, we just allow restart if stop flag was cleared
            //    self.running = false;
            //    self.status = "Idle".to_owned();
            //}
        });

        // keep repainting so we see log updates
        ctx.request_repaint();
    }
}
