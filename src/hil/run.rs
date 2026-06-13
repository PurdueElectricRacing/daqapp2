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
        preset_info: Option<hil::config::PresetInfo>,
        tests: Vec<HilRunningTest>,
    },
}

impl ExpectResult {
    pub fn as_str(&self) -> &'static str {
        match self {
            ExpectResult::NotInWindow => "Not in window",
            ExpectResult::InProgress => "In progress",
            ExpectResult::Passed => "Passed",
            ExpectResult::Failed => "Failed",
        }
    }
}

impl HilRunningTest {
    pub fn new(test_info: &hil::config::TestInfo) -> Result<Self, String> {
        let test = hil::config::load_test_from_file(&test_info.basename)?;
        let mut in_progress_expects = test
            .expect
            .into_iter()
            .map(InProgressExpect::new)
            .collect::<Vec<_>>();
        in_progress_expects
            .sort_by(|a, b| a.expect.window[0].partial_cmp(&b.expect.window[0]).unwrap());
        Ok(Self {
            name: test_info.name.clone(),
            description: test_info.description.clone(),
            tx_remaining: test.tx,
            in_progress_expects,
        })
    }

    pub fn expect_counts(&self) -> (usize, usize, usize) {
        let mut not_in_window = 0;
        let mut in_progress = 0;
        let mut completed = 0;

        for expect in &self.in_progress_expects {
            match expect.result {
                ExpectResult::NotInWindow => not_in_window += 1,
                ExpectResult::InProgress => in_progress += 1,
                ExpectResult::Passed | ExpectResult::Failed => completed += 1,
            }
        }

        (not_in_window, in_progress, completed)
    }
}

impl InProgressExpect {
    pub fn new(expect: hil::config::Expectation) -> Self {
        Self {
            expect,
            result: ExpectResult::NotInWindow,
        }
    }
}
