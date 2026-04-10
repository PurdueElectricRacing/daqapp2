use crate::{messages, ui};
use eframe::egui::{self, Color32, Frame, RichText, Stroke};

const NUM_MODULES: usize = 8;
const CELLS_PER_MODULE: usize = 16;

// ─── Data model ────────────────────────────────────────────────────────────────

#[derive(Default, Clone)]
pub struct CellData {
    pub voltage: f32,
    pub balancing: bool,
}

pub struct BatteryPage {
    pub title: String,
    pub modules: Vec<Vec<CellData>>, // [module_idx][cell_idx]

    pub last_update: std::time::Instant,
    pub is_data_stale: bool,
    pub timeout_seconds: u64,
}

impl BatteryPage {
    pub fn new(instance_num: usize) -> Self {
        Self {
            title: format!("Battery Viewer #{}", instance_num),
            modules: vec![vec![CellData::default(); CELLS_PER_MODULE]; NUM_MODULES],
            last_update: std::time::Instant::now() - std::time::Duration::from_secs(10),
            is_data_stale: true,
            timeout_seconds: 2,
        }
    }

    // ─── CAN handler ──────────────────────────────────────────────────────────

    pub fn handle_can_message(&mut self, msg: &messages::MsgFromCan) {
        if let messages::MsgFromCan::ParsedMessage(parsed) = msg {
            match parsed.decoded.name.as_str() {
                "cell_telemetry" => {
                    let mut module_num: Option<usize> = None;
                    let mut cell_num: Option<usize> = None;
                    let mut voltage: Option<f32> = None;
                    let mut balancing: Option<bool> = None;

                    for (_, sig) in parsed.decoded.signals.iter() {
                        match sig.name.as_str() {
                            "module_num" => {
                                if let can_decode::DecodedSignalValue::Numeric(v) = &sig.value {
                                    module_num = Some(*v as usize);
                                }
                            }
                            "cell_num" => {
                                if let can_decode::DecodedSignalValue::Numeric(v) = &sig.value {
                                    cell_num = Some(*v as usize);
                                }
                            }
                            "cell_voltage" => {
                                if let can_decode::DecodedSignalValue::Numeric(v) = &sig.value {
                                    voltage = Some(*v as f32 * 0.01); // scale
                                }
                            }
                            "balance_status" => {
                                if let can_decode::DecodedSignalValue::Enum(v, _) = &sig.value {
                                    balancing = Some(*v != 0);
                                }
                            }
                            _ => {}
                        }
                    }

                    if let (Some(m), Some(c), Some(v), Some(b)) =
                        (module_num, cell_num, voltage, balancing)
                    {
                        if m < self.modules.len() && c < self.modules[m].len() {
                            self.modules[m][c].voltage = v;
                            self.modules[m][c].balancing = b;
                        }
                    }

                    self.last_update = std::time::Instant::now();
                    self.is_data_stale = false;
                }
                _ => {}
            }
        }
    }

    // ─── UI ───────────────────────────────────────────────────────────────────

    pub fn show(&mut self, ui: &mut egui::Ui) -> egui_tiles::UiResponse {
        let theme = ui::theme::get_theme(ui.ctx());

        self.is_data_stale =
            self.last_update.elapsed() > std::time::Duration::from_secs(self.timeout_seconds);
        let stale = self.is_data_stale;
        let elapsed = self.last_update.elapsed().as_secs_f32();

        // ── collect pack-level stats ─────────────────────────────────────────
        let all_voltages: Vec<f32> = self
            .modules
            .iter()
            .flat_map(|m| m.iter().map(|c| c.voltage))
            .filter(|v| *v > 0.0)
            .collect();

        let pack_sum: f32 = all_voltages.iter().sum();
        let pack_min = all_voltages.iter().cloned().fold(f32::MAX, f32::min);
        let pack_max = all_voltages.iter().cloned().fold(f32::MIN, f32::max);
        let pack_delta = if pack_min < f32::MAX {
            pack_max - pack_min
        } else {
            0.0
        };

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(4.0);
            ui.heading(&self.title);
            ui.add_space(4.0);

            // ── staleness banner ─────────────────────────────────────────────
            {
                let (bg, dot, text) = if stale {
                    let c = theme.warning_color();
                    (
                        Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), 30),
                        c,
                        format!("No data — last message {:.1} s ago", elapsed),
                    )
                } else {
                    let c = theme.success_color();
                    (
                        Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), 30),
                        c,
                        format!("Live — last message {:.1} s ago", elapsed),
                    )
                };
                Frame::NONE
                    .fill(bg)
                    .stroke(Stroke::new(1.0, dot.linear_multiply(0.5)))
                    .inner_margin(egui::Margin::symmetric(10, 6))
                    .corner_radius(egui::CornerRadius::same(4))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let (rect, _) = ui
                                .allocate_exact_size(egui::Vec2::splat(8.0), egui::Sense::hover());
                            ui.painter().circle_filled(rect.center(), 4.0, dot);
                            ui.add_space(4.0);
                            ui.colored_label(dot, &text);
                        });
                    });
            }

            ui.add_space(8.0);

            // ── pack summary cards ───────────────────────────────────────────
            ui.label(
                RichText::new("PACK SUMMARY")
                    .size(10.0)
                    .color(theme.text_color().linear_multiply(0.5)),
            );
            ui.add_space(4.0);

            ui.columns(4, |cols| {
                Self::stat_card(&mut cols[0], &theme, "PACK SUM", pack_sum, "V", stale, None);
                Self::stat_card(
                    &mut cols[1],
                    &theme,
                    "CELL MIN",
                    if pack_min < f32::MAX { pack_min } else { 0.0 },
                    "V",
                    stale,
                    None,
                );
                Self::stat_card(
                    &mut cols[2],
                    &theme,
                    "CELL MAX",
                    if pack_max > f32::MIN { pack_max } else { 0.0 },
                    "V",
                    stale,
                    None,
                );
                Self::stat_card(
                    &mut cols[3],
                    &theme,
                    "DELTA",
                    pack_delta,
                    "V",
                    stale,
                    Some(if pack_delta > 0.050 {
                        theme.error_color()
                    } else if pack_delta > 0.020 {
                        theme.warning_color()
                    } else {
                        theme.success_color()
                    }),
                );
            });

            ui.add_space(12.0);

            // ── per-module panels ────────────────────────────────────────────
            for (mi, module) in self.modules.iter().enumerate() {
                let mvs: Vec<f32> = module.iter().map(|c| c.voltage).collect();
                let mod_sum: f32 = mvs.iter().sum();
                let mod_min = mvs.iter().cloned().fold(f32::MAX, f32::min);
                let mod_max = mvs.iter().cloned().fold(f32::MIN, f32::max);
                let mod_delta = if mod_min < f32::MAX {
                    mod_max - mod_min
                } else {
                    0.0
                };

                Frame::NONE
                    .fill(theme.panel_color())
                    .stroke(Stroke::new(1.0, theme.accent_color()))
                    .inner_margin(egui::Margin::same(10))
                    .corner_radius(egui::CornerRadius::same(4))
                    .show(ui, |ui| {
                        // module header row
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(format!("MODULE {mi}")).size(11.0).strong());
                            ui.add_space(8.0);
                            ui.label(
                                RichText::new(format!("sum {:.2} V", mod_sum))
                                    .size(10.0)
                                    .color(theme.text_color().linear_multiply(0.55)),
                            );
                            ui.label(
                                RichText::new(format!("min {:.3} V", mod_min))
                                    .size(10.0)
                                    .color(theme.text_color().linear_multiply(0.55)),
                            );
                            ui.label(
                                RichText::new(format!("max {:.3} V", mod_max))
                                    .size(10.0)
                                    .color(theme.text_color().linear_multiply(0.55)),
                            );
                            ui.label(
                                RichText::new(format!("Δ {:.3} V", mod_delta))
                                    .size(10.0)
                                    .color(if mod_delta > 0.050 {
                                        theme.error_color()
                                    } else if mod_delta > 0.020 {
                                        theme.warning_color()
                                    } else {
                                        theme.text_color().linear_multiply(0.55)
                                    }),
                            );
                        });

                        ui.add_space(6.0);

                        // cell bars — wrap at 12 per row
                        ui.horizontal_wrapped(|ui| {
                            for (ci, cell) in module.iter().enumerate() {
                                Self::cell_bar(ui, &theme, ci, cell, stale);
                            }
                        });
                    });

                ui.add_space(6.0);
            }
        });

        egui_tiles::UiResponse::None
    }

    // ─── helpers ──────────────────────────────────────────────────────────────

    fn stat_card(
        ui: &mut egui::Ui,
        theme: &ui::theme::ThemeColors,
        label: &str,
        value: f32,
        unit: &str,
        stale: bool,
        override_color: Option<Color32>,
    ) {
        Frame::NONE
            .fill(theme.panel_color())
            .stroke(Stroke::new(1.0, theme.accent_color()))
            .inner_margin(egui::Margin::same(10))
            .corner_radius(egui::CornerRadius::same(4))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(label)
                            .size(10.0)
                            .color(theme.text_color().linear_multiply(0.5)),
                    );
                    ui.add_space(2.0);
                    let val_color = override_color.unwrap_or(theme.info_color());
                    if stale {
                        ui.label(
                            RichText::new("—")
                                .size(20.0)
                                .color(theme.text_color().linear_multiply(0.25)),
                        );
                    } else {
                        ui.label(
                            RichText::new(format!("{:.3}", value))
                                .size(20.0)
                                .color(val_color),
                        );
                        ui.label(
                            RichText::new(unit)
                                .size(10.0)
                                .color(theme.text_color().linear_multiply(0.4)),
                        );
                    }
                });
            });
    }

    fn cell_bar(
        ui: &mut egui::Ui,
        theme: &ui::theme::ThemeColors,
        cell_idx: usize,
        cell: &CellData,
        stale: bool,
    ) {
        const V_MIN: f32 = 3.2;
        const V_MAX: f32 = 4.2;
        const BAR_W: f32 = 28.0;
        const BAR_H: f32 = 52.0;

        let fill_color = if stale {
            theme.text_color().linear_multiply(0.12)
        } else {
            Self::voltage_color(cell.voltage)
        };

        let fill_frac = ((cell.voltage - V_MIN) / (V_MAX - V_MIN)).clamp(0.0, 1.0);

        ui.vertical(|ui| {
            ui.set_max_width(BAR_W + 4.0);

            let (outer_rect, _) =
                ui.allocate_exact_size(egui::Vec2::new(BAR_W, BAR_H), egui::Sense::hover());

            let painter = ui.painter();

            // background track
            painter.rect_filled(outer_rect, 3.0, theme.text_color().linear_multiply(0.06));
            painter.rect_stroke(
                outer_rect,
                3.0,
                Stroke::new(0.5, theme.accent_color()),
                egui::StrokeKind::Inside,
            );

            // filled bar (bottom-anchored)
            let fill_h = outer_rect.height() * fill_frac;
            let fill_rect = egui::Rect::from_min_max(
                egui::pos2(outer_rect.min.x, outer_rect.max.y - fill_h),
                outer_rect.max,
            );
            painter.rect_filled(fill_rect, 2.0, fill_color);

            // blue balancing indicator dot at top of bar
            if cell.balancing && !stale {
                let dot_center = egui::pos2(outer_rect.center().x, outer_rect.min.y + 6.0);
                painter.circle_filled(dot_center, 3.5, Color32::from_rgb(77, 166, 255));
                painter.circle_stroke(
                    dot_center,
                    3.5,
                    Stroke::new(1.0, Color32::from_rgb(120, 200, 255)),
                );
            }

            // voltage label below bar
            ui.label(
                RichText::new(if stale {
                    "—".to_string()
                } else {
                    format!("{:.2}", cell.voltage)
                })
                .size(9.0)
                .color(
                    theme
                        .text_color()
                        .linear_multiply(if stale { 0.25 } else { 0.65 }),
                ),
            );

            // cell index
            ui.label(
                RichText::new(format!("C{cell_idx}"))
                    .size(9.0)
                    .color(theme.text_color().linear_multiply(0.35)),
            );
        });
    }

    fn voltage_color(v: f32) -> Color32 {
        if v < 3.50 {
            Color32::from_rgb(217, 83, 79) // red — low
        } else if v < 3.70 {
            Color32::from_rgb(232, 160, 42) // amber — warning
        } else if v <= 4.10 {
            Color32::from_rgb(76, 175, 80) // green — good
        } else {
            Color32::from_rgb(33, 150, 243) // blue — high
        }
    }
}
