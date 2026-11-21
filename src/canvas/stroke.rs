//! One immutable marker stroke in world space.

use egui::{Pos2, Rect};

#[derive(Clone, Debug)]
pub struct Stroke {
    points: Vec<Pos2>,
    width: f32,
    /// Cached axis-aligned bounds of `points` (without width).
    bounds: Rect,
}

impl Stroke {
    pub const MIN_WIDTH: f32 = 0.25;
    pub const MAX_WIDTH: f32 = 64.0;

    pub fn new(points: Vec<Pos2>, width: f32) -> Self {
        let mut stroke = Self {
            points,
            width: sanitize_width(width),
            bounds: Rect::NOTHING,
        };
        stroke.recompute_bounds();
        stroke
    }

    pub fn points(&self) -> &[Pos2] {
        &self.points
    }

    pub fn width(&self) -> f32 {
        self.width
    }

    pub fn bounds(&self) -> Rect {
        self.bounds
    }

    pub fn recompute_bounds(&mut self) {
        self.bounds = Rect::from_points(&self.points);
    }

    /// Drop non-finite points and make sure the width is usable.
    /// Returns `false` if the stroke is empty and should be discarded.
    pub fn sanitize(&mut self) -> bool {
        self.points.retain(|p| p.x.is_finite() && p.y.is_finite());
        self.width = sanitize_width(self.width);
        self.recompute_bounds();
        !self.points.is_empty()
    }

    pub fn is_valid(&self) -> bool {
        !self.points.is_empty()
            && self.width.is_finite()
            && self
                .points
                .iter()
                .all(|p| p.x.is_finite() && p.y.is_finite())
    }
}

pub fn sanitize_width(width: f32) -> f32 {
    if width.is_finite() {
        width.clamp(Stroke::MIN_WIDTH, Stroke::MAX_WIDTH)
    } else {
        3.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::pos2;

    #[test]
    fn bounds_cover_points() {
        let stroke = Stroke::new(vec![pos2(-2.0, 5.0), pos2(10.0, -3.0)], 2.0);
        assert_eq!(stroke.bounds().min, pos2(-2.0, -3.0));
        assert_eq!(stroke.bounds().max, pos2(10.0, 5.0));
    }

    #[test]
    fn invalid_points_are_dropped() {
        let mut stroke = Stroke::new(
            vec![pos2(0.0, 0.0), pos2(f32::NAN, 1.0), pos2(4.0, 4.0)],
            2.0,
        );
        assert!(stroke.sanitize());
        assert_eq!(stroke.points().len(), 2);
    }

    #[test]
    fn width_is_clamped() {
        assert_eq!(
            Stroke::new(vec![pos2(0.0, 0.0)], 1.0e9).width(),
            Stroke::MAX_WIDTH
        );
        assert_eq!(
            Stroke::new(vec![pos2(0.0, 0.0)], 0.0).width(),
            Stroke::MIN_WIDTH
        );
        assert_eq!(Stroke::new(vec![pos2(0.0, 0.0)], f32::NAN).width(), 3.0);
    }

    #[test]
    fn empty_stroke_is_not_valid() {
        assert!(!Stroke::new(Vec::new(), 3.0).is_valid());
    }
}
