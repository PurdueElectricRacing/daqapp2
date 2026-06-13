use crate::hil::{self, config};

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
    pub in_progress_expects: Vec<InProgressExpect>,
}

pub enum HilState {
    Idle,
    Running {
        start_time: std::time::Instant,
        preset_name: Option<String>,
        tests: Vec<HilRunningTest>,
    },
}

impl HilRunningTest {
    pub fn new(test_info: &hil::config::TestInfo) -> Result<Self, String> {
        let test = hil::config::load_test_from_file(&test_info.basename)?;
        Ok(Self {
            name: test_info.name.clone(),
            description: test_info.description.clone(),
            tx_remaining: test.tx,
            in_progress_expects: test
                .expect
                .into_iter()
                .map(|expect| InProgressExpect {
                    expect,
                    result: ExpectResult::NotInWindow,
                })
                .collect(),
        })
    }
}
