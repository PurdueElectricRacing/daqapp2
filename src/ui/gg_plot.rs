use crate::{messages, ui};
use eframe::egui::{self, Color32, Frame, Stroke};
use std::collections::VecDeque;
use std::time::{Duration, Instant};

const STALE_TIMEOUT_SECONDS: u64 = 1;
const MAX_HISTORY_POINTS: usize = 350;
const AXIS_LIMIT_G: f32 = 2.0;
const IMU_ACCEL_MSG_NAME: &str = "IMU_acceleration";
/// CAN signal named X — in our DBC this axis is **lateral** (+ left).
const IMU_ACCEL_CAN_X_NAME: &str = "X_axis";
/// CAN signal named Y — in our DBC this axis is **longitudinal** (+ forward).
const IMU_ACCEL_CAN_Y_NAME: &str = "Y_axis";

pub struct GgPlot {
    pub title: String,
    accel_x_g: f32,
    accel_y_g: f32,
    points_g: VecDeque<(f32, f32)>, // (forward X+, left Y+) in g
    last_update: Instant,
    is_data_stale: bool,
}

impl GgPlot {
    pub fn new(instance_num: usize) -> Self {
        Self {
            title: format!("G-G Plot #{}", instance_num),
            accel_x_g: 0.0,
            accel_y_g: 0.0,
            points_g: VecDeque::with_capacity(MAX_HISTORY_POINTS),
            last_update: Instant::now() - Duration::from_secs(10),
            is_data_stale: true,
        }
    }

    pub fn handle_can_message(&mut self, msg: &messages::MsgFromCan) {
        if let messages::MsgFromCan::ParsedMessage(parsed) = msg {
            if parsed.decoded.name != IMU_ACCEL_MSG_NAME {
                return;
            }

            let mut can_x = None;
            let mut can_y = None;
            for (_, sig) in &parsed.decoded.signals {
                match sig.name.as_str() {
                    IMU_ACCEL_CAN_X_NAME => {
                        if let can_decode::DecodedSignalValue::Numeric(v) = &sig.value {
                            can_x = Some(*v as f32);
                        }
                    }
                    IMU_ACCEL_CAN_Y_NAME => {
                        if let can_decode::DecodedSignalValue::Numeric(v) = &sig.value {
                            can_y = Some(*v as f32);
                        }
                    }
                    _ => {}
                }
            }

            // Vehicle frame: forward (+ longitudinal plot axis), left (+ lateral plot axis).
            // Map CAN naming → body: X_axis ↔ lateral, Y_axis ↔ longitudinal.
            if let (Some(can_x), Some(can_y)) = (can_x, can_y) {
                let forward_g = can_y;
                let left_g = can_x;
                self.accel_x_g = forward_g;
                self.accel_y_g = left_g;
                self.points_g.push_back((forward_g, left_g));
                while self.points_g.len() > MAX_HISTORY_POINTS {
                    self.points_g.pop_front();
                }

                self.last_update = Instant::now();
                self.is_data_stale = false;
            }
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> egui_tiles::UiResponse {
        let theme = ui::theme::get_theme(ui.ctx());
        self.is_data_stale = self.last_update.elapsed() > Duration::from_secs(STALE_TIMEOUT_SECONDS);

        self.draw_status_banner(ui, &theme);
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if ui.button("🗑 Clear").clicked() {
                self.points_g.clear();
            }
        });
        ui.add_space(4.0);

        let axis_limit = AXIS_LIMIT_G as f64;
        egui_plot::Plot::new(format!("gg_plot_{}", self.title))
            .view_aspect(1.0)
            .data_aspect(1.0)
            .allow_axis_zoom_drag(false)
            .allow_scroll(false)
            .include_x(-axis_limit)
            .include_x(axis_limit)
            .include_y(-axis_limit)
            .include_y(axis_limit)
            .show_axes([true, true])
            .x_axis_label("Longitudinal (g), forward +")
            .y_axis_label("Lateral (g), left +")
            .show(ui, |plot_ui| {
                for radius in [0.5, 1.0, 1.5] {
                    let mut points = Vec::with_capacity(65);
                    for i in 0..=64 {
                        let theta = (i as f64 / 64.0) * std::f64::consts::TAU;
                        points.push([radius * theta.cos(), radius * theta.sin()]);
                    }
                    plot_ui.line(
                        egui_plot::Line::new(
                            format!("{radius:.1}g"),
                            egui_plot::PlotPoints::from(points),
                        )
                        .color(theme.text_color().linear_multiply(0.18)),
                    );
                }

                // Plot X = forward (X_axis), plot Y = left (Y_axis)
                let trail: Vec<[f64; 2]> = self
                    .points_g
                    .iter()
                    .map(|(ax_g, ay_g)| [*ax_g as f64, *ay_g as f64])
                    .collect();
                if !trail.is_empty() {
                    plot_ui.points(
                        egui_plot::Points::new("trail", egui_plot::PlotPoints::from(trail))
                            .radius(2.0)
                            .color(theme.error_color().linear_multiply(0.45)),
                    );
                }

                if let Some((ax_g, ay_g)) = self.points_g.back() {
                    plot_ui.points(
                        egui_plot::Points::new(
                            "current",
                            egui_plot::PlotPoints::from(vec![[*ax_g as f64, *ay_g as f64]]),
                        )
                        .radius(4.5)
                        .color(theme.error_color()),
                    );
                }
            });

        ui.add_space(6.0);
        let mag_g = (self.accel_x_g.powi(2) + self.accel_y_g.powi(2)).sqrt();
        ui.label(format!(
            "Ax: {:+.2} g | Ay: {:+.2} g",
            self.accel_x_g, self.accel_y_g
        ));
        ui.label(format!("|a|: {:.2} g", mag_g));

        egui_tiles::UiResponse::None
    }

    fn draw_status_banner(&self, ui: &mut egui::Ui, theme: &ui::theme::ThemeColors) {
        let elapsed = self.last_update.elapsed().as_secs_f64();
        let (bg, dot, text) = if self.is_data_stale {
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
                    let (rect, _) =
                        ui.allocate_exact_size(egui::Vec2::splat(8.0), egui::Sense::hover());
                    ui.painter().circle_filled(rect.center(), 4.0, dot);
                    ui.add_space(4.0);
                    ui.colored_label(dot, text);
                });
            });
    }
}
