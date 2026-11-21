//! Visual identity: a dark graphite canvas, white ink, quiet chrome.

use egui::{Color32, CornerRadius, Stroke, ThemePreference, Visuals};

pub const CANVAS_BG: Color32 = Color32::from_rgb(0x14, 0x16, 0x1a);
pub const GRID_DOT: Color32 = Color32::from_rgb(0x25, 0x28, 0x2f);
pub const MARKER: Color32 = Color32::WHITE;

pub const TOOLBAR_BG: Color32 = Color32::from_rgb(0x1c, 0x1f, 0x25);
pub const TOOLBAR_BORDER: Color32 = Color32::from_rgb(0x2c, 0x31, 0x3a);

pub const TEXT: Color32 = Color32::from_rgb(0xe7, 0xea, 0xf0);
pub const TEXT_DIM: Color32 = Color32::from_rgb(0x8b, 0x93, 0x9f);

pub const WIDGET_BG: Color32 = Color32::from_rgb(0x26, 0x2a, 0x32);
pub const WIDGET_HOVER: Color32 = Color32::from_rgb(0x32, 0x37, 0x41);
pub const WIDGET_ACTIVE: Color32 = Color32::from_rgb(0x3d, 0x43, 0x4f);
pub const ACCENT: Color32 = Color32::from_rgb(0xf2, 0xf4, 0xf8);

pub const NOTICE_BG: Color32 = Color32::from_rgb(0x2a, 0x21, 0x23);
pub const NOTICE_BORDER: Color32 = Color32::from_rgb(0x5c, 0x3c, 0x40);

pub fn apply(ctx: &egui::Context) {
    ctx.set_theme(ThemePreference::Dark);

    let mut visuals = Visuals::dark();
    visuals.override_text_color = Some(TEXT);
    visuals.panel_fill = CANVAS_BG;
    visuals.window_fill = TOOLBAR_BG;
    visuals.window_stroke = Stroke::new(1.0, TOOLBAR_BORDER);
    visuals.extreme_bg_color = CANVAS_BG;
    visuals.hyperlink_color = ACCENT;
    visuals.selection.bg_fill = Color32::from_rgb(0x3a, 0x41, 0x4d);
    visuals.selection.stroke = Stroke::new(1.0, ACCENT);

    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, TOOLBAR_BORDER);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT);

    visuals.widgets.inactive.weak_bg_fill = WIDGET_BG;
    visuals.widgets.inactive.bg_fill = WIDGET_BG;
    visuals.widgets.inactive.bg_stroke = Stroke::NONE;
    visuals.widgets.inactive.corner_radius = CornerRadius::same(7);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT);
    visuals.widgets.inactive.expansion = 0.0;

    visuals.widgets.hovered.weak_bg_fill = WIDGET_HOVER;
    visuals.widgets.hovered.bg_fill = WIDGET_HOVER;
    visuals.widgets.hovered.bg_stroke = Stroke::NONE;
    visuals.widgets.hovered.corner_radius = CornerRadius::same(7);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, ACCENT);
    visuals.widgets.hovered.expansion = 0.0;

    visuals.widgets.active.weak_bg_fill = WIDGET_ACTIVE;
    visuals.widgets.active.bg_fill = WIDGET_ACTIVE;
    visuals.widgets.active.bg_stroke = Stroke::NONE;
    visuals.widgets.active.corner_radius = CornerRadius::same(7);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, ACCENT);
    visuals.widgets.active.expansion = 0.0;

    visuals.widgets.open = visuals.widgets.active;

    ctx.set_visuals(visuals);

    ctx.all_styles_mut(|style| {
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.spacing.button_padding = egui::vec2(10.0, 5.0);
        style.spacing.slider_width = 120.0;
        style.spacing.interact_size.y = 26.0;
    });
}
