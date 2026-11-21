//! Floating toolbar: marker size, undo/redo, clear, and a quiet status line.

use egui::{
    Align2, Area, Button, Context, CornerRadius, Frame, Id, Margin, Order, RichText, Slider, Stroke,
};

use crate::theme;

#[derive(Clone, Copy, Debug, Default)]
pub struct ToolbarActions {
    pub undo: bool,
    pub redo: bool,
    pub clear: bool,
}

pub fn toolbar(
    ctx: &Context,
    marker_width: &mut f32,
    stroke_count: usize,
    can_undo: bool,
    can_redo: bool,
    zoom: f32,
    actions: &mut ToolbarActions,
) {
    Area::new(Id::unique("darkboard.toolbar"))
        .order(Order::Foreground)
        .anchor(Align2::CENTER_BOTTOM, egui::vec2(0.0, -18.0))
        .show(ctx, |ui| {
            Frame::NONE
                .fill(theme::TOOLBAR_BG)
                .stroke(Stroke::new(1.0, theme::TOOLBAR_BORDER))
                .corner_radius(CornerRadius::same(12))
                .inner_margin(Margin {
                    left: 12,
                    right: 12,
                    top: 8,
                    bottom: 8,
                })
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.add(
                            Slider::new(marker_width, 1.0..=24.0)
                                .fixed_decimals(0)
                                .text("Marker"),
                        );

                        ui.add_space(4.0);
                        ui.separator();
                        ui.add_space(4.0);

                        let button_size = egui::vec2(58.0, 26.0);
                        actions.undo = ui
                            .add_enabled(can_undo, Button::new("Undo").min_size(button_size))
                            .clicked();
                        actions.redo = ui
                            .add_enabled(can_redo, Button::new("Redo").min_size(button_size))
                            .clicked();
                        actions.clear = ui
                            .add_enabled(
                                stroke_count > 0,
                                Button::new("Clear").min_size(button_size),
                            )
                            .clicked();

                        ui.add_space(6.0);
                        let plural = if stroke_count == 1 { "" } else { "s" };
                        ui.label(
                            RichText::new(format!(
                                "{stroke_count} stroke{plural}  ·  {:.0}%",
                                zoom * 100.0
                            ))
                            .size(12.0)
                            .color(theme::TEXT_DIM),
                        );
                    });
                });
        });
}
