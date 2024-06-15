use std::{
    collections::HashMap,
    fs::{self, File},
    io::{BufRead, BufReader, Read, Write},
    net::{TcpStream, ToSocketAddrs},
    path::PathBuf,
    sync::mpsc::{self, Receiver, Sender},
    thread,
};

use dns_lookup::lookup_host;
use url::Url;

pub enum RecvMessage {
    AddNew { name: String },
    AddNewFail { name: String, reason: String },
    ProgressUpdated { name: String, new_progress: f32 },
}

pub enum SendMessage {
    QueueNew { url: String, location: PathBuf },
    Delete { name: String },
}

enum ToWorkerMessage {
    Delete,
}

pub fn start(sender: Sender<RecvMessage>, receiver: Receiver<SendMessage>) {
    let mut workers = HashMap::new();

    thread::spawn(move || loop {
        match receiver.recv().unwrap() {
            SendMessage::QueueNew { url, location } => {
                let name = url[url.rfind('/').unwrap() + 1..].to_string();
                workers.insert(name.clone(), worker(url, name, sender.clone(), location));
            }
            SendMessage::Delete { name } => {
                workers
                    .remove(&name)
                    .unwrap()
                    .send(ToWorkerMessage::Delete)
                    .unwrap();
            }
        }
    });
}

fn worker(
    url: String,
    name: String,
    sender: Sender<RecvMessage>,
    location: PathBuf,
) -> Sender<ToWorkerMessage> {
    let (s, r) = mpsc::channel();
    thread::spawn(move || {
        sender
            .send(RecvMessage::AddNew { name: name.clone() })
            .unwrap();
        let url = Url::parse(&url).unwrap();
        let Ok(mut stream) = TcpStream::connect(get_ip(url.host_str().unwrap())) else {
            sender
                .send(RecvMessage::AddNewFail {
                    name,
                    reason: "Cannot open tcpstream".into(),
                })
                .unwrap();
            return;
        };
        let Ok(_) = stream.write(b"GET /path HTTP/1.0\r\n\r\n") else {
            sender
                .send(RecvMessage::AddNewFail {
                    name,
                    reason: "Cannot write to tcpstream".into(),
                })
                .unwrap();
            return;
        };
        let headers = get_headers(&mut stream);
        let content_length = headers
            .get("content-length")
            .map(|x| x.parse::<usize>().unwrap());
        if content_length.is_none() {
            sender
                .send(RecvMessage::ProgressUpdated {
                    name: name.clone(),
                    new_progress: 0.99,
                })
                .unwrap();
        }
        let file_location = get_unused_filename(location.clone(), name.clone());
        let Ok(mut file) = File::create(location.join(&file_location)) else {
            sender
                .send(RecvMessage::AddNewFail {
                    name,
                    reason: "Cannot write to tcpstream".into(),
                })
                .unwrap();
            return;
        };
        let mut reader = BufReader::new(stream);
        let mut written = 0;
        loop {
            if matches!(r.try_recv(), Ok(ToWorkerMessage::Delete)) {
                drop(file);
                fs::remove_file(location.join(&file_location)).unwrap();
                return;
            }
            let Ok(buf) = reader.fill_buf() else {
                sender
                    .send(RecvMessage::AddNewFail {
                        name,
                        reason: "Cannot read from tcpstream".into(),
                    })
                    .unwrap();
                drop(file);
                fs::remove_file(location.join(&file_location)).unwrap();
                return;
            };
            file.write(buf).unwrap();
            let len = buf.len();
            if len == 0 {
                sender
                    .send(RecvMessage::ProgressUpdated {
                        name: name.clone(),
                        new_progress: 1.,
                    })
                    .unwrap();
                return;
            }
            written += len;
            if let Some(total) = content_length {
                sender
                    .send(RecvMessage::ProgressUpdated {
                        name: name.clone(),
                        new_progress: (total as f32) / (written as f32),
                    })
                    .unwrap();
            }
            reader.consume(len);
        }
    });
    s
}

fn get_ip(host: &str) -> impl ToSocketAddrs {
    (*lookup_host(host).unwrap().first().unwrap(), 80u16)
}

fn get_headers<R: Read>(stream: R) -> HashMap<String, String> {
    let mut result = String::new();
    for i in stream.bytes() {
        let i = i.unwrap();
        result.push(i as char);
        if result.len() > 3 && &result[result.len() - 4..] == "\r\n\r\n" {
            break;
        }
    }
    result
        .split('\n')
        .filter_map(|x| {
            x.trim()
                .split_once(": ")
                .map(|(a, b)| (a.to_lowercase(), b.to_string()))
        })
        .collect()
}

fn get_unused_filename(location: PathBuf, base: String) -> String {
    let path = location.join(&base);
    if !path.exists() {
        return base;
    }
    let dot = base.find('.').unwrap();
    for i in 0.. {
        let prev = &base[..dot];
        let next = &base[dot + 1..];
        let base = format!("{prev} ({i}){next}");
        let path = location.join(&base);
        if !path.exists() {
            return base;
        }
    }
    unreachable!()
}
