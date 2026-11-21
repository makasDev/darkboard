//! Viewport camera: a world-space center plus a uniform zoom factor.

use egui::{Pos2, Rect, Vec2};

pub const MIN_ZOOM: f32 = 0.02;
pub const MAX_ZOOM: f32 = 32.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    /// World position shown at the center of the viewport.
    pub center: Vec2,
    /// Screen points per world unit.
    pub zoom: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            center: Vec2::ZERO,
            zoom: 1.0,
        }
    }
}

impl Camera {
    pub fn world_to_screen(&self, world: Pos2, viewport: Rect) -> Pos2 {
        viewport.center() + (world.to_vec2() - self.center) * self.zoom
    }

    pub fn screen_to_world(&self, screen: Pos2, viewport: Rect) -> Pos2 {
        let v = self.center + (screen - viewport.center()) / self.zoom;
        Pos2::new(v.x, v.y)
    }

    pub fn world_rect_to_screen(&self, rect: Rect, viewport: Rect) -> Rect {
        Rect::from_min_max(
            self.world_to_screen(rect.min, viewport),
            self.world_to_screen(rect.max, viewport),
        )
    }

    pub fn visible_world_rect(&self, viewport: Rect) -> Rect {
        Rect::from_min_max(
            self.screen_to_world(viewport.min, viewport),
            self.screen_to_world(viewport.max, viewport),
        )
    }

    /// Pan by a screen-space delta. Stored world coordinates never change.
    pub fn pan(&mut self, screen_delta: Vec2) {
        self.center -= screen_delta / self.zoom;
    }

    /// Zoom by `factor`, keeping the world point under `anchor` fixed on screen.
    pub fn zoom_by(&mut self, factor: f32, anchor: Pos2, viewport: Rect) {
        if !factor.is_finite() || factor <= 0.0 {
            return;
        }

        let anchored_world = self.screen_to_world(anchor, viewport);
        self.zoom = (self.zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);

        let offset = (anchor - viewport.center()) / self.zoom;
        self.center = anchored_world.to_vec2() - offset;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::pos2;

    fn viewport() -> Rect {
        Rect::from_min_size(pos2(0.0, 0.0), egui::vec2(800.0, 600.0))
    }

    #[test]
    fn round_trip_world_screen() {
        let camera = Camera {
            center: egui::vec2(120.0, -40.0),
            zoom: 2.5,
        };
        let viewport = viewport();
        let world = pos2(310.0, 220.0);
        let screen = camera.world_to_screen(world, viewport);
        let back = camera.screen_to_world(screen, viewport);
        assert!((back - world).length() < 1.0e-3);
    }

    #[test]
    fn zoom_keeps_anchor_fixed() {
        let mut camera = Camera::default();
        let viewport = viewport();
        let anchor = pos2(640.0, 120.0);
        let before = camera.screen_to_world(anchor, viewport);

        camera.zoom_by(1.7, anchor, viewport);

        let after = camera.screen_to_world(anchor, viewport);
        assert!((after - before).length() < 1.0e-3);
        assert!((camera.zoom - 1.7).abs() < 1.0e-5);
    }

    #[test]
    fn zoom_is_clamped() {
        let mut camera = Camera::default();
        camera.zoom_by(1.0e9, pos2(0.0, 0.0), viewport());
        assert_eq!(camera.zoom, MAX_ZOOM);
        camera.zoom_by(1.0e-9, pos2(0.0, 0.0), viewport());
        assert_eq!(camera.zoom, MIN_ZOOM);
    }

    #[test]
    fn pan_does_not_change_world_anchor() {
        let mut camera = Camera::default();
        let viewport = viewport();
        let world = camera.screen_to_world(pos2(100.0, 100.0), viewport);
        camera.pan(egui::vec2(50.0, -25.0));
        let screen = camera.world_to_screen(world, viewport);
        assert!((screen - pos2(150.0, 75.0)).length() < 1.0e-3);
    }
}
