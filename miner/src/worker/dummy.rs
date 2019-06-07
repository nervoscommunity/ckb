use super::{Worker, WorkerMessage};
use crate::config::Distribution;
use ckb_core::header::Seal;
use ckb_logger::error;
use crossbeam_channel::{Receiver, Sender};
use indicatif::ProgressBar;
use numext_fixed_hash::H256;
use rand::{
    distributions::{self as dist, Distribution as _},
    random, thread_rng,
};
use serde_derive::{Deserialize, Serialize};
use std::thread;
use std::time::Duration;

pub struct Dummy {
    delay: Delay,
    start: bool,
    pow_hash: Option<H256>,
    seal_tx: Sender<(H256, Seal)>,
    worker_rx: Receiver<WorkerMessage>,
}

pub enum Delay {
    Constant(u64),
    Uniform(dist::Uniform<u64>),
    Normal(dist::Normal),
    Poisson(dist::Poisson),
}

impl From<Distribution> for Delay {
    fn from(dis: Distribution) -> Delay {
        match dis {
            Distribution::Constant { value } => Delay::Constant(value),
            Distribution::Uniform { low, high } => Delay::Uniform(dist::Uniform::new(low, high)),
            Distribution::Normal { mean, std_dev } => {
                Delay::Normal(dist::Normal::new(mean as f64, std_dev as f64))
            }
            Distribution::Poisson { lambda } => Delay::Poisson(dist::Poisson::new(lambda as f64)),
        }
    }
}

impl Default for Delay {
    fn default() -> Self {
        Delay::Constant(5000)
    }
}

impl Delay {
    fn duration(&self) -> Duration {
        let mut rng = thread_rng();
        let millis = match self {
            Delay::Constant(v) => *v,
            Delay::Uniform(ref d) => d.sample(&mut rng),
            Delay::Normal(ref d) => d.sample(&mut rng) as u64,
            Delay::Poisson(ref d) => d.sample(&mut rng),
        };
        Duration::from_millis(millis)
    }
}

impl Dummy {
    pub fn new(
        dis: Distribution,
        seal_tx: Sender<(H256, Seal)>,
        worker_rx: Receiver<WorkerMessage>,
    ) -> Self {
        Self {
            start: true,
            pow_hash: None,
            delay: dis.into(),
            seal_tx,
            worker_rx,
        }
    }

    fn poll_worker_message(&mut self) {
        if let Ok(msg) = self.worker_rx.try_recv() {
            match msg {
                WorkerMessage::NewWork(pow_hash) => self.pow_hash = Some(pow_hash),
                WorkerMessage::Stop => {
                    self.start = false;
                }
                WorkerMessage::Start => {
                    self.start = true;
                }
            }
        }
    }

    fn solve(&self, pow_hash: &H256, nonce: u64) {
        thread::sleep(self.delay.duration());
        let seal = Seal::new(nonce, Vec::new().into());
        if let Err(err) = self.seal_tx.send((pow_hash.clone(), seal)) {
            error!("seal_tx send error {:?}", err);
        }
    }
}

impl Worker for Dummy {
    fn run(&mut self, _progress_bar: ProgressBar) {
        loop {
            self.poll_worker_message();
            if self.start {
                if let Some(pow_hash) = &self.pow_hash {
                    self.solve(pow_hash, random());
                }
            } else {
                thread::sleep(Duration::from_millis(100));
            }
        }
    }
}
