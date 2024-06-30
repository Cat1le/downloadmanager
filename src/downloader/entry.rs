use std::{sync::Arc, thread};

use reqwest::{Client, Url};
use tokio::{
    fs::{self, File},
    runtime::Runtime,
    sync::mpsc::{unbounded_channel, UnboundedSender},
};

use super::worker::{self, WorkerData, WorkerId};

macro_rules! or_fail_global {
    ($expr:expr, $sender:expr, $id:expr) => {
        match $expr {
            Ok(x) => x,
            Err(e) => {
                $sender
                    .send(Response::FailedGlobal {
                        id: $id,
                        reason: format!("{e}"),
                    })
                    .unwrap();
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
    pub id: usize,
    pub url: String,
    pub name: String,
    pub runtime: Arc<Runtime>,
    pub client: Client,
}

pub struct EntryId {
    pub entry_id: usize,
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
        name,
        runtime,
        client,
    }: EntryData,
) {
    let url = or_fail_global!(Url::parse(&url), sender, EntryId { entry_id: id });
    let (sx, mut rx) = unbounded_channel();
    let sender_ref = sender.clone();
    let runtime_ref = runtime.clone();
    let par = thread::available_parallelism().unwrap().get();
    runtime.spawn(async move {
        let id = EntryId { entry_id: id };
        let resp = or_fail_global!(client.get(url.clone()).send().await, sender_ref, id);
        let content_length = or_fail_global!(
            resp.content_length().filter(|&x| x != 0).ok_or("No Content-Length header"),
            sender_ref,
            id
        ) as usize;
        drop(resp);
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
                    runtime: runtime_ref.clone(),
                    client: client.clone(),
                },
            ));
        }
    });
    runtime.spawn(async move {
        let mut done = Vec::with_capacity(par);
        while let Some(resp) = rx.recv().await {
            match resp {
                worker::Response::MadeProgress { id, value } => {
                    sender.send(Response::MadeProgress { id, value }).unwrap();
                }
                worker::Response::Succeed { id, filepath } => {
                    sender.send(Response::Succeed { id: id.clone() }).unwrap();
                    done.push(filepath);
                    if done.len() == par {
                        let mut file = File::create(name.clone()).await.unwrap();
                        done.sort_by_key(|x| {
                            let fname = x.file_name().unwrap().to_str().unwrap();
                            fname.split('-').nth(2).unwrap().parse::<usize>().unwrap()
                        });
                        for d in done {
                            let mut d_file = tokio::fs::File::open(&d).await.unwrap();
                            tokio::io::copy(&mut d_file, &mut file).await.unwrap();
                            fs::remove_file(d).await.unwrap();
                        }
                        sender
                            .send(Response::SucceedGlobal {
                                id: EntryId {
                                    entry_id: id.entry_id,
                                },
                            })
                            .unwrap();
                        break;
                    }
                }
                worker::Response::Failed { id, reason } => {
                    sender.send(Response::Failed { id, reason }).unwrap();
                }
            }
        }
    });
}

fn min(a: usize, b: usize) -> usize {
    if a > b {
        b
    } else {
        a
    }
}

fn calculate_ranges(mut total: usize, par: usize) -> Vec<(usize, usize)> {
    let mut v = Vec::new();
    let mut start = 0;
    let mut item = total / par;
    while total > 0 {
        item = min(total, item);
        v.push((start, start + item - 1));
        start += item;
        total -= item;
    }
    v
}
