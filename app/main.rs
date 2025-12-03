use eframe::egui;

fn main() -> Result<(), eframe::Error> {
    // launch back end in seperated thread
    std::thread::spawn(|| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            tokio::task::spawn_blocking(|| {
                let rt2 = tokio::runtime::Handle::current();
                rt2.block_on(async {
                    backend::run_backend_logic().await;
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
        Box::new(|_cc| Ok(Box::new(MyApp::default()))),
    )
}

struct MyApp;

impl Default for MyApp {
    fn default() -> Self {
        Self
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Application");
            ui.label("Backend executing in background...");
        });
    }
}
