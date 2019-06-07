use jsonrpc_types::JsonBytes;
use numext_fixed_hash::H256;
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;
use toml::Value;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MinerConfig {
    pub client: ClientConfig,
    pub workers: Vec<Value>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ClientConfig {
    pub rpc_url: String,
    pub poll_interval: u64,
    pub block_on_submit: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum CuckooParams {
    Simple(CuckooSimple),
    Lean(CuckooLean),
}

#[derive(Deserialize, Serialize, Copy, Clone, Eq, PartialEq, Hash, Debug)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum Distribution {
    Constant { value: u64 },
    Uniform { low: u64, high: u64 },
    Normal { mean: u64, std_dev: u64 },
    Poisson { lambda: u64 },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CuckooSimple {
    pub threads: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CuckooLean {
    pub device_id: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BlockAssemblerConfig {
    pub code_hash: H256,
    pub args: Vec<JsonBytes>,
}
