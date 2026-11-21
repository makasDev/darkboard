//! Stroke smoothing: light denoise + centripetal Catmull-Rom resampling.
//!
//! Raw pointer samples are noisy and unevenly spaced. We first average each
//! interior sample with its neighbours (a centred, zero-lag 3-tap filter), then
//! walk a centripetal Catmull-Rom spline through the result and emit points at a
//! fixed screen-space density. Dense output keeps egui's line joins invisible
//! and makes handwriting feel continuous rather than polygonal.

use egui::Pos2;

/// Smooth `points` and write the result to `out` (cleared first).
///
/// `step` is the desired spacing between output points in world units.
pub fn smooth_into(points: &[Pos2], step: f32, out: &mut Vec<Pos2>) {
    out.clear();

    let n = points.len();
    if n == 0 {
        return;
    }
    if n == 1 {
        out.push(points[0]);
        return;
    }

    let step = if step.is_finite() && step > 0.0 {
        step
    } else {
        1.0
    };

    let denoised = |i: isize| -> Pos2 {
        let last = n as isize - 1;
        let idx = i.clamp(0, last) as usize;
        if idx == 0 || idx == n - 1 {
            points[idx]
        } else {
            let a = points[idx - 1];
            let b = points[idx];
            let c = points[idx + 1];
            Pos2::new(
                (a.x + 2.0 * b.x + c.x) * 0.25,
                (a.y + 2.0 * b.y + c.y) * 0.25,
            )
        }
    };

    out.push(denoised(0));

    for i in 0..n - 1 {
        let p0 = denoised(i as isize - 1);
        let p1 = denoised(i as isize);
        let p2 = denoised(i as isize + 1);
        let p3 = denoised(i as isize + 2);

        let d1 = p0.distance(p1).sqrt();
        let d2 = p1.distance(p2).sqrt();
        let d3 = p2.distance(p3).sqrt();

        if d2 <= 1.0e-6 {
            push_if_far(out, p2);
            continue;
        }

        let samples = ((p1.distance(p2) / step).ceil() as usize).clamp(1, 64);
        for s in 1..=samples {
            let t = s as f32 / samples as f32;
            let p = if d1 <= 1.0e-6 || d3 <= 1.0e-6 {
                // Degenerate neighbours: a straight segment is the safe answer.
                p1 + (p2 - p1) * t
            } else {
                catmull_rom(p0, p1, p2, p3, d1, d2, d3, t)
            };
            out.push(p);
        }
    }
}

fn push_if_far(out: &mut Vec<Pos2>, p: Pos2) {
    if out.last().is_none_or(|last| last.distance(p) > 1.0e-4) {
        out.push(p);
    }
}

#[allow(clippy::too_many_arguments)]
fn catmull_rom(p0: Pos2, p1: Pos2, p2: Pos2, p3: Pos2, d1: f32, d2: f32, d3: f32, t: f32) -> Pos2 {
    let t0 = 0.0;
    let t1 = t0 + d1;
    let t2 = t1 + d2;
    let t3 = t2 + d3;
    let t = t1 + (t2 - t1) * t;

    let a1 = lerp(p0, p1, (t - t0) / (t1 - t0));
    let a2 = lerp(p1, p2, (t - t1) / (t2 - t1));
    let a3 = lerp(p2, p3, (t - t2) / (t3 - t2));

    let b1 = lerp(a1, a2, (t - t0) / (t2 - t0));
    let b2 = lerp(a2, a3, (t - t1) / (t3 - t1));

    lerp(b1, b2, (t - t1) / (t2 - t1))
}

fn lerp(a: Pos2, b: Pos2, t: f32) -> Pos2 {
    a + (b - a) * t
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::pos2;

    #[test]
    fn single_point_is_preserved() {
        let mut out = Vec::new();
        smooth_into(&[pos2(3.0, 4.0)], 1.0, &mut out);
        assert_eq!(out, vec![pos2(3.0, 4.0)]);
    }

    #[test]
    fn empty_input_is_empty() {
        let mut out = vec![pos2(1.0, 1.0)];
        smooth_into(&[], 1.0, &mut out);
        assert!(out.is_empty());
    }

    #[test]
    fn straight_line_stays_straight() {
        let points: Vec<Pos2> = (0..=10).map(|i| pos2(i as f32 * 4.0, 0.0)).collect();
        let mut out = Vec::new();
        smooth_into(&points, 1.0, &mut out);

        assert!(out.len() > points.len());
        assert!(out.iter().all(|p| p.y.abs() < 1.0e-3));
        assert!((out.first().unwrap().x - 0.0).abs() < 1.0e-4);
        assert!((out.last().unwrap().x - 40.0).abs() < 1.0e-4);
    }

    #[test]
    fn duplicate_points_do_not_produce_nan() {
        let points = vec![pos2(0.0, 0.0); 6];
        let mut out = Vec::new();
        smooth_into(&points, 1.0, &mut out);
        assert!(!out.is_empty());
        assert!(out.iter().all(|p| p.x.is_finite() && p.y.is_finite()));
    }

    #[test]
    fn output_spacing_is_bounded() {
        let points = vec![pos2(0.0, 0.0), pos2(100.0, 0.0)];
        let mut out = Vec::new();
        smooth_into(&points, 2.0, &mut out);
        assert!(out.len() >= 50);
        assert!(out.len() <= 51);
    }
}
