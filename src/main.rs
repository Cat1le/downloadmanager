use std::{collections::HashMap, sync::Arc, thread};

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

#[derive(Clone)]
pub struct Progress {
    pub value: f32,
    pub failed: bool,
}

pub struct Download {
    pub id: usize,
    pub progress: Vec<Progress>,
    pub name: String,
    pub ready: bool,
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

impl AppState {
    fn next_id(&mut self) -> usize {
        let id = self.last_id;
        self.last_id += 1;
        id
    }
}

struct App {
    #[allow(dead_code)] // Runtime must not be dropped to keep futures running
    runtime: Arc<Runtime>,
    sender: UnboundedSender<Command>,
    state: Arc<Mutex<AppState>>,
}

impl App {
    fn new(context: &Context) -> Self {
        let runtime = Arc::new(Runtime::new().unwrap());
        let state = Arc::new(Mutex::new(AppState {
            last_id: 0,
            downloads: HashMap::new(),
            download_sel: None,
            add_download_dialog: None,
        }));
        Self {
            sender: downloader::start(runtime.clone(), state.clone(), context.clone()),
            runtime,
            state,
        }
    }

     fn enqueue(&mut self, url: String, name: String) {
        let mut state = self.state.blocking_lock();
        let id = state.next_id();
        state.downloads.insert(
            id,
            Download {
                id,
                progress: vec![
                    Progress {
                        failed: false,
                        value: 0.
                    };
                    thread::available_parallelism().unwrap().get()
                ],
                name: name.clone(),
                ready: false,
            },
        );
        self.sender
            .send(Command::Enqueue { id, url, name })
            .unwrap();
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        let state_arc = self.state.clone();
        let mut state = state_arc.blocking_lock();
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
                    state.add_download_dialog = Some(AddDownloadDialog {
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
            }
        });
    }
}
