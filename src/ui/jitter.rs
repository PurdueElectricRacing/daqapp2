use crate::{app, messages, util};
use eframe::egui;

pub struct Jitter {
    pub title: String,

    search_text: String,
    search_results: Vec<can_dbc::Message>,

    selected_msg: Option<can_dbc::Message>,
    period_ms: usize,

    active: bool,
    last_timestamp: Option<chrono::DateTime<chrono::Local>>,

    interval_count: u64,
    max_pct: f64,
    sum_pct: f64,
}

impl Jitter {
    pub fn new(instance_num: usize) -> Self {
        Self {
            title: format!("Jitter #{}", instance_num),

            search_text: String::new(),
            search_results: Vec::new(),

            selected_msg: None,
            period_ms: 100,

            active: false,
            last_timestamp: None,

            interval_count: 0,
            max_pct: 0.0,
            sum_pct: 0.0,
        }
    }

    fn reset_stats(&mut self) {
        self.last_timestamp = None;
        self.interval_count = 0;
        self.max_pct = 0.0;
        self.sum_pct = 0.0;
    }

    fn selected_msg_id(&self) -> Option<u32> {
        self.selected_msg
            .as_ref()
            .map(|m| util::msg_id::can_dbc_to_u32_without_extid_flag(&m.id))
    }

    pub fn handle_can_message(&mut self, msg: &messages::MsgFromCan) {
        if !self.active {
            return;
        }

        let Some(target_id) = self.selected_msg_id() else {
            return;
        };

        let messages::MsgFromCan::ParsedMessage(parsed_msg) = msg else {
            return;
        };

        if parsed_msg.decoded.msg_id != target_id {
            return;
        }

        let ts = parsed_msg.timestamp;
        if let Some(prev) = self.last_timestamp {
            let delta_ms = (ts - prev).num_milliseconds() as f64;
            let t = self.period_ms as f64;
            let pct = 100.0 * (delta_ms - t).abs() / t;

            self.interval_count += 1;
            self.sum_pct += pct;
            self.max_pct = self.max_pct.max(pct);
        }
        self.last_timestamp = Some(ts);
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        parser: Option<&app::ParserInfo>,
    ) -> egui_tiles::UiResponse {
        let Some(parser) = parser else {
            ui.vertical_centered(|ui| {
                ui.label("No DBC selected yet.");
                ui.label("CMD+S to toggle the sidebar.");
                ui.label("Use the sidebar to select a DBC file");
            });

            return egui_tiles::UiResponse::None;
        };

        let period_ok = self.period_ms > 0;

        egui::Frame::group(ui.style())
            .inner_margin(egui::Margin::symmetric(8, 6))
            .stroke(egui::Stroke::NONE)
            .show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Search:");
                        if ui
                            .add(
                                egui::TextEdit::singleline(&mut self.search_text)
                                    .hint_text("Message name..."),
                            )
                            .changed()
                        {
                            if self.search_text.is_empty() {
                                self.search_results.clear();
                            } else if self.search_text.trim() == "*" {
                                self.search_results = parser.parser.msg_defs().clone();
                            } else {
                                let search_lower = self.search_text.to_lowercase();
                                self.search_results = parser
                                    .parser
                                    .msg_defs()
                                    .iter()
                                    .filter(|msg| {
                                        let id_str = format!(
                                            "0x{:03X}",
                                            util::msg_id::can_dbc_to_u32_without_extid_flag(&msg.id)
                                        );
                                        id_str.contains(&search_lower)
                                            || msg.name.to_lowercase().contains(&search_lower)
                                    })
                                    .cloned()
                                    .collect();
                            }
                        }
                    });

                    ui.add_space(8.0);

                    if self.search_results.is_empty() && !self.search_text.is_empty() {
                        ui.label(egui::RichText::new("No messages found.").italics().weak());
                    } else if self.search_text.is_empty() && self.selected_msg.is_none() {
                        ui.label(
                            egui::RichText::new(
                                "Start typing to search for messages... (Use * to show all messages.)",
                            )
                            .italics()
                            .weak(),
                        );
                    } else {
                        let mut should_clear_search = false;
                        for msg in &self.search_results {
                            if ui
                                .button(format!(
                                    "{} (0x{:03X})",
                                    msg.name,
                                    util::msg_id::can_dbc_to_u32_without_extid_flag(&msg.id)
                                ))
                                .clicked()
                            {
                                self.selected_msg = Some(msg.clone());
                                should_clear_search = true;
                            }
                        }
                        if should_clear_search {
                            self.search_text.clear();
                            self.search_results.clear();
                        }
                    }

                    if let Some(selected_msg) = &self.selected_msg {
                        ui.separator();

                        ui.label(
                            egui::RichText::new(format!(
                                "Selected Message: {} (0x{:03X})",
                                selected_msg.name,
                                util::msg_id::can_dbc_to_u32_without_extid_flag(&selected_msg.id)
                            ))
                            .strong()
                            .size(16.0),
                        );

                        ui.horizontal(|ui| {
                            ui.label("Nominal period:");
                            ui.add(
                                egui::DragValue::new(&mut self.period_ms)
                                    .speed(1)
                                    .range(1..=1_000_000)
                                    .suffix(" ms"),
                            );

                            ui.separator();

                            let start_stop = if self.active { "Stop" } else { "Start" };
                            if ui
                                .add_enabled(period_ok, egui::Button::new(start_stop))
                                .clicked()
                            {
                                self.active = !self.active;
                                if self.active {
                                    self.reset_stats();
                                }
                            }

                            if ui
                                .add_enabled(self.interval_count > 0, egui::Button::new("Clear"))
                                .clicked()
                            {
                                self.reset_stats();
                            }
                        });

                        if !period_ok {
                            ui.label(
                                egui::RichText::new("Period must be greater than zero.")
                                    .color(ui.visuals().error_fg_color),
                            );
                        }

                        ui.separator();
                        ui.label(egui::RichText::new("Statistics").strong());
                        ui.label("(Absolute deviation from nominal period, % of period)");

                        let status = if self.active {
                            "Monitoring"
                        } else {
                            "Stopped"
                        };
                        ui.label(format!("Status: {}", status));

                        if self.interval_count == 0 {
                            ui.label(egui::RichText::new("Intervals recorded: 0").weak());
                            ui.label(
                                egui::RichText::new("Max / avg: — (need at least one interval)")
                                    .weak(),
                            );
                        } else {
                            let avg = self.sum_pct / self.interval_count as f64;
                            ui.label(format!("Intervals recorded: {}", self.interval_count));
                            ui.label(format!(
                                "Max: {:.2}%    Avg: {:.2}%",
                                self.max_pct, avg
                            ));
                        }
                    }
                });
            });

        egui_tiles::UiResponse::None
    }
}
