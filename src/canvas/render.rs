//! Painting the infinite board: dot grid, strokes, and screen-space culling.

use egui::{Mesh, Painter, Pos2, Rect, Shape, Stroke as LineStroke, Vec2};

use crate::canvas::camera::Camera;
use crate::canvas::stroke::Stroke;
use crate::theme;

/// Points closer than this on screen add nothing but triangles.
const MIN_SCREEN_POINT_DISTANCE: f32 = 1.0;
/// Upper bound on grid dots, so an extreme viewport cannot stall a frame.
const MAX_GRID_DOTS: i64 = 20_000;

pub fn draw_grid(painter: &Painter, camera: &Camera, viewport: Rect) {
    let mut spacing = 32.0_f32;
    for _ in 0..64 {
        if spacing * camera.zoom >= 28.0 {
            break;
        }
        spacing *= 2.0;
    }
    for _ in 0..64 {
        if spacing * camera.zoom <= 112.0 {
            break;
        }
        spacing *= 0.5;
    }

    if !spacing.is_finite() || spacing <= 0.0 {
        return;
    }

    let world = camera.visible_world_rect(viewport);
    let x0 = (world.min.x / spacing).floor() as i64;
    let x1 = (world.max.x / spacing).ceil() as i64;
    let y0 = (world.min.y / spacing).floor() as i64;
    let y1 = (world.max.y / spacing).ceil() as i64;

    let columns = x1.saturating_sub(x0).saturating_add(1);
    let rows = y1.saturating_sub(y0).saturating_add(1);
    let count = columns.saturating_mul(rows);
    if count <= 0 || count > MAX_GRID_DOTS {
        return;
    }

    let mut mesh = Mesh::default();
    let dot = Rect::from_center_size(Pos2::ZERO, Vec2::splat(2.0));
    for ix in x0..=x1 {
        let x = ix as f32 * spacing;
        for iy in y0..=y1 {
            let y = iy as f32 * spacing;
            let screen = camera.world_to_screen(Pos2::new(x, y), viewport);
            mesh.add_colored_rect(dot.translate(screen.to_vec2()), theme::GRID_DOT);
        }
    }

    painter.add(Shape::mesh(mesh));
}

pub fn draw_strokes(painter: &Painter, strokes: &[Stroke], camera: &Camera, viewport: Rect) {
    let visible = viewport.expand(2.0);
    for stroke in strokes {
        let bounds = stroke.bounds().expand(stroke.width() * 0.5);
        if !camera
            .world_rect_to_screen(bounds, viewport)
            .intersects(visible)
        {
            continue;
        }
        draw_polyline(painter, stroke.points(), stroke.width(), camera, viewport);
    }
}

/// Draw a world-space polyline, decimating points that are sub-pixel on screen.
pub fn draw_polyline(
    painter: &Painter,
    points: &[Pos2],
    width_world: f32,
    camera: &Camera,
    viewport: Rect,
) {
    let Some(&first_world) = points.first() else {
        return;
    };

    let width_px = (width_world * camera.zoom).max(0.35);
    let radius = (width_px * 0.5).max(0.35);
    let first = camera.world_to_screen(first_world, viewport);

    if points.len() == 1 {
        painter.circle_filled(first, radius, theme::MARKER);
        return;
    }

    let last_world = *points.last().unwrap_or(&first_world);
    let last = camera.world_to_screen(last_world, viewport);

    let mut screen: Vec<Pos2> = Vec::with_capacity(points.len().min(4096));
    screen.push(first);
    let mut previous = first;
    for &point in &points[1..] {
        let p = camera.world_to_screen(point, viewport);
        if p.distance(previous) >= MIN_SCREEN_POINT_DISTANCE {
            screen.push(p);
            previous = p;
        }
    }
    if screen.last().is_none_or(|p| p.distance(last) > 0.5) {
        screen.push(last);
    }

    if screen.len() >= 2 {
        painter.add(Shape::line(
            screen,
            LineStroke::new(width_px, theme::MARKER),
        ));
    }

    // egui draws flat caps on open paths; a marker wants rounded ones.
    if radius >= 0.6 {
        painter.circle_filled(first, radius, theme::MARKER);
        painter.circle_filled(last, radius, theme::MARKER);
    }
}
