use std::{collections::HashMap, sync::Arc};

use downloader::Command;
use eframe::{
    egui::{
        vec2, Button, CentralPanel, Color32, Context, ProgressBar, SidePanel, TextEdit, Window,
    },
    NativeOptions,
};
use tokio::{
    runtime::Runtime,
    sync::{mpsc::UnboundedSender, Mutex},
};
use widget::ManyProgressBar;

mod downloader;
mod widget;

fn main() {
    eframe::run_native(
        "Download Manager",
        NativeOptions::default(),
        Box::new(|ctx| Box::new(App::new(&ctx.egui_ctx))),
    )
    .unwrap();
}

pub struct Progress {
    pub value: f32,
    pub failed: bool,
}

pub struct Download {
    pub id: usize,
    pub progress: Vec<Progress>,
    pub name: String,
}

struct AddDownloadDialog {
    url: String,
    filename: String,
}

struct AppState {
    last_id: usize,
    downloads: HashMap<usize, Download>,
    download_sel: Option<usize>,
    add_download_dialog: Option<AddDownloadDialog>,
}

struct App {
    #[allow(dead_code)] // Runtime must not be dropped to keep futures running
    runtime: Arc<Runtime>,
    sender: UnboundedSender<Command>,
    state: Arc<Mutex<AppState>>,
}

impl App {
    fn new(_context: &Context) -> Self {
        let runtime = Arc::new(Runtime::new().unwrap());
        let state = Arc::new(Mutex::new(AppState {
            last_id: 0,
            downloads: HashMap::new(),
            download_sel: None,
            add_download_dialog: None,
        }));
        Self {
            sender: downloader::start(runtime.clone(), state.clone()),
            runtime,
            state,
        }
    }

    fn enqueue(&mut self, url: String, name: String) {
        self.sender.send(Command::Enqueue { url, name }).unwrap();
    }

    fn delete(&mut self, id: usize) {
        self.sender.send(Command::Delete { id }).unwrap();
    }

    fn restart(&mut self, id: usize) {
        self.sender.send(Command::Restart { id }).unwrap();
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        let state_arc = self.state.clone();
        let mut state = state_arc.blocking_lock();
        let mut to_delete = None;
        let mut to_restart = None;
        SidePanel::left("left panel")
            .exact_width(200.)
            .show_separator_line(true)
            .show(ctx, |ui| {
                if ui
                    .add(
                        Button::new("                 Add download")
                            .min_size(vec2(ui.available_size_before_wrap().x, 30.)),
                    )
                    .clicked()
                {
                    self.state.blocking_lock().add_download_dialog = Some(AddDownloadDialog {
                        url: "".into(),
                        filename: "".into(),
                    });
                }
                let mut close_add_download = false;
                let mut enqueue = false;
                if let Some(dialog) = &mut state.add_download_dialog {
                    Window::new("Add download")
                        .collapsible(false)
                        .show(ctx, |ui| {
                            ui.add(TextEdit::singleline(&mut dialog.url).hint_text("Url"));
                            ui.add(
                                TextEdit::singleline(&mut dialog.filename).hint_text("File name"),
                            );
                            ui.add_space(20.);
                            ui.horizontal(|ui| {
                                if ui.button("Add").clicked() {
                                    enqueue = true;
                                }
                                if ui.button("Close").clicked() {
                                    close_add_download = true;
                                }
                            })
                        });
                }
                if enqueue {
                    let AddDownloadDialog { url, filename } =
                        state.add_download_dialog.take().unwrap();
                    self.enqueue(url, filename);
                }
                if close_add_download {
                    state.add_download_dialog = None;
                }

                ui.separator();
                let mut set_download_sel = None;
                for (&download_id, download) in state.downloads.iter() {
                    let mut ranges = Vec::with_capacity(download.progress.len());
                    for (progress_idx, progress) in download.progress.iter().enumerate() {
                        ranges.push((
                            progress_idx as f32 / download.progress.len() as f32,
                            progress_idx as f32 / download.progress.len() as f32
                                + progress.value / download.progress.len() as f32,
                            progress.failed,
                        ));
                    }
                    ui.horizontal(|ui| {
                        if ui.button(&download.name).clicked() {
                            set_download_sel = Some(download_id);
                        }
                        if download.progress.iter().any(|x| x.failed) {
                            if ui.button("R").clicked() {
                                to_restart = Some(download_id);
                            }
                        }
                        if ui.button("D").clicked() {
                            to_delete = Some(download_id);
                        }
                    });
                    ui.add(ManyProgressBar::new(vec2(0., 7.), &ranges));
                    ui.add_space(10.);
                }
                if set_download_sel.is_some() {
                    state.download_sel = set_download_sel;
                }
            });
        CentralPanel::default().show(ctx, |ui| {
            if let Some(selected_download) = state.download_sel {
                let download = &state.downloads[&selected_download];
                ui.label(format!("Total workers: {}", download.progress.len()));
                for progress in &download.progress {
                    ui.separator();
                    let mut bar = ProgressBar::new(progress.value);
                    if progress.failed {
                        bar = bar.fill(Color32::RED);
                    }
                    ui.add(bar);
                }
                ui.separator();
                ui.horizontal(|ui| {
                    if download.progress.iter().any(|x| x.failed) {
                        if ui.button("Restart failed workers").clicked() {
                            to_restart = state.download_sel;
                        }
                    }
                    if ui.button("Delete").clicked() {
                        to_delete = state.download_sel;
                    }
                });
            }
        });
        if let Some(to_remove) = to_delete {
            self.delete(to_remove)
        }
        if let Some(to_restart) = to_restart {
            self.restart(to_restart);
        }
    }
}
