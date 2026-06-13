use crate::{hil, messages, ui};
use eframe::egui;

pub struct Hil {
    pub title: String,
    pub found_presets: Vec<hil::config::PresetInfo>,
    pub found_tests: Vec<hil::config::TestInfo>,
    pub load_errors: Vec<String>,
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

    pub fn show(&mut self, ui: &mut egui::Ui) -> egui_tiles::UiResponse {
        egui::ScrollArea::vertical().show(ui, |ui| match &mut self.hil_state {
            hil::run::HilState::Idle => {
                ui.label("HIL is idle. Select a preset or test to run.");
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
                    for preset in &self.found_presets {
                        if ui.button(&preset.name).clicked() {
                            self.hil_state = hil::run::HilState::Running {
                                start_time: std::time::Instant::now(),
                                preset_name: Some(preset.name.clone()),
                                tests: Vec::new(),
                            };
                        }
                    }
                    ui.separator();
                }
                if !self.found_tests.is_empty() {
                    ui.label("Individual Tests:");
                    for test in &self.found_tests {
                        let test_info = format!("{}: {}", test.name, test.basename);
                        if ui.button(&test_info).clicked() {
                            self.hil_state = hil::run::HilState::Running {
                                start_time: std::time::Instant::now(),
                                preset_name: None,
                                // tests: vec![hil::run::HilRunningTest {
                                // 	name: test.name.clone(),
                                // 	description: test.description.clone(),
                                // 	tx_remaining: test.tx.clone(),
                                // 	expect: test.expect.clone(),
                                // }],
                                tests: Vec::new(),
                            };
                        }
                    }
                }
            }
            _ => {}
        });
        egui_tiles::UiResponse::None
    }
}
