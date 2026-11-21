//! Application state: document, camera, live stroke, autosave.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use egui::{
    Color32, Context, CursorIcon, FontId, Id, Key, Modifiers, Pos2, Rect, Sense,
    Stroke as LineStroke, Ui, Vec2,
};

use crate::canvas::camera::Camera;
use crate::canvas::input::{self, FrameInput};
use crate::canvas::render;
use crate::canvas::smoothing;
use crate::canvas::stroke::Stroke;
use crate::document::{Document, History};
use crate::persistence;
use crate::theme;
use crate::ui::{self, ToolbarActions};

/// How long the board stays quiet before it is written to disk.
const AUTOSAVE_DELAY: Duration = Duration::from_millis(1200);
/// How long to wait before retrying a failed autosave.
const AUTOSAVE_RETRY: Duration = Duration::from_secs(4);
/// Raw pointer samples closer than this (screen points) are dropped.
const MIN_POINT_DISTANCE_PX: f32 = 0.8;
/// Spacing of the smoothed polyline in screen points.
const SMOOTH_STEP_PX: f32 = 1.2;
/// Screen-space radius of the eraser brush.
const ERASER_RADIUS_PX: f32 = 14.0;
/// How long a one-off notice stays on screen.
const NOTICE_LIFETIME: Duration = Duration::from_secs(14);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Idle,
    Drawing,
    Panning,
    Erasing,
}

struct LiveStroke {
    raw: Vec<Pos2>,
    smoothed: Vec<Pos2>,
    width: f32,
    /// World position where the press happened. Anchors a Shift straight line.
    anchor: Pos2,
}

pub struct DarkboardApp {
    document: Document,
    history: History,
    camera: Camera,
    marker_width: f32,

    mode: Mode,
    live: Option<LiveStroke>,
    last_raw: Option<Pos2>,

    erasing: Vec<Stroke>,

    save_path: PathBuf,
    dirty: bool,
    dirty_since: Option<Instant>,
    retry_at: Option<Instant>,
    save_error: Option<String>,
    notice: Option<(String, Instant)>,
}

impl DarkboardApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::apply(&cc.egui_ctx);

        let save_path = persistence::default_path();
        let mut document = Document::new();
        let mut camera = Camera::default();
        let mut marker_width = 3.0;
        let mut notice = None;

        match persistence::load(&save_path) {
            Ok(Some(loaded)) => {
                document.replace_sanitized(loaded.strokes);
                if let Some(saved_camera) = loaded.camera {
                    camera = saved_camera;
                }
                if let Some(saved_width) = loaded.marker_width {
                    marker_width = saved_width;
                }
            }
            Ok(None) => {}
            Err(err) => {
                let message = match persistence::quarantine(&save_path) {
                    Some(path) => format!(
                        "Could not read the saved board ({err}). The old file was kept as {}.",
                        path.display()
                    ),
                    None => format!(
                        "Could not read the saved board ({err}). Starting with an empty canvas."
                    ),
                };
                notice = Some((message, Instant::now() + NOTICE_LIFETIME));
            }
        }

        Self {
            document,
            history: History::default(),
            camera,
            marker_width,
            mode: Mode::Idle,
            live: None,
            last_raw: None,
            erasing: Vec::new(),
            save_path,
            dirty: false,
            dirty_since: None,
            retry_at: None,
            save_error: None,
            notice,
        }
    }

    fn mark_dirty(&mut self) {
        self.dirty = true;
        self.dirty_since = Some(Instant::now());
        self.retry_at = None;
    }

    fn apply_actions(&mut self, actions: ToolbarActions) {
        if actions.undo && self.history.undo(&mut self.document) {
            self.mark_dirty();
        }
        if actions.redo && self.history.redo(&mut self.document) {
            self.mark_dirty();
        }
        if actions.clear && !self.document.is_empty() {
            let cleared = self.document.take_all();
            self.history.record_clear(cleared);
            self.mark_dirty();
        }
    }

    fn handle_shortcuts(&mut self, ctx: &Context) {
        let shift_command = Modifiers {
            command: true,
            shift: true,
            ..Modifiers::NONE
        };

        let redo = ctx.input_mut(|input| {
            input.consume_key(shift_command, Key::Z)
                || input.consume_key(Modifiers::COMMAND, Key::Y)
        });
        let undo = ctx.input_mut(|input| input.consume_key(Modifiers::COMMAND, Key::Z));

        if redo && self.history.redo(&mut self.document) {
            self.mark_dirty();
        }
        if undo && self.history.undo(&mut self.document) {
            self.mark_dirty();
        }
    }

    fn handle_navigation(&mut self, frame_input: &FrameInput, viewport: Rect) {
        if (frame_input.zoom_factor - 1.0).abs() > f32::EPSILON
            && frame_input.over_canvas
            && let Some(anchor) = frame_input.pointer
        {
            self.camera
                .zoom_by(frame_input.zoom_factor, anchor, viewport);
            self.mark_dirty();
        }

        match self.mode {
            Mode::Panning => {
                if frame_input.pan_delta != Vec2::ZERO {
                    self.camera.pan(frame_input.pan_delta);
                    self.mark_dirty();
                }
                if !frame_input.middle_down && !frame_input.primary_down {
                    self.mode = Mode::Idle;
                }
            }
            Mode::Idle => {
                let wants_pan =
                    frame_input.middle_down || (frame_input.space_down && frame_input.primary_down);
                if frame_input.over_canvas && wants_pan {
                    self.mode = Mode::Panning;
                    self.camera.pan(frame_input.pan_delta);
                    self.mark_dirty();
                }
            }
            Mode::Drawing => {}
            Mode::Erasing => {}
        }
    }

    fn handle_drawing(&mut self, frame_input: &FrameInput, viewport: Rect, over_canvas: bool) {
        match self.mode {
            Mode::Drawing => {
                let zoom = self.camera.zoom.max(0.001);
                let step = SMOOTH_STEP_PX / zoom;
                let min_distance = MIN_POINT_DISTANCE_PX / zoom;

                if let Some(live) = self.live.as_mut() {
                    if let Some(pointer) = frame_input.pointer {
                        let world = self.camera.screen_to_world(pointer, viewport);
                        if self
                            .last_raw
                            .is_none_or(|last| last.distance(world) >= min_distance)
                        {
                            live.raw.push(world);
                            self.last_raw = Some(world);
                        }
                    }

                    if frame_input.shift_down {
                        let end = live.raw.last().copied().unwrap_or(live.anchor);
                        live.smoothed.clear();
                        live.smoothed.push(live.anchor);
                        if live.anchor.distance(end) > 1.0e-4 {
                            live.smoothed.push(end);
                        }
                    } else {
                        smoothing::smooth_into(&live.raw, step, &mut live.smoothed);
                    }
                }

                if !frame_input.primary_down || frame_input.lost_focus {
                    self.finish_stroke(frame_input.shift_down);
                }
            }
            Mode::Idle => {
                let wants_draw =
                    frame_input.primary_pressed && over_canvas && !frame_input.space_down;
                if wants_draw && let Some(pointer) = frame_input.pointer {
                    let world = self.camera.screen_to_world(pointer, viewport);
                    self.live = Some(LiveStroke {
                        raw: vec![world],
                        smoothed: vec![world],
                        width: self.marker_width,
                        anchor: world,
                    });
                    self.last_raw = Some(world);
                    self.mode = Mode::Drawing;
                }
            }
            Mode::Panning => {}
            Mode::Erasing => {}
        }
    }

    fn handle_erasing(&mut self, frame_input: &FrameInput, viewport: Rect, over_canvas: bool) {
        match self.mode {
            Mode::Erasing => {
                if frame_input.right_down {
                    if let Some(pointer) = frame_input.pointer {
                        let world = self.camera.screen_to_world(pointer, viewport);
                        let radius = ERASER_RADIUS_PX / self.camera.zoom.max(0.001);
                        let mut i = 0;
                        while i < self.document.len() {
                            let bounds = self.document.strokes()[i].bounds().expand(radius);
                            if bounds.contains(world) {
                                let removed = self.document.remove(i);
                                self.erasing.push(removed);
                            } else {
                                i += 1;
                            }
                        }
                    }
                } else {
                    if !self.erasing.is_empty() {
                        let removed = std::mem::take(&mut self.erasing);
                        self.history.record_erase(removed);
                        self.mark_dirty();
                    }
                    self.mode = Mode::Idle;
                }
            }
            Mode::Idle => {
                if frame_input.right_pressed && over_canvas {
                    self.mode = Mode::Erasing;
                    self.erasing.clear();
                }
            }
            Mode::Drawing | Mode::Panning => {}
        }
    }

    fn finish_stroke(&mut self, shift: bool) {
        self.mode = Mode::Idle;
        self.last_raw = None;

        let Some(mut live) = self.live.take() else {
            return;
        };

        let width = live.width;
        let points = if shift {
            let end = live.raw.last().copied().unwrap_or(live.anchor);
            let start = if live.raw.len() <= 1 {
                // A Shift+click with no drag continues from the end of the last
                // stroke, so a sequence of clicks draws connected segments.
                self.document
                    .strokes()
                    .last()
                    .and_then(|stroke| stroke.points().last().copied())
                    .unwrap_or(live.anchor)
            } else {
                live.anchor
            };
            if start.distance(end) > 1.0e-4 {
                vec![start, end]
            } else {
                vec![end]
            }
        } else {
            let step = SMOOTH_STEP_PX / self.camera.zoom.max(0.001);
            smoothing::smooth_into(&live.raw, step, &mut live.smoothed);
            if live.smoothed.is_empty() {
                live.raw
            } else {
                live.smoothed
            }
        };

        let stroke = Stroke::new(points, width);
        if stroke.is_valid() {
            self.document.push(stroke);
            self.history.record_add();
            self.mark_dirty();
        }
    }

    fn draw_cursor_preview(
        &self,
        painter: &egui::Painter,
        frame_input: &FrameInput,
        over_canvas: bool,
    ) {
        if !over_canvas {
            return;
        }
        if matches!(self.mode, Mode::Erasing) {
            if let Some(pointer) = frame_input.pointer {
                painter.circle_stroke(
                    pointer,
                    ERASER_RADIUS_PX,
                    LineStroke::new(1.5, Color32::from_white_alpha(120)),
                );
            }
            return;
        }
        if matches!(self.mode, Mode::Panning) || frame_input.space_down {
            return;
        }
        let Some(pointer) = frame_input.pointer else {
            return;
        };

        let radius = (self.marker_width * self.camera.zoom * 0.5).clamp(2.0, 40.0);
        painter.circle_stroke(
            pointer,
            radius,
            LineStroke::new(1.0, Color32::from_white_alpha(70)),
        );
    }

    fn draw_hint(&self, painter: &egui::Painter, viewport: Rect) {
        if !self.document.is_empty() || self.live.is_some() {
            return;
        }

        painter.text(
            viewport.center(),
            egui::Align2::CENTER_CENTER,
            "Draw with the left mouse button\nPan with middle-drag or Space+drag · Zoom with the wheel\nHold Shift for a straight line · Shift+click continues · Right-drag erases",
            FontId::proportional(14.0),
            theme::TEXT_DIM,
        );
    }

    fn autosave(&mut self, ctx: &Context) {
        if !self.dirty {
            return;
        }

        let now = Instant::now();

        if let Some(retry_at) = self.retry_at {
            if now < retry_at {
                ctx.request_repaint_after(retry_at - now);
                return;
            }
        } else if let Some(dirty_since) = self.dirty_since {
            let due = dirty_since + AUTOSAVE_DELAY;
            if now < due {
                ctx.request_repaint_after(due - now);
                return;
            }
        }

        self.save_now();
    }

    fn save_now(&mut self) {
        match persistence::save(
            &self.save_path,
            &self.document,
            &self.camera,
            self.marker_width,
        ) {
            Ok(()) => {
                self.dirty = false;
                self.dirty_since = None;
                self.retry_at = None;
                self.save_error = None;
            }
            Err(err) => {
                self.retry_at = Some(Instant::now() + AUTOSAVE_RETRY);
                self.save_error = Some(err);
            }
        }
    }

    fn show_notice(&mut self, ctx: &Context) {
        if let Some((_, until)) = &self.notice
            && Instant::now() >= *until
        {
            self.notice = None;
        }

        let text = match (&self.save_error, &self.notice) {
            (Some(err), _) => format!("Autosave failed: {err}"),
            (None, Some((message, _))) => message.clone(),
            (None, None) => return,
        };

        egui::Area::new(Id::unique("darkboard.notice"))
            .order(egui::Order::Foreground)
            .interactable(false)
            .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 14.0))
            .show(ctx, |ui| {
                egui::Frame::NONE
                    .fill(theme::NOTICE_BG)
                    .stroke(LineStroke::new(1.0, theme::NOTICE_BORDER))
                    .corner_radius(egui::CornerRadius::same(8))
                    .inner_margin(egui::Margin {
                        left: 12,
                        right: 12,
                        top: 8,
                        bottom: 8,
                    })
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new(text).size(13.0).color(theme::TEXT));
                    });
            });

        ctx.request_repaint_after(Duration::from_millis(500));
    }
}

impl eframe::App for DarkboardApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        self.handle_shortcuts(&ctx);

        let viewport = ui.max_rect();

        let mut actions = ToolbarActions::default();
        ui::toolbar(
            &ctx,
            &mut self.marker_width,
            self.document.len(),
            self.history.can_undo(),
            self.history.can_redo(),
            self.camera.zoom,
            &mut actions,
        );
        self.apply_actions(actions);

        // The toolbar lives on a foreground layer, so this interaction on the
        // background layer correctly yields over the toolbar.
        let canvas = ui.interact(
            viewport,
            Id::unique("darkboard.canvas"),
            Sense::click_and_drag(),
        );
        let over_canvas = canvas.contains_pointer();
        let frame_input = input::read(&ctx, over_canvas);

        self.handle_navigation(&frame_input, viewport);
        self.handle_drawing(&frame_input, viewport, over_canvas);
        self.handle_erasing(&frame_input, viewport, over_canvas);

        let painter = ui.painter();
        painter.rect_filled(viewport, egui::CornerRadius::ZERO, theme::CANVAS_BG);
        render::draw_grid(painter, &self.camera, viewport);
        render::draw_strokes(painter, self.document.strokes(), &self.camera, viewport);
        if let Some(live) = &self.live {
            render::draw_polyline(painter, &live.smoothed, live.width, &self.camera, viewport);
        }
        self.draw_hint(painter, viewport);
        self.draw_cursor_preview(painter, &frame_input, over_canvas);

        if matches!(self.mode, Mode::Panning) {
            ctx.set_cursor_icon(CursorIcon::Grabbing);
        } else if matches!(self.mode, Mode::Erasing) {
            ctx.set_cursor_icon(CursorIcon::Crosshair);
        } else if over_canvas {
            ctx.set_cursor_icon(if frame_input.space_down {
                CursorIcon::Grab
            } else {
                CursorIcon::Crosshair
            });
        }

        self.autosave(&ctx);
        self.show_notice(&ctx);
    }

    fn on_exit(&mut self) {
        if self.dirty {
            self.save_now();
        }
    }
}
