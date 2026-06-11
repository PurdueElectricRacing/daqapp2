// Read HIL config files (preset and tests)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PresetsFile(pub HashMap<String, Vec<String>>);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestFile {
    pub name: String,
    pub description: String,

    #[serde(default)]
    pub tx: Vec<TxMessage>,

    pub expect: Vec<Expectation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxMessage {
    pub timestamp: f64,
    pub msg_name: String,

    pub signals: HashMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Expectation {
    pub window: [f64; 2],

    pub msg_name: String,

    #[serde(default)]
    pub signals: HashMap<String, [f64; 2]>,
}
