use app::App;
use eframe::{egui::ViewportBuilder, NativeOptions};

mod app;
mod download;

fn main() {
    eframe::run_native(
        "Download Manager",
        NativeOptions {
            viewport: ViewportBuilder::default().with_inner_size((640., 480.)),
            ..Default::default()
        },
        Box::new(|_ctx| Box::<App>::default()),
    )
    .unwrap();
}
