use eframe::egui;
use crossbeam::channel::{unbounded, Receiver};
use backend::LogMessage;

const PROG_VERSION: &str = env!("CARGO_PKG_VERSION");

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
            .with_inner_size([900.0, 700.0])
            .with_title("Bot Paradex"),
        ..Default::default()
    };

    eframe::run_native(
        "Bot Paradex",
        options,
        Box::new(|_cc| Ok(Box::new(MyApp::new(log_receiver)))),
    )
}

struct MyApp {
    log_receiver: Receiver<LogMessage>,
    logs: Vec<LogMessage>,
    auto_scroll: bool,
    show_readme: bool,
    readme_content: String,
}

impl MyApp {
    fn new(log_receiver: Receiver<LogMessage>) -> Self {
        // Load README content
        let readme_content = std::fs::read_to_string("README.md")
            .unwrap_or_else(|_| "README.md not found".to_string());

        Self {
            log_receiver,
            logs: Vec::new(),
            auto_scroll: true,
            show_readme: false,
            readme_content,
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
            let available_height = ui.available_height();
            
            // Header section (3/4 of space at top)
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), available_height * 0.75),
                egui::Layout::top_down(egui::Align::Center),
                |ui| {
                    ui.add_space(40.0);
                    
                    // Title
                    ui.heading(egui::RichText::new("🤖 Bot Paradex")
                        .size(32.0)
                        .strong());
                    
                    ui.add_space(10.0);
                    
                    ui.label(egui::RichText::new("Farm Volume Options")
                        .size(18.0)
                        .color(egui::Color32::GRAY));
                    
                    ui.add_space(20.0);
                    
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(format!("v{}", PROG_VERSION))
                            .size(14.0)
                            .color(egui::Color32::DARK_GRAY));
                        ui.label("|");
                        ui.label(egui::RichText::new("MIT License")
                            .size(14.0)
                            .color(egui::Color32::DARK_GRAY));
                    });
                    
                    ui.add_space(15.0);
                    
                    if ui.button(egui::RichText::new("📖 Show README").size(16.0)).clicked() {
                        self.show_readme = !self.show_readme;
                    }
                },
            );
            
            ui.separator();

            // Logs section (1/4 of space at bottom)
            ui.heading("📝 Logs");
            
            ui.horizontal(|ui| {
                ui.label(format!("Total: {} logs", self.logs.len()));
                ui.separator();
                ui.checkbox(&mut self.auto_scroll, "Auto-scroll");
                if ui.button("🗑 Clean").clicked() {
                    self.logs.clear();
                }
            });
            
            ui.add_space(5.0);

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

        // README window
        if self.show_readme {
            egui::Window::new("📖 README")
                .collapsible(false)
                .resizable(true)
                .default_width(700.0)
                .default_height(500.0)
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        // Display README content as monospace text
                        ui.add(
                            egui::TextEdit::multiline(&mut self.readme_content.as_str())
                                .font(egui::TextStyle::Monospace)
                                .desired_width(f32::INFINITY)
                                .interactive(false)
                        );
                    });
                    
                    ui.add_space(10.0);
                    if ui.button("Close").clicked() {
                        self.show_readme = false;
                    }
                });
        }
    }
}
