use crate::{messages, ui, util};
use eframe::egui::{self, Color32, Frame, RichText, Stroke};

const NUM_MODULES: usize = 8;
const THERMISTORS_PER_MODULE: usize = 10;

// Celsius range for coloring
const T_MIN: f64 = 23.0;
const T_MAX: f64 = 60.0;
const T_NOM: f64 = 37.0;

const STALE_TIMEOUT_SECONDS: u64 = 1;

#[derive(Default, Clone)]
pub struct TempData {
    pub temperature: f64,
}
impl TempData {
    pub fn color(&self) -> Color32 {
        let temp = self.temperature.clamp(T_MIN, T_MAX);

        // Piecewise interpolate hue
		let hue = if temp <= T_NOM {
			// T_MIN -> T_NOM maps 120° -> 45°
			let t = (temp - T_MIN) / (T_NOM - T_MIN);
			util::lerp(120.0, 45.0, t)
		} else {
			// T_NOM -> T_MAX maps 45° -> 0°
			let t = (temp - T_NOM) / (T_MAX - T_NOM);
			util::lerp(45.0, 0.0, t)
		};
		
        util::hsv_to_color32(hue, 1.0, 1.0)
    }
}

pub struct BatteryTemps {
    pub title: String,

    modules: Vec<Vec<TempData>>, // [module_idx][cell_idx]

    last_update: std::time::Instant,
    is_data_stale: bool,
}

impl BatteryTemps {
    pub fn new(instance_num: usize) -> Self {
        Self {
            title: format!("Battery Temps #{}", instance_num),
            modules: vec![vec![TempData::default(); THERMISTORS_PER_MODULE]; NUM_MODULES],
            last_update: std::time::Instant::now() - std::time::Duration::from_secs(10),
            is_data_stale: true,
        }
    }

    pub fn handle_can_message(&mut self, msg: &messages::MsgFromCan) {
        if let messages::MsgFromCan::ParsedMessage(parsed) = msg
            && parsed.decoded.name.as_str() == "thermistor_telemetry_ccan" {
                let mut module_num: Option<usize> = None;
                let mut thermistor_num: Option<usize> = None;
                let mut temperature: Option<f64> = None;

                for (_, sig) in parsed.decoded.signals.iter() {
                    match sig.name.as_str() {
                        "module_num" => module_num = Some(sig.value.physical.round() as usize),
                        "thermistor_num" => {
                            thermistor_num = Some(sig.value.physical.round() as usize)
                        }
                        "temperature" => temperature = Some(sig.value.physical),
                        _ => {}
                    }
                }

                if let (Some(m), Some(t), Some(temp)) = (module_num, thermistor_num, temperature)
                    && m < self.modules.len()
                    && t < self.modules[m].len()
                {
                    self.modules[m][t].temperature = temp;
                }

                self.last_update = std::time::Instant::now();
                self.is_data_stale = false;
            }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> egui_tiles::UiResponse {
        let theme = ui::theme::get_theme(ui.ctx());

        self.is_data_stale =
            self.last_update.elapsed() > std::time::Duration::from_secs(STALE_TIMEOUT_SECONDS);
        let stale = self.is_data_stale;
        let elapsed = self.last_update.elapsed().as_secs_f64();

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(4.0);
            ui.heading(&self.title);
            ui.add_space(4.0);

            // Staleness banner
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

            for (mi, module) in self.modules.iter().enumerate() {
                let mod_avg: f64 = module.iter().map(|c| c.temperature).sum::<f64>()
                    / (module.len() as f64);
                let mod_min = module.iter().map(|c| c.temperature).fold(f64::MAX, f64::min);
                let mod_max = module.iter().map(|c| c.temperature).fold(f64::MIN, f64::max);

                Frame::NONE
                    .fill(theme.panel_color())
                    .stroke(Stroke::new(1.0, theme.accent_color()))
                    .inner_margin(egui::Margin::same(10))
                    .corner_radius(egui::CornerRadius::same(4))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(format!("MODULE {mi}")).size(11.0).strong());
                            ui.add_space(8.0);
                            ui.label(
                                RichText::new(format!("avg {:.1} °C", mod_avg))
                                    .size(10.0)
                                    .color(theme.text_color().linear_multiply(0.55)),
                            );
                            ui.label(
                                RichText::new(format!("min {:.1} °C", mod_min))
                                    .size(10.0)
                                    .color(theme.text_color().linear_multiply(0.55)),
                            );
                            ui.label(
                                RichText::new(format!("max {:.1} °C", mod_max))
                                    .size(10.0)
                                    .color(theme.text_color().linear_multiply(0.55)),
                            );
                        });

                        ui.add_space(6.0);

                        let available = ui.available_width();
                        let cell_spacing = ui.spacing().item_spacing.x;
                        let cells = THERMISTORS_PER_MODULE as f32;
                        let bar_w = ((available - cell_spacing * cells) / cells).max(8.0);

                        ui.horizontal(|ui| {
                            for cell in module.iter() {
                                Self::temp_bar(ui, &theme, cell, stale, bar_w);
                            }
                        });
                    });

                ui.add_space(6.0);
            }
        });

        egui_tiles::UiResponse::None
    }

    fn temp_bar(
        ui: &mut egui::Ui,
        theme: &ui::theme::ThemeColors,
        cell: &TempData,
        stale: bool,
        bar_w: f32,
    ) {
        use egui::{Align2, FontId};

        let fill_color = if stale {
            theme.text_color().linear_multiply(0.12)
        } else {
            cell.color()
        };

        // fill fraction based on temperature range
        let fill_frac = ((cell.temperature - T_MIN) / (T_MAX - T_MIN)).clamp(0.0, 1.0) as f32;

        ui.vertical(|ui| {
            ui.set_max_width(bar_w + 4.0);

            let (outer_rect, _) =
                ui.allocate_exact_size(egui::Vec2::new(bar_w, 24.0), egui::Sense::hover());

            let painter = ui.painter();

            painter.rect_filled(outer_rect, 3.0, theme.text_color().linear_multiply(0.06));

            painter.rect_stroke(
                outer_rect,
                3.0,
                Stroke::new(0.5, theme.accent_color()),
                egui::StrokeKind::Inside,
            );

            let fill_h = outer_rect.height() * fill_frac;

            let fill_rect = egui::Rect::from_min_max(
                egui::pos2(outer_rect.min.x, outer_rect.max.y - fill_h),
                outer_rect.max,
            );

            painter.rect_filled(fill_rect, 2.0, fill_color);

            let text = if stale {
                "—".to_string()
            } else {
                format!("{:.1}°C", cell.temperature)
            };

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
                FontId::proportional(11.0),
                text_color,
            );
        });
    }
}
