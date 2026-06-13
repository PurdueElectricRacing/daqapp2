use crate::hil;

pub enum ExpectResult {
    NotInWindow,
    InProgress,
    Passed,
    Failed,
}

pub struct InProgressExpect {
    pub expect: hil::config::Expectation,
    pub result: ExpectResult,
}

pub struct HilRunningTest {
    pub name: String,
    pub description: String,

    pub tx_remaining: Vec<hil::config::TxMessage>,
    pub expect: Vec<hil::config::Expectation>,
}

pub enum HilState {
    Idle,
    Running {
        start_time: std::time::Instant,
        preset_name: Option<String>,
        tests: Vec<HilRunningTest>,
    },
}
