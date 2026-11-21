//! Per-frame pointer and wheel input, read from egui's raw event stream.
//!
//! Raw events are used instead of widget drag state so that every pointer
//! sample is captured (important for handwriting) and so the board can keep
//! receiving the stroke while the pointer is outside the window.

use egui::{Context, Event, MouseWheelUnit, PointerButton, Pos2, Vec2};

/// Zoom per scroll point. One wheel notch (~40 points) is about 9%.
const WHEEL_ZOOM_SPEED: f32 = 0.0022;
/// egui's default conversion for line-unit wheel events.
const POINTS_PER_LINE: f32 = 40.0;
const POINTS_PER_PAGE: f32 = 400.0;

#[derive(Clone, Copy, Debug)]
pub struct FrameInput {
    pub pointer: Option<Pos2>,
    pub primary_pressed: bool,
    pub primary_down: bool,
    pub middle_down: bool,
    pub space_down: bool,
    pub pan_delta: Vec2,
    /// Multiplicative zoom for this frame (1.0 = no change).
    pub zoom_factor: f32,
    pub lost_focus: bool,
    pub over_canvas: bool,
}

pub fn read(ctx: &Context, over_canvas: bool) -> FrameInput {
    let mut wheel_points = 0.0_f32;
    let mut pinch_factor = 1.0_f32;
    let mut lost_focus = false;

    let (pointer, primary_pressed, primary_down, middle_down, space_down, pan_delta) =
        ctx.input(|input| {
            for event in &input.events {
                match event {
                    Event::MouseWheel { unit, delta, .. } => {
                        let scale = match unit {
                            MouseWheelUnit::Line => POINTS_PER_LINE,
                            MouseWheelUnit::Point => 1.0,
                            MouseWheelUnit::Page => POINTS_PER_PAGE,
                        };
                        wheel_points += delta.y * scale;
                    }
                    Event::Zoom(factor) => pinch_factor *= *factor,
                    Event::WindowFocused(false) => lost_focus = true,
                    _ => {}
                }
            }

            (
                input.pointer.interact_pos(),
                input.pointer.button_pressed(PointerButton::Primary),
                input.pointer.primary_down(),
                input.pointer.middle_down(),
                input.key_down(egui::Key::Space),
                input.pointer.delta(),
            )
        });

    let mut zoom_factor = pinch_factor * (wheel_points * WHEEL_ZOOM_SPEED).exp();
    if !zoom_factor.is_finite() || zoom_factor <= 0.0 {
        zoom_factor = 1.0;
    }

    FrameInput {
        pointer,
        primary_pressed,
        primary_down,
        middle_down,
        space_down,
        pan_delta,
        zoom_factor,
        lost_focus,
        over_canvas,
    }
}
