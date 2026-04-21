use crate::{messages, ui, util};
use eframe::egui::{self, Color32, Frame, RichText, Stroke};

const NUM_MODULES: usize = 8;
const CELLS_PER_MODULE: usize = 16;
const V_MIN: f64 = 2.7;
const V_MAX: f64 = 4.2;
const V_NOM: f64 = 3.7;
const STALE_TIMEOUT_SECONDS: u64 = 1;

// ─── Data model ────────────────────────────────────────────────────────────────

#[derive(Default, Clone)]
pub struct CellData {
    pub voltage: f64,
    pub balancing: bool,
}
impl CellData {
    pub fn color(&self) -> Color32 {
        // Blue override for discharge
        if self.balancing {
            return Color32::from_rgb(33, 150, 243);
        }

        // Clamp voltage range
        let v = self.voltage.clamp(V_MIN, V_MAX);

        // Piecewise interpolate hue
        let hue = if v <= V_NOM {
            // V_MIN → V_NOM maps 0° → 45°
            let t = (v - V_MIN) / (V_NOM - V_MIN);
            util::lerp(0.0, 45.0, t)
        } else {
            // V_NOM → V_MAX maps 45° → 120°
            let t = (v - V_NOM) / (V_MAX - V_NOM);
            util::lerp(45.0, 120.0, t)
        };

        util::hsv_to_color32(hue, 1.0, 1.0)
    }
}

pub struct BatteryViewer {
    pub title: String,

    pub modules: Vec<Vec<CellData>>, // [module_idx][cell_idx]
    pub current: f64,
    pub pack_sum: f64,
    pub cell_min: f64,
    pub cell_max: f64,

    pub last_update: std::time::Instant,
    pub is_data_stale: bool,
}

impl BatteryViewer {
    pub fn new(instance_num: usize) -> Self {
        Self {
            title: format!("Battery Viewer #{}", instance_num),
            modules: vec![vec![CellData::default(); CELLS_PER_MODULE]; NUM_MODULES],
            current: -1.0,
            pack_sum: -1.0,
            cell_min: -1.0,
            cell_max: -1.0,
            last_update: std::time::Instant::now() - std::time::Duration::from_secs(10),
            is_data_stale: true,
        }
    }

    // ─── CAN handler ──────────────────────────────────────────────────────────

    pub fn handle_can_message(&mut self, msg: &messages::MsgFromCan) {
        if let messages::MsgFromCan::ParsedMessage(parsed) = msg {
            match parsed.decoded.name.as_str() {
                "cell_telemetry" => {
                    let mut module_num: Option<usize> = None;
                    let mut cell_num: Option<usize> = None;
                    let mut voltage: Option<f64> = None;
                    let mut balancing: Option<bool> = None;

                    for (_, sig) in parsed.decoded.signals.iter() {
                        match sig.name.as_str() {
                            "module_num" => {
                                if let can_decode::DecodedSignalValue::Numeric(v) = &sig.value {
                                    module_num = Some(v.round() as usize);
                                }
                            }
                            "cell_num" => {
                                if let can_decode::DecodedSignalValue::Numeric(v) = &sig.value {
                                    cell_num = Some(v.round() as usize);
                                }
                            }
                            "cell_voltage" => {
                                if let can_decode::DecodedSignalValue::Numeric(v) = &sig.value {
                                    voltage = Some(*v);
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
                        && m < self.modules.len()
                        && c < self.modules[m].len()
                    {
                        self.modules[m][c].voltage = v;
                        self.modules[m][c].balancing = b;
                    }

                    self.last_update = std::time::Instant::now();
                    self.is_data_stale = false;
                }
                "charging_telemetry" => {
                    for (_, sig) in parsed.decoded.signals.iter() {
                        match sig.name.as_str() {
                            "pack_voltage" => {
                                if let can_decode::DecodedSignalValue::Numeric(v) = &sig.value {
                                    self.pack_sum = *v;
                                }
                            }
                            "pack_current" => {
                                if let can_decode::DecodedSignalValue::Numeric(v) = &sig.value {
                                    self.current = *v;
                                }
                            }
                            "min_cell_voltage" => {
                                if let can_decode::DecodedSignalValue::Numeric(v) = &sig.value {
                                    self.cell_min = *v;
                                }
                            }
                            "max_cell_voltage" => {
                                if let can_decode::DecodedSignalValue::Numeric(v) = &sig.value {
                                    self.cell_max = *v;
                                }
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
    }

    // ─── UI ───────────────────────────────────────────────────────────────────

    pub fn show(&mut self, ui: &mut egui::Ui) -> egui_tiles::UiResponse {
        let theme = ui::theme::get_theme(ui.ctx());

        self.is_data_stale =
            self.last_update.elapsed() > std::time::Duration::from_secs(STALE_TIMEOUT_SECONDS);
        let stale = self.is_data_stale;
        let elapsed = self.last_update.elapsed().as_secs_f64();

        // ── collect pack-level stats ─────────────────────────────────────────
        let pack_sum = self.pack_sum;
        let pack_min = self.cell_min;
        let pack_max = self.cell_max;
        let pack_delta = if pack_min < f64::MAX {
            pack_max - pack_min
        } else {
            -1.0
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

            ui.columns(5, |cols| {
                Self::stat_card(&mut cols[0], &theme, "PACK SUM", pack_sum, "V", stale, None);
                Self::stat_card(
                    &mut cols[1],
                    &theme,
                    "CURRENT",
                    self.current,
                    "A",
                    stale,
                    None,
                );
                Self::stat_card(
                    &mut cols[2],
                    &theme,
                    "CELL MIN",
                    if pack_min < f64::MAX { pack_min } else { 0.0 },
                    "V",
                    stale,
                    None,
                );
                Self::stat_card(
                    &mut cols[3],
                    &theme,
                    "CELL MAX",
                    if pack_max > f64::MIN { pack_max } else { 0.0 },
                    "V",
                    stale,
                    None,
                );
                Self::stat_card(
                    &mut cols[4],
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
                let mvs: Vec<f64> = module.iter().map(|c| c.voltage).collect();
                let mod_sum: f64 = mvs.iter().sum();
                let mod_min = mvs.iter().cloned().fold(f64::MAX, f64::min);
                let mod_max = mvs.iter().cloned().fold(f64::MIN, f64::max);
                let mod_delta = if mod_min < f64::MAX {
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

                        // ── right: dynamic cell grid ──────────────────────
                        let available = ui.available_width();
                        let cell_spacing = ui.spacing().item_spacing.x; // actual egui spacing, ~4px
                        let cells = CELLS_PER_MODULE as f32;
                        let bar_w = ((available - cell_spacing * cells) / cells).max(8.0);

                        ui.horizontal(|ui| {
                            for cell in module.iter() {
                                Self::cell_bar(ui, &theme, cell, stale, bar_w);
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
        value: f64,
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
                            RichText::new(format!("{:.2}", value))
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

    // fn cell_bar(
    //     ui: &mut egui::Ui,
    //     theme: &ui::theme::ThemeColors,
    //     cell_idx: usize,
    //     cell: &CellData,
    //     stale: bool,
    //     bar_w: f32,
    // ) {
    //     let fill_color = if stale {
    //         theme.text_color().linear_multiply(0.12)
    //     } else {
    //         cell.color()
    //     };

    //     let fill_frac = ((cell.voltage - V_MIN) / (V_MAX - V_MIN)).clamp(0.0, 1.0) as f32;

    //     ui.vertical(|ui| {
    //         ui.set_max_width(bar_w + 4.0);

    //         let (outer_rect, _) =
    //             ui.allocate_exact_size(egui::Vec2::new(bar_w, 20.0), egui::Sense::hover());

    //         let painter = ui.painter();

    //         // background track
    //         painter.rect_filled(outer_rect, 3.0, theme.text_color().linear_multiply(0.06));
    //         painter.rect_stroke(
    //             outer_rect,
    //             3.0,
    //             Stroke::new(0.5, theme.accent_color()),
    //             egui::StrokeKind::Inside,
    //         );

    //         // filled bar (bottom-anchored)
    //         let fill_h = outer_rect.height() * fill_frac;
    //         let fill_rect = egui::Rect::from_min_max(
    //             egui::pos2(outer_rect.min.x, outer_rect.max.y - fill_h),
    //             outer_rect.max,
    //         );
    //         painter.rect_filled(fill_rect, 2.0, fill_color);

    //         ui.horizontal_wrapped(|ui| {
    //             // voltage label below bar
    //             ui.label(
    //                 RichText::new(if stale {
    //                     "—".to_string()
    //                 } else {
    //                     format!("{:.2}", cell.voltage)
    //                 })
    //                 .size(9.0)
    //                 .color(
    //                     theme
    //                         .text_color()
    //                         .linear_multiply(if stale { 0.25 } else { 0.65 }),
    //                 ),
    //             );

    //             // cell index
    //             ui.label(
    //                 RichText::new(format!("C{cell_idx}"))
    //                     .italics()
    //                     .size(9.0)
    //                     .color(theme.text_color().linear_multiply(0.35)),
    //             );
    //         });
    //     });
    // }
    fn cell_bar(
        ui: &mut egui::Ui,
        theme: &ui::theme::ThemeColors,
        cell: &CellData,
        stale: bool,
        bar_w: f32,
    ) {
        use egui::{Align2, FontId, Stroke};

        let fill_color = if stale {
            theme.text_color().linear_multiply(0.12)
        } else {
            cell.color()
        };

        let fill_frac = ((cell.voltage - V_MIN) / (V_MAX - V_MIN)).clamp(0.0, 1.0) as f32;

        ui.vertical(|ui| {
            ui.set_max_width(bar_w + 4.0);

            let (outer_rect, _) =
                ui.allocate_exact_size(egui::Vec2::new(bar_w, 20.0), egui::Sense::hover());

            let painter = ui.painter();

            // --- Background track ---
            painter.rect_filled(outer_rect, 3.0, theme.text_color().linear_multiply(0.06));

            painter.rect_stroke(
                outer_rect,
                3.0,
                Stroke::new(0.5, theme.accent_color()),
                egui::StrokeKind::Inside,
            );

            // --- Filled portion (bottom-anchored) ---
            let fill_h = outer_rect.height() * fill_frac;

            let fill_rect = egui::Rect::from_min_max(
                egui::pos2(outer_rect.min.x, outer_rect.max.y - fill_h),
                outer_rect.max,
            );

            painter.rect_filled(fill_rect, 2.0, fill_color);

            // --- Voltage text (inside bar) ---
            let text = if stale {
                "—".to_string()
            } else {
                format!("{:.2}", cell.voltage)
            };

            // Contrast-aware text color
            let text_color = if stale {
                theme.text_color().linear_multiply(0.25)
            } else if fill_frac > 0.5 {
                egui::Color32::BLACK
            } else {
                egui::Color32::WHITE
            };

            painter.text(
                outer_rect.center(),
                Align2::CENTER_CENTER,
                text,
                FontId::proportional(10.0),
                text_color,
            );
        });
    }
}
