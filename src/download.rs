use std::{
    collections::HashMap, io::Write, net::{TcpListener, TcpStream}, sync::mpsc::{self, Receiver, Sender}, thread, time::Duration
};

pub enum RecvMessage {
    AddNew { name: String },
    AddNewFail { name: String, reason: String },
    ProgressUpdated { name: String, new_progress: f32 },
}

pub enum SendMessage {
    QueueNew { url: String },
    Delete { name: String },
}

enum ToWorkerMessage {
    Delete,
}

pub fn start(sender: Sender<RecvMessage>, receiver: Receiver<SendMessage>) {
    let mut workers = HashMap::new();

    thread::spawn(move || loop {
        match receiver.recv().unwrap() {
            SendMessage::QueueNew { url } => {
                let name = url[url.rfind('/').unwrap() + 1..].to_string();
                workers.insert(name.clone(), worker(url, name, sender.clone()));
            }
            SendMessage::Delete { name } => {
                workers[&name].send(ToWorkerMessage::Delete).unwrap();
                workers.remove(&name);
            }
        }
    });
}

fn worker(url: String, name: String, sender: Sender<RecvMessage>) -> Sender<ToWorkerMessage> {
    let (s, r) = mpsc::channel();
    thread::spawn(move || {
        let mut stream = TcpStream::connect(url).unwrap();
        // TOOD
    });
    s
}