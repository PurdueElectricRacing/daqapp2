use crate::{messages, ui};
use eframe::egui::{self, Color32, Frame, Stroke, Vec2, Pos2, StrokeKind};
use std::time::Instant;

const STALE_TIMEOUT_SECONDS: u64 = 1;
const WHEELBASE_M: f32 = 1.530; // Standard Formula Student wheelbase
const CHASSIS_WIDTH_M: f32 = 1.4;
const CHASSIS_LENGTH_M: f32 = 2.5;

pub struct Dynamics {
    pub title: String,

    // Motor/Vehicle Speed
    pub velocity_mps: f32,

    // IMU Data
    pub accel_x: f32, // Longitudinal (X+ = Forward)
    pub accel_y: f32, // Lateral (Y+ = Left)

    // Steering
    pub steer_angle_rad: f32,
    pub yaw_rate_rads: f32,

    last_update: Instant,
    pub is_data_stale: bool,
}

impl Dynamics {
    pub fn new(instance_num: usize) -> Self {
        Self {
            title: format!("Dynamics #{}", instance_num),
            velocity_mps: 0.0,
            accel_x: 0.0,
            accel_y: 0.0,
            steer_angle_rad: 0.0,
            yaw_rate_rads: 0.0,
            last_update: Instant::now() - std::time::Duration::from_secs(10),
            is_data_stale: true,
        }
    }

    pub fn handle_can_message(&mut self, msg: &messages::MsgFromCan) {
        if let messages::MsgFromCan::ParsedMessage(parsed) = msg {
            match parsed.decoded.name.as_str() {
                "IMU_acceleration" => {
                    for (_, sig) in parsed.decoded.signals.iter() {
                        match sig.name.as_str() {
                            "X_axis" => {
                                if let can_decode::DecodedSignalValue::Numeric(v) = &sig.value {
                                    self.accel_x = *v as f32;
                                }
                            }
                            "Y_axis" => {
                                if let can_decode::DecodedSignalValue::Numeric(v) = &sig.value {
                                    self.accel_y = *v as f32;
                                }
                            }
                            _ => {}
                        }
                    }
                    self.last_update = Instant::now();
                    self.is_data_stale = false;
                }
                "IMU_angular_rate" => {
                    for (_, sig) in parsed.decoded.signals.iter() {
                        match sig.name.as_str() {
                            "Z_axis" => {
                                if let can_decode::DecodedSignalValue::Numeric(v) = &sig.value {
                                    self.yaw_rate_rads = v.to_radians() as f32; // DBC is in deg/s
                                }
                            }
                            _ => {}
                        }
                    }
                    self.last_update = Instant::now();
                    self.is_data_stale = false;
                }
                "steering_angle" => {
                    for (_, sig) in parsed.decoded.signals.iter() {
                        if sig.name == "angle" {
                            if let can_decode::DecodedSignalValue::Numeric(v) = &sig.value {
                                self.steer_angle_rad = v.to_radians() as f32;
                            }
                        }
                    }
                    self.last_update = Instant::now();
                    self.is_data_stale = false;
                }
                // TODO: Implement velocity tracking (GPS velocity or Wheel speed)
                _ => {}
            }
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> egui_tiles::UiResponse {
        let theme = ui::theme::get_theme(ui.ctx());

        self.is_data_stale =
            self.last_update.elapsed() > std::time::Duration::from_secs(STALE_TIMEOUT_SECONDS);

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(4.0);
            ui.heading(&self.title);
            ui.add_space(4.0);

            self.draw_status_banner(ui, &theme);

            ui.add_space(8.0);

            // Allocation for custom visualization
            let available_w = ui.available_width();
            let size = Vec2::splat(available_w.min(400.0));
            let (rect, _response) = ui.allocate_exact_size(size, egui::Sense::hover());

            let painter = ui.painter();
            painter.rect_filled(rect, 4.0, theme.panel_color());
            painter.rect_stroke(
                rect,
                4.0,
                Stroke::new(1.0, theme.accent_color()),
                egui::StrokeKind::Inside,
            );

            // --- 2D Drawing Logic ---
            let center = rect.center();
            let pixels_per_meter = rect.width() / 6.0; // 6 meters total width

            self.draw_chassis(painter, center, pixels_per_meter, &theme);

            if !self.is_data_stale {
                self.draw_dynamics(painter, center, pixels_per_meter, &theme);
            }
        });

        egui_tiles::UiResponse::None
    }

    fn draw_status_banner(&self, ui: &mut egui::Ui, theme: &ui::theme::ThemeColors) {
        let stale = self.is_data_stale;
        let elapsed = self.last_update.elapsed().as_secs_f64();
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
                    let (rect, _) =
                        ui.allocate_exact_size(egui::Vec2::splat(8.0), egui::Sense::hover());
                    ui.painter().circle_filled(rect.center(), 4.0, dot);
                    ui.add_space(4.0);
                    ui.colored_label(dot, &text);
                });
            });
    }

    fn draw_chassis(
        &self,
        painter: &egui::Painter,
        center: Pos2,
        pixels_per_meter: f32,
        theme: &ui::theme::ThemeColors,
    ) {
        // 1. Draw Chassis (Top-down)
        let chassis_w = CHASSIS_WIDTH_M * pixels_per_meter;
        let chassis_h = CHASSIS_LENGTH_M * pixels_per_meter;
        let chassis_rect = egui::Rect::from_center_size(center, Vec2::new(chassis_w, chassis_h));

        painter.rect_stroke(
            chassis_rect,
            2.0,
            Stroke::new(2.0, theme.text_color().linear_multiply(0.3)),
            StrokeKind::Outside,
        );

        // 2. Draw Wheelbase / Axles
        let axle_dist_px = WHEELBASE_M * pixels_per_meter;
        let front_axle_y = center.y - axle_dist_px / 2.0;
        let rear_axle_y = center.y + axle_dist_px / 2.0;

        painter.line_segment(
            [
                Pos2::new(center.x - chassis_w / 2.0, front_axle_y),
                Pos2::new(center.x + chassis_w / 2.0, front_axle_y),
            ],
            Stroke::new(1.0, theme.text_color().linear_multiply(0.2)),
        );
        painter.line_segment(
            [
                Pos2::new(center.x - chassis_w / 2.0, rear_axle_y),
                Pos2::new(center.x + chassis_w / 2.0, rear_axle_y),
            ],
            Stroke::new(1.0, theme.text_color().linear_multiply(0.2)),
        );
    }

    fn draw_dynamics(
        &self,
        painter: &egui::Painter,
        center: Pos2,
        pixels_per_meter: f32,
        theme: &ui::theme::ThemeColors,
    ) {
        let axle_dist_px = WHEELBASE_M * pixels_per_meter;
        let front_axle_y = center.y - axle_dist_px / 2.0;
        let rear_axle_y = center.y + axle_dist_px / 2.0;

        // 3. Acceleration Vector (Red)
        let accel_scale = 20.0;
        let accel_vec = Vec2::new(-self.accel_y * accel_scale, -self.accel_x * accel_scale);
        painter.line_segment([center, center + accel_vec], Stroke::new(3.0, theme.error_color()));
        painter.circle_filled(center + accel_vec, 4.0, theme.error_color());

        // 4. Velocity Vector (Green)
        let speed_scale = 2.0;
        let vel_vec = Vec2::new(0.0, -self.velocity_mps * speed_scale);
        painter.line_segment(
            [
                Pos2::new(center.x, front_axle_y),
                Pos2::new(center.x, front_axle_y) + vel_vec,
            ],
            Stroke::new(3.0, theme.success_color()),
        );

        // 5. Turn Radius Projection
        if self.steer_angle_rad.abs() > 0.01 {
            let r_m = WHEELBASE_M / self.steer_angle_rad.tan();
            let r_px = r_m * pixels_per_meter;
            let turn_center = Pos2::new(center.x - r_px, rear_axle_y);

            painter.circle_stroke(
                turn_center,
                r_px.abs(),
                Stroke::new(1.0, theme.info_color().linear_multiply(0.3)),
            );
        }
    }
}


