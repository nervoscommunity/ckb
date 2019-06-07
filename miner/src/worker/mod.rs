mod cuckoo_simple;
mod dummy;

use crate::config::{CuckooParams, Distribution};
use ckb_core::header::Seal;
use ckb_logger::error;
use ckb_pow::{CuckooEngine, DummyPowEngine, PowEngine};
use crossbeam_channel::{unbounded, Sender};
use cuckoo_simple::CuckooSimple;
use dummy::{Delay, Dummy};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use numext_fixed_hash::H256;
use std::sync::Arc;
use std::thread;
use toml::Value;

#[derive(Clone)]
pub enum WorkerMessage {
    Stop,
    Start,
    NewWork(H256),
}

pub struct WorkerController {
    inner: Vec<Sender<WorkerMessage>>,
}

impl WorkerController {
    pub fn new(inner: Vec<Sender<WorkerMessage>>) -> Self {
        Self { inner }
    }

    pub fn send_message(&self, message: WorkerMessage) {
        for worker_tx in self.inner.iter() {
            if let Err(err) = worker_tx.send(message.clone()) {
                error!("worker_tx send error {:?}", err);
            };
        }
    }
}

fn parse_work_config<'de, T: serde::de::Deserialize<'de>>(config: &Value) -> T {
    config.clone().try_into().unwrap()
}

pub fn start_worker(
    pow: Arc<dyn PowEngine>,
    config: &Value,
    seal_tx: Sender<(H256, Seal)>,
) -> WorkerController {
    let mp = MultiProgress::new();
    let spinner_style = ProgressStyle::default_bar()
        .template("{prefix:.bold.dim} {spinner:.green} [{elapsed_precise}] {wide_msg}");

    if let Some(_dummy_engine) = pow.as_any().downcast_ref::<DummyPowEngine>() {
        let worker_name = "Dummy-Worker";
        let pb = mp.add(ProgressBar::new(100));
        pb.set_style(spinner_style.clone());
        pb.set_prefix(&worker_name);

        let (worker_tx, worker_rx) = unbounded();
        let dis = parse_work_config::<Distribution>(config);
        let mut worker = Dummy::new(dis, seal_tx, worker_rx);

        thread::Builder::new()
            .name(worker_name.to_string())
            .spawn(move || {
                worker.run(pb);
            })
            .expect("Start `Dummy` worker thread failed");
        return WorkerController::new(vec![worker_tx]);
    }

    if let Some(cuckoo_engine) = pow.as_any().downcast_ref::<CuckooEngine>() {
        let params = parse_work_config::<CuckooParams>(config);
        let controller = match params {
            CuckooParams::Simple(params) => {
                let worker_txs = (0..params.threads)
                    .map(|i| {
                        let worker_name = format!("CuckooSimple-Worker-{}", i);
                        // `100` is the len of progress bar, we can use any dummy value here, since we only show the spinner in console.
                        // `17` is a prime number, it will draw each spinner char in different ticks.
                        let pb = mp.add(ProgressBar::new(100));
                        pb.set_draw_delta(17);
                        pb.set_style(spinner_style.clone());
                        pb.set_prefix(&worker_name);

                        let (worker_tx, worker_rx) = unbounded();
                        let (cuckoo, seal_tx) = (cuckoo_engine.cuckoo.clone(), seal_tx.clone());
                        thread::Builder::new()
                            .name(worker_name)
                            .spawn(move || {
                                let mut worker = CuckooSimple::new(cuckoo, seal_tx, worker_rx);
                                worker.run(pb);
                            })
                            .expect("Start `CuckooSimple` worker thread failed");
                        worker_tx
                    })
                    .collect();

                thread::spawn(move || {
                    mp.join().expect("MultiProgress join failed");
                });
                WorkerController::new(worker_txs)
            }
            _ => {
                unimplemented!();
            }
        };
        return controller;
    }

    panic!("Unknown Pow Engine");
}

pub trait Worker {
    fn run(&mut self, progress_bar: ProgressBar);
}



#[cfg(test)]
mod tests {
    use super::*;
    use serde_derive::{Deserialize, Serialize};

    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    pub struct TestConfig {
        pub workers: Vec<Value>,
    }

    #[test]
    fn parse_cuckoo() {
        let config: TestConfig = toml::from_str(r#"
            [[workers]]
            mode      = "simple"
            threads   = 1

            [[workers]]
            mode        = "lean"
            device_id   = "1"
        "#).unwrap();

        for worker in config.workers  {
            let _ = parse_work_config::<CuckooParams>(&worker);
        }
    }

    #[test]
    fn parse_dummy() {
        let config: TestConfig = toml::from_str(r#"
            [[workers]]
            mode   = "constant"
            value  = 5000
        "#).unwrap();

        for worker in config.workers {
            let _ = parse_work_config::<Distribution>(&worker);
        }
    }
}
