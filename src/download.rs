use std::{
    sync::mpsc::{Receiver, Sender},
    thread,
};

pub enum RecvMessage {
    AddNew { name: String },
    AddNewFail { url: String, reason: String },
    ProgressUpdated { name: String, new_progress: f32 },
}

pub enum SendMessage {
    QueueNew { url: String },
    Delete { name: String },
}

pub fn start(sender: Sender<RecvMessage>, receiver: Receiver<SendMessage>) {
    thread::spawn(move || loop {
        match receiver.recv().unwrap() {
            SendMessage::QueueNew { url } => {
                sender
                .send(RecvMessage::AddNewFail {
                    url,
                    reason: "Nope.".into(),
                })
                .unwrap()},
            SendMessage::Delete { name } => todo!(),
        }
    });
}
