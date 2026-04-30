use crate::{messages, ui};
use chrono::DateTime;
use eframe::egui::{self, Color32, Frame, Stroke};
use std::collections::VecDeque;
use std::time::{Duration, Instant};

const STALE_TIMEOUT_SECONDS: u64 = 1;
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
    /// IMU sample time and vehicle-frame acceleration (g).
    points_g: VecDeque<(DateTime<chrono::Local>, f32, f32)>,
    /// How long to keep samples (matches Scope-style time window, in minutes).
    history_window_minutes: f64,
    last_update: Instant,
    is_data_stale: bool,
}

impl GgPlot {
    pub fn new(instance_num: usize) -> Self {
        Self {
            title: format!("G-G Plot #{}", instance_num),
            points_g: VecDeque::new(),
            history_window_minutes: 5.0,
            last_update: Instant::now() - Duration::from_secs(10),
            is_data_stale: true,
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

                self.last_update = Instant::now();
                self.is_data_stale = false;
            }
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> egui_tiles::UiResponse {
        let theme = ui::theme::get_theme(ui.ctx());
        self.is_data_stale = self.last_update.elapsed() > Duration::from_secs(STALE_TIMEOUT_SECONDS);
        self.prune_points_to_window();

        self.draw_status_banner(ui, &theme);
        ui.add_space(6.0);
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
            // Vehicle: +X forward (Ax), +Y left (Ay). egui_plot: plot x = −Ay, plot y = Ax.
            .x_axis_label("plot x = −Ay (g)")
            .y_axis_label("plot y = Ax (g)")
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
