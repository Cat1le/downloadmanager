use std::{fs::{self, File}, hash::Hash, io::Write, path::PathBuf, sync::Arc};

use eframe::{
    egui::{
        ahash::{HashMap, HashMapExt},
        CentralPanel, Context, ProgressBar, TextEdit, ViewportBuilder,
    },
    NativeOptions,
};
use tokio::sync::Mutex;

fn main() {
    eframe::run_native(
        "Download manager",
        NativeOptions {
            viewport: ViewportBuilder::default(),
            ..Default::default()
        },
        Box::new(|ctx| Box::new(App::new(&ctx.egui_ctx))),
    )
    .unwrap()
}

struct Download {
    url: String,
    name: String,
    progress: f32,
}

impl Download {
    fn new(url: String) -> Self {
        Self {
            progress: 0.,
            name: url[url.rfind('/').unwrap() + 1..].to_string(),
            url,
        }
    }
}

impl Hash for Download {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.url.hash(state);
        self.name.hash(state);
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
            ui.separator();
            let mut downloads = self.downloads.blocking_lock();
            let mut to_delete = None;
            for (id, download) in downloads.iter() {
                ui.horizontal(|ui| {
                    ui.add(
                        ProgressBar::new(download.progress)
                            .desired_height(10.)
                            .desired_width(100.),
                    );
                    ui.label(&download.name);
                    if download.progress == 1. {
                        if ui.button("Open").clicked() {
                            drop(open::that(
                                self.location.as_ref().unwrap().join(&download.name),
                            ));
                        }
                        if ui.button("Delete").clicked() {
                            to_delete = Some(id.clone());
                        }
                    }
                });
            }
            if let Some(id) = to_delete {
                let path = self
                    .location
                    .as_ref()
                    .unwrap()
                    .join(&downloads.iter().find(|x| x.0 == &id).unwrap().1.name);
                fs::remove_file(path).unwrap();
                downloads.remove(&id);
            }
            ui.separator();
            ui.horizontal(|ui| {
                ui.add(TextEdit::singleline(&mut self.new_download_url).hint_text(""));
                if ui.button("Add").clicked() {
                    let location = self.location.clone().unwrap();
                    let download = Download::new(self.new_download_url.clone());
                    let download_id = downloads.keys().max().cloned().unwrap_or(0);
                    let download_url = self.new_download_url.clone();
                    let download_name = download.name.clone();
                    downloads.insert(download_id, download);
                    drop(downloads);
                    let downloads_ref = Arc::clone(&self.downloads);
                    let context_ref = Arc::clone(&self.context);
                    self.runtime.spawn(async move {
                        let mut file = File::create(location.join(download_name)).unwrap();
                        let mut resp = reqwest::get(download_url).await.unwrap();
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
