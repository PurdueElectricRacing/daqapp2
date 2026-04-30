use crate::{messages, ui};
use chrono::DateTime;
use eframe::egui;
use std::collections::VecDeque;

/// Safety cap so a mis-set window / flood cannot grow without bound.
const MAX_HISTORY_POINTS: usize = 500_000;
const AXIS_LIMIT_G: f32 = 2.0;
const IMU_ACCEL_MSG_NAME: &str = "IMU_acceleration";
/// DBC is vehicle frame: +X forward, +Y left (g).
const IMU_ACCEL_X_NAME: &str = "X_axis";
const IMU_ACCEL_Y_NAME: &str = "Y_axis";

/// Vehicle (+X forward, +Y left) to `egui_plot` data: horizontal = −Ay, vertical = Ax.
#[inline]
fn vehicle_accel_to_plot_xy(ax_g: f32, ay_g: f32) -> [f64; 2] {
    [-ay_g as f64, ax_g as f64]
}

pub struct GgPlot {
    pub title: String,
    points_g: VecDeque<(DateTime<chrono::Local>, f32, f32)>,
    history_window_minutes: f64,
}

impl GgPlot {
    pub fn new(instance_num: usize) -> Self {
        Self {
            title: format!("G-G Plot #{}", instance_num),
            points_g: VecDeque::new(),
            history_window_minutes: 5.0,
        }
    }

    fn retention_chrono(&self) -> chrono::Duration {
        let window = std::time::Duration::from_secs_f64(self.history_window_minutes * 60.0);
        chrono::Duration::from_std(window).unwrap_or_else(|_| chrono::Duration::zero())
    }

    fn prune_points_to_window(&mut self) {
        if self.points_g.is_empty() {
            return;
        }
        let Some((newest, _, _)) = self.points_g.back().cloned() else {
            return;
        };
        let cutoff = newest - self.retention_chrono();
        while let Some((t, _, _)) = self.points_g.front() {
            if *t < cutoff {
                self.points_g.pop_front();
            } else {
                break;
            }
        }
        while self.points_g.len() > MAX_HISTORY_POINTS {
            self.points_g.pop_front();
        }
    }

    pub fn handle_can_message(&mut self, msg: &messages::MsgFromCan) {
        if let messages::MsgFromCan::ParsedMessage(parsed) = msg {
            if parsed.decoded.name != IMU_ACCEL_MSG_NAME {
                return;
            }

            let mut forward_g = None;
            let mut left_g = None;
            for (_, sig) in &parsed.decoded.signals {
                match sig.name.as_str() {
                    IMU_ACCEL_X_NAME => {
                        if let can_decode::DecodedSignalValue::Numeric(v) = &sig.value {
                            forward_g = Some(*v as f32);
                        }
                    }
                    IMU_ACCEL_Y_NAME => {
                        if let can_decode::DecodedSignalValue::Numeric(v) = &sig.value {
                            left_g = Some(*v as f32);
                        }
                    }
                    _ => {}
                }
            }

            if let (Some(ax), Some(ay)) = (forward_g, left_g) {
                self.points_g
                    .push_back((parsed.timestamp, ax, ay));
                self.prune_points_to_window();
            }
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> egui_tiles::UiResponse {
        let theme = ui::theme::get_theme(ui.ctx());
        self.prune_points_to_window();

        ui.horizontal(|ui| {
            ui.label("Retention:");
            ui.add(
                egui::Slider::new(&mut self.history_window_minutes, 0.5..=20.0).suffix(" min"),
            );
            ui.separator();
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
            .x_axis_label("Longitudinal acceleration")
            .y_axis_label("Lateral acceleration")
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

                let trail: Vec<[f64; 2]> = self
                    .points_g
                    .iter()
                    .map(|(_, ax_g, ay_g)| vehicle_accel_to_plot_xy(*ax_g, *ay_g))
                    .collect();
                if !trail.is_empty() {
                    plot_ui.points(
                        egui_plot::Points::new("trail", egui_plot::PlotPoints::from(trail))
                            .radius(2.0)
                            .color(theme.error_color().linear_multiply(0.45)),
                    );
                }

                if let Some((_, ax_g, ay_g)) = self.points_g.back() {
                    plot_ui.points(
                        egui_plot::Points::new(
                            "current",
                            egui_plot::PlotPoints::from(vec![vehicle_accel_to_plot_xy(
                                *ax_g, *ay_g,
                            )]),
                        )
                        .radius(4.5)
                        .color(theme.error_color()),
                    );
                }
            });

        egui_tiles::UiResponse::None
    }
}
