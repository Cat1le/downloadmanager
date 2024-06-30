use std::{
    env::current_dir,
    fs::{self, File},
    io::Write,
    path::PathBuf,
    sync::Arc,
};

use reqwest::{header::HeaderValue, Client, Method, Request, Url};
use tokio::{
    runtime::Runtime,
    sync::mpsc::{unbounded_channel, UnboundedSender},
};

macro_rules! or_fail {
    ($expr:expr, $sender:expr, $id:expr) => {
        match $expr {
            Ok(x) => x,
            Err(e) => {
                $sender.send(Response::Failed {
                    id: $id,
                    reason: format!("{e}"),
                });
                return;
            }
        }
    };
    ($expr:expr, $sender:expr, $id:expr, $cleanup:tt) => {
        match $expr {
            Ok(x) => x,
            Err(e) => {
                $sender.send(Response::Failed {
                    id: $id,
                    reason: format!("{e}"),
                });
                $cleanup;
                return;
            }
        }
    };
}

#[derive(Clone)]
pub struct WorkerId {
    entry_id: usize,
    worker_index: usize,
}

pub struct WorkerData {
    id: WorkerId,
    url: Url,
    range: (usize, usize),
    runtime: Arc<Runtime>,
    client: Client,
}

pub enum Command {
    Restart,
    Delete,
}

pub enum Response {
    MadeProgress { id: WorkerId, value: f32 },
    Succeed { id: WorkerId },
    Failed { id: WorkerId, reason: String },
}

pub fn start(
    sender: UnboundedSender<Response>,
    WorkerData {
        id,
        url,
        range,
        runtime,
        client,
    }: WorkerData,
) -> UnboundedSender<Command> {
    let (cmd_sender, cmd_receiver) = unbounded_channel();
    let mut req = Request::new(Method::GET, url);
    req.headers_mut().insert(
        "Range",
        HeaderValue::from_str(&format!("{}-{}", range.0, range.1)).unwrap(),
    );
    runtime.spawn(async move {
        let filepath = file_name(&id);
        let file = or_fail!(File::create_new(&filepath), sender, id);
        let resp = or_fail!(client.execute(req).await, sender, id, {
            fs::remove_file(filepath)
        });
        let total_length = range.1 - range.0;
        let current_length = 0;
        while let Some(chunk) = or_fail!(resp.chunk().await, sender, id, {
            fs::remove_file(filepath)
        }) {
            or_fail!(file.write(&chunk), sender, id, {
                fs::remove_file(filepath)
            });
            sender.send(Response::MadeProgress {
                id: id.clone(),
                value: current_length as f32 / total_length as f32,
            });
        }
        or_fail!(file.flush(), sender, id, { fs::remove_file(filepath) });
        sender.send(Response::Succeed { id: id.clone() });
    });
    cmd_sender
}

fn file_name(
    WorkerId {
        entry_id,
        worker_index,
    }: &WorkerId,
) -> PathBuf {
    let dir = current_dir().unwrap();
    for i in 0usize.. {
        let path = dir.join(format!("worker-{entry_id}-{worker_index}-{i}.bin"));
        if !path.exists() {
            return path;
        }
    }
    unreachable!()
}
