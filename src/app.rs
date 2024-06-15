use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver, Sender},
};

use eframe::egui::{CentralPanel, ProgressBar, TextEdit, Window};
use notify_rust::Notification;
use url::Url;

use crate::download::{self, RecvMessage, SendMessage};

struct DownloadInfo {
    progress: f32,
    name: String,
}

pub struct App {
    location: PathBuf,
    downloads: Vec<DownloadInfo>,
    send: Sender<SendMessage>,
    recv: Receiver<RecvMessage>,
    add_download_show: bool,
    add_download_text: String,
}

impl Default for App {
    fn default() -> Self {
        let (s1, r1) = mpsc::channel();
        let (s2, r2) = mpsc::channel();
        download::start(s1, r2);
        Self {
            location: format!("C:/Users/{}/Downloads", whoami::username()).into(),
            downloads: vec![],
            send: s2,
            recv: r1,
            add_download_show: false,
            add_download_text: String::new(),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(self.location.to_string_lossy());
                // TODO: move files when changing directory
                // if ui.button("Change").clicked() {
                //     if let Some(folder) = FileDialog::new().pick_folder() {
                //         self.location = folder;
                //     }
                // }
            });
            ui.separator();
            let mut to_delete = None;
            for download in &self.downloads {
                ui.horizontal(|ui| {
                    ui.add(
                        ProgressBar::new(download.progress)
                            .desired_height(10.)
                            .desired_width(100.),
                    );
                    ui.label(&download.name);
                    if download.progress == 1. {
                        let path = self.location.join(&download.name);
                        if ui.button("Open").clicked() {
                            open::that(&path).unwrap();
                        }
                    }
                    if ui.button("Delete").clicked() {
                        self.send
                            .send(SendMessage::Delete {
                                name: download.name.clone(),
                            })
                            .unwrap();
                        let mut index = 0;
                        for (idx, item) in self.downloads.iter().enumerate() {
                            if item.name == download.name {
                                index = idx;
                                break;
                            }
                        }
                        to_delete = Some(index);
                    }
                });
            }
            if let Some(index) = to_delete {
                self.downloads.remove(index);
            }
            if !self.downloads.is_empty() {
                ui.separator();
            }
            if ui.button("Add new download").clicked() {
                self.add_download_show = true;
            }
            if self.add_download_show {
                Window::new("Add download")
                    .show(ctx, |ui| {
                        ui.add(
                            TextEdit::singleline(&mut self.add_download_text)
                                .hint_text("http://example.com/file.txt"),
                        );
                        ui.add_space(10.);
                        ui.horizontal(|ui| {
                            if Url::parse(&self.add_download_text).is_ok() {
                                if ui.button("Apply").clicked() {
                                    self.send
                                        .send(SendMessage::QueueNew {
                                            url: self.add_download_text.clone(),
                                            location: self.location.clone(),
                                        })
                                        .unwrap();
                                    self.add_download_show = false;
                                    self.add_download_text.clear();
                                }
                            }
                            if ui.button("Close").clicked() {
                                self.add_download_show = false;
                                self.add_download_text.clear();
                            }
                        })
                    })
                    .unwrap();
            }
            if let Ok(msg) = self.recv.try_recv() {
                match msg {
                    RecvMessage::AddNew { name } => {
                        self.downloads.push(DownloadInfo { progress: 0., name })
                    }
                    RecvMessage::AddNewFail { name, reason } => {
                        Notification::new()
                            .summary("Download Manager")
                            .body(&format!("{name} failed because {reason}"))
                            .show()
                            .unwrap();
                    }
                    RecvMessage::ProgressUpdated { name, new_progress } => {
                        self.downloads
                            .iter_mut()
                            .find(|x| x.name == name)
                            .unwrap()
                            .progress = new_progress;
                    }
                }
            }
        });
    }
}
