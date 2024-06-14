use std::sync::mpsc::{Receiver, Sender};

pub enum RecvMessage {
    AddNew { name: String },
    ProgressUpdated { name: String, new_progress: f32 },
}

pub enum SendMessage {
    QueueNew { url: String },
    Delete{name: String}
}

pub fn start(_sender: Sender<RecvMessage>, _receiver: Receiver<SendMessage>) {
    
}
