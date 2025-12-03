use eframe::egui;
use crossbeam::channel::{unbounded, Receiver};
use backend::LogMessage;

fn main() -> Result<(), eframe::Error> {
    // receive back-end logs
    let (log_sender, log_receiver) = unbounded::<LogMessage>();
    
    // launch back end in seperated thread
    std::thread::spawn(|| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            tokio::task::spawn_blocking(|| {
                let rt2 = tokio::runtime::Handle::current();
                rt2.block_on(async {
                    backend::run_backend_logic(log_sender).await;
                });
            }).await.unwrap();
        });
    });

    // launch frontend
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_title("My Application"),
        ..Default::default()
    };

    eframe::run_native(
        "My Application",
        options,
        Box::new(|_cc| Ok(Box::new(MyApp::new(log_receiver)))),
    )
}

struct MyApp {
    log_receiver: Receiver<LogMessage>,
    logs: Vec<LogMessage>,
    auto_scroll: bool,
}

impl MyApp {
    fn new(log_receiver: Receiver<LogMessage>) -> Self {
        Self {
            log_receiver,
            logs: Vec::new(),
            auto_scroll: true,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Get new logs
        while let Ok(log) = self.log_receiver.try_recv() {
            self.logs.push(log);
        }

        // Refresh
        ctx.request_repaint_after(std::time::Duration::from_millis(100));

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("📝 Logs Backend");
            
            ui.horizontal(|ui| {
                ui.label(format!("Total: {} logs", self.logs.len()));
                ui.separator();
                ui.checkbox(&mut self.auto_scroll, "Auto-scroll");
                if ui.button("🗑 Clean").clicked() {
                    self.logs.clear();
                }
            });
            
            ui.separator();

            // Log zone with scrolling
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .stick_to_bottom(self.auto_scroll)
                .show(ui, |ui| {
                    for log in &self.logs {
                        ui.horizontal(|ui| {
                            // Different color for levels
                            let (color, emoji) = match log.level.as_str() {
                                "ERROR" => (egui::Color32::RED, "❌"),
                                "WARN" => (egui::Color32::YELLOW, "⚠️"),
                                "INFO" => (egui::Color32::GREEN, "ℹ️"),
                                "DEBUG" => (egui::Color32::GRAY, "🔍"),
                                _ => (egui::Color32::WHITE, "📝"),
                            };

                            ui.colored_label(egui::Color32::DARK_GRAY, &log.timestamp);
                            ui.label(emoji);
                            ui.colored_label(color, format!("[{}]", log.level));
                            ui.label(&log.message);
                        });
                    }
                });
        });
    }
}
