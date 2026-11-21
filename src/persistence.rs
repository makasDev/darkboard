//! Versioned, local, human-readable document storage.
//!
//! The file is plain JSON in the user's app-data directory. Writes go to a
//! temporary file and are then renamed over the real one, so a crash mid-write
//! cannot truncate the previous board. A file we cannot understand is never
//! overwritten: it is renamed to `*.corrupt-<timestamp>.json` first.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use egui::{Pos2, Vec2};
use serde::{Deserialize, Serialize};

use crate::canvas::camera::{Camera, MAX_ZOOM, MIN_ZOOM};
use crate::canvas::stroke::{Stroke, sanitize_width};
use crate::document::Document;

pub const FORMAT_VERSION: u32 = 1;

#[derive(Serialize, Deserialize)]
struct FileDocument {
    version: u32,
    #[serde(default)]
    camera: Option<FileCamera>,
    #[serde(default)]
    marker_width: Option<f32>,
    #[serde(default)]
    strokes: Vec<FileStroke>,
}

#[derive(Serialize, Deserialize)]
struct FileCamera {
    center: [f32; 2],
    zoom: f32,
}

#[derive(Serialize, Deserialize)]
struct FileStroke {
    width: f32,
    points: Vec<[f32; 2]>,
}

pub struct Loaded {
    pub strokes: Vec<Stroke>,
    pub camera: Option<Camera>,
    pub marker_width: Option<f32>,
}

/// `%APPDATA%\Darkboard\whiteboard.json` on Windows, XDG data home elsewhere.
pub fn default_path() -> PathBuf {
    let base = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("XDG_DATA_HOME").map(PathBuf::from))
        .or_else(|| {
            std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local").join("share"))
        })
        .unwrap_or_else(|| PathBuf::from("."));

    base.join("Darkboard").join("whiteboard.json")
}

/// Load a board. `Ok(None)` means the file does not exist yet.
pub fn load(path: &Path) -> Result<Option<Loaded>, String> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(format!("could not read {}: {err}", path.display())),
    };

    let file: FileDocument = serde_json::from_str(&text)
        .map_err(|err| format!("invalid save data in {}: {err}", path.display()))?;

    if file.version == 0 || file.version > FORMAT_VERSION {
        return Err(format!(
            "unsupported save version {} (this build understands version {FORMAT_VERSION})",
            file.version
        ));
    }

    let strokes = file
        .strokes
        .into_iter()
        .filter_map(|stroke| {
            let points: Vec<Pos2> = stroke
                .points
                .into_iter()
                .filter(|[x, y]| x.is_finite() && y.is_finite())
                .map(|[x, y]| Pos2::new(x, y))
                .collect();
            if points.is_empty() {
                return None;
            }
            Some(Stroke::new(points, stroke.width))
        })
        .collect();

    let camera = file.camera.and_then(|camera| {
        let [x, y] = camera.center;
        let valid = x.is_finite()
            && y.is_finite()
            && camera.zoom.is_finite()
            && (MIN_ZOOM..=MAX_ZOOM).contains(&camera.zoom);
        valid.then(|| Camera {
            center: Vec2::new(x, y),
            zoom: camera.zoom,
        })
    });

    let marker_width = file.marker_width.map(sanitize_width);

    Ok(Some(Loaded {
        strokes,
        camera,
        marker_width,
    }))
}

/// Write the board atomically. Returns a human-readable error on failure.
pub fn save(
    path: &Path,
    document: &Document,
    camera: &Camera,
    marker_width: f32,
) -> Result<(), String> {
    let file = FileDocument {
        version: FORMAT_VERSION,
        camera: Some(FileCamera {
            center: [camera.center.x, camera.center.y],
            zoom: camera.zoom,
        }),
        marker_width: Some(marker_width),
        strokes: document
            .strokes()
            .iter()
            .map(|stroke| FileStroke {
                width: stroke.width(),
                points: stroke
                    .points()
                    .iter()
                    .map(|point| [point.x, point.y])
                    .collect(),
            })
            .collect(),
    };

    let json = serde_json::to_string(&file)
        .map_err(|err| format!("could not serialize the board: {err}"))?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("could not create {}: {err}", parent.display()))?;
    }

    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, json).map_err(|err| format!("could not write {}: {err}", tmp.display()))?;

    if let Err(err) = fs::rename(&tmp, path) {
        let _ = fs::remove_file(&tmp);
        return Err(format!("could not replace {}: {err}", path.display()));
    }

    Ok(())
}

/// Move an unreadable file aside so autosave can start fresh without losing it.
pub fn quarantine(path: &Path) -> Option<PathBuf> {
    if !path.exists() {
        return None;
    }

    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("whiteboard");

    for attempt in 0..100 {
        let suffix = if attempt == 0 {
            String::new()
        } else {
            format!("-{attempt}")
        };
        let target = path.with_file_name(format!("{stem}.corrupt-{stamp}{suffix}.json"));
        if target.exists() {
            continue;
        }
        return fs::rename(path, &target).ok().map(|()| target);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::pos2;

    fn temp_dir(name: &str) -> PathBuf {
        let mut dir = std::env::temp_dir();
        dir.push(format!("darkboard-test-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    fn sample_document() -> Document {
        let mut document = Document::new();
        document.push(Stroke::new(vec![pos2(1.0, 2.0), pos2(3.0, 4.0)], 2.5));
        document.push(Stroke::new(vec![pos2(-5.0, 8.0)], 7.0));
        document
    }

    #[test]
    fn round_trip() {
        let dir = temp_dir("round-trip");
        let path = dir.join("board.json");
        let document = sample_document();
        let camera = Camera {
            center: Vec2::new(12.0, -3.5),
            zoom: 1.75,
        };

        save(&path, &document, &camera, 4.0).expect("save");
        let loaded = load(&path).expect("load").expect("file exists");

        assert_eq!(loaded.strokes.len(), 2);
        assert_eq!(loaded.strokes[0].points().len(), 2);
        assert!((loaded.strokes[0].width() - 2.5).abs() < 1.0e-6);
        assert_eq!(loaded.camera, Some(camera));
        assert_eq!(loaded.marker_width, Some(4.0));
    }

    #[test]
    fn missing_file_is_none() {
        let dir = temp_dir("missing");
        assert!(load(&dir.join("nope.json")).expect("load").is_none());
    }

    #[test]
    fn corrupted_file_is_an_error() {
        let dir = temp_dir("corrupt");
        let path = dir.join("board.json");
        fs::write(&path, "{ this is not json").expect("write");
        assert!(load(&path).is_err());
    }

    #[test]
    fn newer_version_is_rejected_and_kept() {
        let dir = temp_dir("newer");
        let path = dir.join("board.json");
        fs::write(&path, r#"{"version":999,"strokes":[]}"#).expect("write");

        assert!(load(&path).is_err());

        let moved = quarantine(&path).expect("quarantine");
        assert!(!path.exists());
        assert!(moved.exists());
    }

    #[test]
    fn invalid_points_are_dropped_on_load() {
        let dir = temp_dir("sanitize");
        let path = dir.join("board.json");
        fs::write(
            &path,
            r#"{"version":1,"strokes":[{"width":2.0,"points":[[0.0,0.0],[null,1.0],[4.0,4.0]]}]}"#,
        )
        .expect("write");

        // `null` is not a valid pair, so the whole file fails to parse. That is
        // the documented behaviour: quarantine, never silently guess.
        assert!(load(&path).is_err());
    }

    #[test]
    fn bad_camera_falls_back_to_none() {
        let dir = temp_dir("camera");
        let path = dir.join("board.json");
        fs::write(
            &path,
            r#"{"version":1,"camera":{"center":[0.0,0.0],"zoom":0.0},"strokes":[]}"#,
        )
        .expect("write");

        let loaded = load(&path).expect("load").expect("file exists");
        assert!(loaded.camera.is_none());
    }
}
