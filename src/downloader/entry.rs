use core::time;
use std::{sync::Arc, thread};

use reqwest::{Client, Url};
use tokio::{
    runtime::Runtime,
    sync::mpsc::{unbounded_channel, UnboundedSender},
};

use super::worker::{self, WorkerData, WorkerId};

macro_rules! or_fail_global {
    ($expr:expr, $sender:expr, $id:expr) => {
        match $expr {
            Ok(x) => x,
            Err(e) => {
                $sender.send(Response::FailedGlobal {
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
                $sender.send(Response::FailedGlobal {
                    id: $id,
                    reason: format!("{e}"),
                });
                $cleanup;
                return;
            }
        }
    };
}

pub struct EntryData {
    id: usize,
    url: Url,
    runtime: Arc<Runtime>,
    client: Client,
}

pub struct EntryId {
    entry_id: usize,
}

pub enum Command {
    Restart { worker_index: usize },
    Delete,
}

pub enum Response {
    MadeProgress { id: WorkerId, value: f32 },
    Succeed { id: WorkerId },
    SucceedGlobal { id: EntryId },
    Failed { id: WorkerId, reason: String },
    FailedGlobal { id: EntryId, reason: String },
}

pub fn start(
    sender: UnboundedSender<Response>,
    EntryData {
        id,
        url,
        runtime,
        client,
    }: EntryData,
) -> UnboundedSender<Command> {
    let (cmd_sender, cmd_receiver) = unbounded_channel();
    let (sx, rx) = unbounded_channel();
    let runtime_ref = runtime.clone();
    let par = thread::available_parallelism().unwrap().get();
    runtime.spawn(async move {
        let id = EntryId { entry_id: id };
        let resp = or_fail_global!(client.head(url.clone()).send().await, sender, id);
        let content_length = or_fail_global!(
            resp.content_length().ok_or("No Content-Length header"),
            sender,
            id
        ) as usize;
        let mut workers = Vec::new();
        for (idx, range) in calculate_ranges(content_length, par)
            .into_iter()
            .enumerate()
        {
            workers.push(worker::start(
                sx.clone(),
                WorkerData {
                    id: WorkerId {
                        entry_id: id.entry_id,
                        worker_index: idx,
                    },
                    url: url.clone(),
                    range,
                    runtime: runtime_ref,
                    client: client.clone(),
                },
            ));
        }
    });
    runtime.spawn(async move {
        let mut done_count = 0;
        while let Some(resp) = rx.recv().await {
            match resp {
                worker::Response::MadeProgress { id, value } => {
                    sender.send(Response::MadeProgress { id, value });
                }
                worker::Response::Succeed { id } => {
                    sender.send(Response::Succeed { id });
                    done_count += 1;
                    if done_count == par {
                        sender.send(Response::SucceedGlobal {
                            id: EntryId {
                                entry_id: id.entry_id,
                            },
                        });
                    }
                }
                worker::Response::Failed { id, reason } => {
                    sender.send(Response::Failed { id, reason });
                }
            }
        }
    });
    cmd_sender
}

fn calculate_ranges(total: usize, par: usize) -> Vec<(usize, usize)> {
    todo!()
}
