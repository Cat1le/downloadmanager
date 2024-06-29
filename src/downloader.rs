use std::sync::Arc;

use tokio::{
    runtime::Runtime,
    sync::{
        mpsc::{unbounded_channel, UnboundedSender},
        Mutex,
    },
};

use crate::AppState;

pub enum Command {
    Enqueue { url: String, name: String },
    Restart { id: usize },
    Delete { id: usize },
}

pub fn start(runtime: Arc<Runtime>, state: Arc<Mutex<AppState>>) -> UnboundedSender<Command> {
    let (sender, receiver) = unbounded_channel();
    sender
}
