pub mod i18n;

use eframe::egui;

pub const ACTION_BUTTON: [f32; 2] = [190.0, 40.0];

pub fn sized_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    ui.add_sized(ACTION_BUTTON, egui::Button::new(label))
}
