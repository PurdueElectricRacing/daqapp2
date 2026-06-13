// Read HIL config files (preset and tests)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const PRESETS_FILE: &str = "hil_config/presets.json";
pub const TESTS_FOLDER: &str = "hil_config/tests/";

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

pub struct PresetInfo {
    pub name: String,
    pub subtests: Vec<String>,
}

pub struct TestInfo {
    pub basename: String,
    pub name: String,
    pub description: String,
}

pub fn list_available_tests() -> (Vec<PresetInfo>, Vec<TestInfo>, Vec<String>) {
    let mut errors = Vec::new();

    // Load individual tests
    let mut individual_tests = Vec::new();
    if let Ok(entries) = std::fs::read_dir(TESTS_FOLDER) {
        for entry in entries.flatten() {
            if let Ok(content) = std::fs::read_to_string(entry.path()) {
                if let Ok(test_data) = serde_json::from_str::<TestFile>(&content) {
                    individual_tests.push(TestInfo {
                        basename: entry
                            .path()
                            .file_stem()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string(),
                        name: test_data.name,
                        description: test_data.description,
                    });
                } else {
                    errors.push(format!("Failed to parse test file: {:?}", entry.path()));
                }
            } else {
                errors.push(format!("Failed to read test file: {:?}", entry.path()));
            }
        }
    } else {
        errors.push(format!("Failed to read tests directory: {}", TESTS_FOLDER));
    }

    // Load presets
    let mut presets = Vec::new();
    if let Ok(presets_file) = std::fs::read_to_string(PRESETS_FILE) {
        if let Ok(presets_data) = serde_json::from_str::<PresetsFile>(&presets_file) {
            for (preset_name, subtests) in presets_data.0 {
                if subtests.is_empty() {
                    errors.push(format!("Preset '{}' has no subtests defined", preset_name));
                    continue;
                }

                if subtests.iter().any(|s| s.trim().is_empty()) {
                    errors.push(format!(
                        "Preset '{}' contains empty subtest names",
                        preset_name
                    ));
                    continue;
                }

                if subtests
                    .iter()
                    .any(|s| individual_tests.iter().all(|t| t.basename != *s))
                {
                    errors.push(format!(
                        "Preset '{}' contains subtests that do not exist: {:?}",
                        preset_name,
                        subtests
                            .iter()
                            .filter(|s| individual_tests.iter().all(|t| t.basename != **s))
                            .collect::<Vec<_>>()
                    ));
                    continue;
                }

                presets.push(PresetInfo {
                    name: preset_name,
                    subtests,
                });
            }
        } else {
            errors.push(format!("Failed to parse presets file: {}", PRESETS_FILE));
        }
    } else {
        errors.push(format!("Failed to read presets file: {}", PRESETS_FILE));
    }

    (presets, individual_tests, errors)
}
