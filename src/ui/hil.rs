use crate::{hil, messages, ui};
use eframe::egui;

pub struct Hil {
    pub title: String,
    pub found_presets: Vec<hil::config::PresetInfo>,
    pub found_tests: Vec<hil::config::TestInfo>,
    pub load_errors: Vec<String>,
    pub start_error: Option<String>,
    pub hil_state: hil::run::HilState,
}

impl Hil {
    pub fn new(instance_num: usize) -> Self {
        let (presets, tests, errors) = hil::config::list_available_tests();

        Self {
            title: format!("HIL #{}", instance_num),
            found_presets: presets,
            found_tests: tests,
            load_errors: errors,
            start_error: None,
            hil_state: hil::run::HilState::Idle,
        }
    }

    pub fn handle_can_message(&mut self, msg: &messages::MsgFromCan) {}

    fn reload_tests(&mut self) {
        let (presets, tests, errors) = hil::config::list_available_tests();
        self.found_presets = presets;
        self.found_tests = tests;
        self.load_errors = errors;
    }

    fn try_start_from_test(&mut self, test: &hil::config::TestInfo) {
        let test = match hil::run::HilRunningTest::new(test) {
            Ok(test) => test,
            Err(err) => {
                self.start_error = Some(format!("Error starting test: {}", err));
                return;
            }
        };

        self.hil_state = hil::run::HilState::Running {
            start_time: std::time::Instant::now(),
            preset_info: None,
            tests: vec![test],
        };
    }

    fn try_start_from_preset(&mut self, preset: &hil::config::PresetInfo) {
        let mut tests = Vec::new();
        for test_name in &preset.tests {
            let test_info = match self.found_tests.iter().find(|t| t.basename == *test_name) {
                Some(info) => info,
                None => {
                    self.start_error = Some(format!(
                        "Error starting preset: Test '{}' not found",
                        test_name
                    ));
                    return;
                }
            };
            let test = match hil::run::HilRunningTest::new(test_info) {
                Ok(test) => test,
                Err(err) => {
                    self.start_error = Some(format!("Error starting preset: {}", err));
                    return;
                }
            };
            tests.push(test);
        }

        self.hil_state = hil::run::HilState::Running {
            start_time: std::time::Instant::now(),
            preset_info: Some(preset.clone()),
            tests,
        };
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> egui_tiles::UiResponse {
        egui::ScrollArea::vertical().show(ui, |ui| match &mut self.hil_state {
            hil::run::HilState::Idle => {
                ui.label("HIL is idle. Select a preset or test to run.");
                if ui.button("Reload Tests").clicked() {
                    self.reload_tests();
                }
                ui.separator();
                if !self.load_errors.is_empty() {
                    ui.label("Errors loading tests:");
                    for err in &self.load_errors {
                        ui.label(format!("- {}", err));
                    }
                    ui.separator();
                }
                if !self.found_presets.is_empty() {
                    ui.label("Presets:");
                    let mut selected_preset = None;
                    for preset in &self.found_presets {
                        let preset_info = format!("{} - {}", preset.name, preset.tests.join(", "));
                        if ui.button(&preset_info).clicked() {
                            selected_preset = Some(preset.clone());
                        }
                    }
                    if let Some(preset) = selected_preset {
                        self.try_start_from_preset(&preset);
                    }
                    ui.separator();
                }
                if !self.found_tests.is_empty() {
                    ui.label("Individual Tests:");
                    let mut selected_test = None;
                    for test in &self.found_tests {
                        let test_info =
                            format!("{} [{}]: {}", test.name, test.basename, test.description);
                        if ui.button(&test_info).clicked() {
                            selected_test = Some(test.clone());
                        }
                    }
                    if let Some(test) = selected_test {
                        self.try_start_from_test(&test);
                    }
                }
            }
            hil::run::HilState::Running {
                start_time,
                preset_info,
                tests,
            } => {
                let time_since_start = start_time.elapsed().as_secs_f64();
                ui.label(format!(
                    "HIL is running... {:.2} seconds since start",
                    time_since_start
                ));
                ui.separator();

                if let Some(preset) = preset_info {
                    ui.heading(format!("Preset: {}", preset.name));
                    ui.label(format!("Tests: {}", preset.tests.join(", ")));
                    ui.separator();
                }

                for test in tests {
                    ui.heading(&test.name);
                    ui.label(&test.description);

                    ui.label(format!(
                        "TX Remaining: {} messages",
                        test.tx_remaining.len()
                    ));

                    let (not_in_window, in_progress, completed) = test.expect_counts();
                    ui.label(format!(
                        "Expects: {}/{}/{}",
                        not_in_window, in_progress, completed
                    ));

                    for ipe in &test.in_progress_expects {
                        let status = ipe.result.as_str();
                        ui.label(format!(
                            "Expect: {} [{} - {}] - {}",
                            ipe.expect.msg_name, ipe.expect.window[0], ipe.expect.window[1], status
                        ));
                        for (sig_name, sig_window) in &ipe.expect.signals {
                            ui.label(format!(
                                "  Signal: {} [{} - {}]",
                                sig_name, sig_window[0], sig_window[1]
                            ));
                        }
                    }

                    ui.separator();
                }
            }
        });
        egui_tiles::UiResponse::None
    }
}
