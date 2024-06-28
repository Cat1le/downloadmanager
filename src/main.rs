#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::{
    fs::{self, File},
    io::Write,
    path::PathBuf,
    sync::Arc,
};

use eframe::{
    egui::{
        ahash::{HashMap, HashMapExt},
        CentralPanel, Color32, Context, ProgressBar, TextEdit, ViewportBuilder,
    },
    NativeOptions,
};
use tokio::sync::Mutex;
use url::Url;

fn main() {
    eframe::run_native(
        "Download Manager",
        NativeOptions {
            viewport: ViewportBuilder::default().with_inner_size((640., 480.)),
            ..Default::default()
        },
        Box::new(|ctx| Box::new(App::new(&ctx.egui_ctx))),
    )
    .unwrap()
}

#[derive(Clone)]
struct Download {
    name: String,
    progress: f32,
}

impl Download {
    fn new(url: &Url) -> Self {
        Self {
            progress: 0.,
            name: url.path()[url.path().rfind('/').unwrap() + 1..].to_string(),
        }
    }
}

struct App {
    runtime: tokio::runtime::Runtime,
    context: Arc<Context>,
    location: Option<PathBuf>,
    downloads: Arc<Mutex<HashMap<usize, Download>>>,
    new_download_url: String,
}

impl App {
    fn new(context: &Context) -> Self {
        Self {
            runtime: tokio::runtime::Runtime::new().unwrap(),
            context: Arc::new(context.clone()),
            location: Self::get_location(),
            downloads: Arc::new(Mutex::new(HashMap::new())),
            new_download_url: String::new(),
        }
    }

    #[cfg(target_os = "windows")]
    fn get_location() -> Option<PathBuf> {
        let pb: PathBuf = format!("C:\\Users\\{}\\Downloads", whoami::username()).into();
        Some(pb).filter(|x| x.exists())
    }

    #[cfg(not(target_os = "windows"))]
    fn get_location() -> Option<PathBuf> {
        None
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Saving to");
                if let Some(loc) = &self.location {
                    if ui.button(loc.to_string_lossy()).clicked() {
                        if let Some(new_loc) = rfd::FileDialog::new().pick_folder() {
                            self.location = Some(new_loc);
                        }
                    }
                } else {
                    if ui.button("Specify location").clicked() {
                        if let Some(new_loc) = rfd::FileDialog::new().pick_folder() {
                            self.location = Some(new_loc);
                        }
                    }
                }
            });
            if self.location.is_none() {
                return;
            }
            let location = self.location.clone().unwrap();
            ui.separator();
            let mut downloads = self.downloads.blocking_lock();
            let mut to_delete = None;
            for (id, download) in downloads.iter() {
                ui.horizontal(|ui| {
                    ui.add(
                        ProgressBar::new(download.progress)
                            .desired_height(10.)
                            .desired_width(200.),
                    );
                    ui.label(&download.name);
                    if download.progress == 1. {
                        if ui.button("Open").clicked() {
                            let _ = open::that(location.join(&download.name));
                        }
                        if ui.button("Delete").clicked() {
                            to_delete = Some(id.clone());
                        }
                    }
                });
            }
            if let Some(id) = to_delete {
                let path = location.join(&downloads.iter().find(|x| x.0 == &id).unwrap().1.name);
                fs::remove_file(path).unwrap();
                downloads.remove(&id);
            }
            ui.separator();
            ui.horizontal(|ui| {
                let url = Url::parse(&self.new_download_url);
                let mut text_edit = TextEdit::singleline(&mut self.new_download_url)
                    .hint_text("http://example.com/file.txt");
                if url.is_err() {
                    text_edit = text_edit.text_color(Color32::RED);
                }
                ui.add(text_edit);
                if url.is_ok() && ui.button("Add").clicked() {
                    self.new_download_url.clear();
                    let url = url.unwrap();
                    let download = Download::new(&url);
                    let download_id = downloads.keys().max().cloned().map(|x| x + 1).unwrap_or(0);
                    let download_name = download.name.clone();
                    downloads.insert(download_id, download);
                    let downloads_ref = Arc::clone(&self.downloads);
                    let context_ref = Arc::clone(&self.context);
                    self.runtime.spawn(async move {
                        let mut file = File::create(location.join(download_name)).unwrap();
                        let mut resp = reqwest::get(url.as_str()).await.unwrap();
                        let total = resp.content_length().unwrap();
                        let mut current = 0;
                        while let Ok(Some(chunk)) = resp.chunk().await {
                            current += chunk.len();
                            downloads_ref
                                .lock()
                                .await
                                .get_mut(&download_id)
                                .unwrap()
                                .progress = current as f32 / total as f32;
                            context_ref.request_repaint();
                            file.write(&chunk).unwrap();
                        }
                    });
                }
            });
        });
    }
}
