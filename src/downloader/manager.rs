use std::sync::Arc;

use eframe::egui::Context;
use notify_rust::Notification;
use reqwest::Client;
use tokio::{
    runtime::Runtime,
    sync::{
        mpsc::{unbounded_channel, UnboundedSender},
        Mutex,
    },
};

use crate::AppState;

use super::entry::{self, EntryData};

pub enum Command {
    Enqueue {
        id: usize,
        url: String,
        name: String,
    },
}

pub fn start(
    runtime: Arc<Runtime>,
    state: Arc<Mutex<AppState>>,
    context: Context,
) -> UnboundedSender<Command> {
    let (sender, mut receiver) = unbounded_channel();
    let (sx, mut rx) = unbounded_channel();
    let client = Client::new();
    let runtime_ref = runtime.clone();
    runtime.spawn(async move {
        while let Some(resp) = receiver.recv().await {
            match resp {
                Command::Enqueue { id, url, name } => entry::start(
                    sx.clone(),
                    EntryData {
                        id,
                        url,
                        name,
                        runtime: runtime_ref.clone(),
                        client: client.clone(),
                    },
                ),
            }
        }
    });
    runtime.spawn(async move {
        while let Some(resp) = rx.recv().await {
            let state = &mut state.lock().await.downloads;
            match resp {
                entry::Response::MadeProgress { id, value } => {
                    state.get_mut(&id.entry_id).unwrap().progress[id.worker_index].value = value;
                }
                entry::Response::Succeed { id } => {
                    let x = &mut state.get_mut(&id.entry_id).unwrap().progress[id.worker_index];
                    x.value = 1.;
                }
                entry::Response::SucceedGlobal { id } => {
                    state.get_mut(&id.entry_id).unwrap().ready = true;
                }
                entry::Response::Failed { id, reason } => {
                    let x = &mut state.get_mut(&id.entry_id).unwrap().progress[id.worker_index];
                    x.failed = true;
                    Notification::new()
                        .appname("Download Manager")
                        .body(&reason)
                        .show()
                        .unwrap();
                }
                entry::Response::FailedGlobal { id, reason } => {
                    Notification::new()
                        .appname("Download Manager")
                        .body(&reason)
                        .show()
                        .unwrap();
                    state.remove(&id.entry_id);
                }
            }
            context.request_repaint();
        }
    });
    sender
}
