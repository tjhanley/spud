use std::path::Path;

use anyhow::{bail, Context, Result};
use image::GenericImageView;

use crate::types::{FacePack, FacePackMeta, Frame, Mood};

/// Decode a face pack from raw PNG and JSON bytes.
///
/// This is the primary entry point — used for both embedded assets
/// (`include_bytes!`) and file-based loading.
pub fn load_from_bytes(png: &[u8], json: &[u8]) -> Result<FacePack> {
    let meta: FacePackMeta =
        serde_json::from_slice(json).context("failed to parse face pack JSON")?;

    let sheet = image::load_from_memory(png).context("failed to decode face pack PNG")?;
    let (sheet_w, sheet_h) = sheet.dimensions();

    let [frame_w, frame_h] = meta.sprite_size;
    let fpm = meta.frames_per_mood;

    let expected_w = frame_w * fpm as u32;
    let expected_h = frame_h * Mood::COUNT as u32;
    if sheet_w != expected_w || sheet_h != expected_h {
        bail!(
            "sprite sheet dimensions {sheet_w}×{sheet_h} don't match \
             expected {expected_w}×{expected_h} \
             ({fpm} frames × {frame_w}px wide, {} moods × {frame_h}px tall)",
            Mood::COUNT,
        );
    }

    let mood_map = parse_mood_map(&meta)?;

    let mut frames: Vec<Vec<Frame>> = Vec::with_capacity(Mood::COUNT);
    for _ in 0..Mood::COUNT {
        frames.push(Vec::with_capacity(fpm));
    }

    for mood in Mood::ALL {
        let row = mood_map[mood as usize];
        let y = row as u32 * frame_h;
        for col in 0..fpm as u32 {
            let x = col * frame_w;
            let sub = sheet.crop_imm(x, y, frame_w, frame_h);
            frames[mood as usize].push(Frame {
                data: sub.to_rgba8().into_raw(),
                width: frame_w,
                height: frame_h,
            });
        }
    }

    Ok(FacePack {
        frames,
        frames_per_mood: fpm,
        frame_width: frame_w,
        frame_height: frame_h,
    })
}

/// Load a face pack from PNG and JSON files on disk.
pub fn load_from_files(png_path: &Path, json_path: &Path) -> Result<FacePack> {
    let png = std::fs::read(png_path)
        .with_context(|| format!("failed to read {}", png_path.display()))?;
    let json = std::fs::read(json_path)
        .with_context(|| format!("failed to read {}", json_path.display()))?;
    load_from_bytes(&png, &json)
}

/// Map each [`Mood`] variant to its sprite-sheet row index via the JSON metadata.
///
/// Returns a `[usize; Mood::COUNT]` array where `result[mood as usize]` is the
/// row in the sheet for that mood. Validates that every mood has an entry.
fn parse_mood_map(meta: &FacePackMeta) -> Result<[usize; Mood::COUNT]> {
    let mut map = [0usize; Mood::COUNT];

    let name_to_mood: &[(&str, Mood)] = &[
        ("neutral", Mood::Neutral),
        ("happy", Mood::Happy),
        ("angry", Mood::Angry),
        ("god_mode", Mood::GodMode),
        ("hurt_real_bad", Mood::HurtRealBad),
        ("thinking", Mood::Thinking),
    ];

    for &(name, mood) in name_to_mood {
        let row = meta
            .moods
            .get(name)
            .with_context(|| format!("missing mood \"{name}\" in face pack metadata"))?;
        map[mood as usize] = *row;
    }

    Ok(map)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_meta(moods: &[(&str, usize)]) -> FacePackMeta {
        FacePackMeta {
            sprite_size: [2, 2],
            frames_per_mood: 1,
            moods: moods.iter().map(|&(k, v)| (k.to_string(), v)).collect(),
        }
    }

    fn all_moods() -> Vec<(&'static str, usize)> {
        vec![
            ("neutral", 0),
            ("happy", 1),
            ("angry", 2),
            ("god_mode", 3),
            ("hurt_real_bad", 4),
            ("thinking", 5),
        ]
    }

    #[test]
    fn parse_mood_map_valid() {
        let meta = make_meta(&all_moods());
        let map = parse_mood_map(&meta).unwrap();
        assert_eq!(map[Mood::Neutral as usize], 0);
        assert_eq!(map[Mood::GodMode as usize], 3);
        assert_eq!(map[Mood::Thinking as usize], 5);
    }

    #[test]
    fn parse_mood_map_reordered() {
        // JSON rows don't match enum order — map should still resolve correctly
        let meta = make_meta(&[
            ("neutral", 5),
            ("happy", 4),
            ("angry", 3),
            ("god_mode", 2),
            ("hurt_real_bad", 1),
            ("thinking", 0),
        ]);
        let map = parse_mood_map(&meta).unwrap();
        assert_eq!(map[Mood::Neutral as usize], 5);
        assert_eq!(map[Mood::Thinking as usize], 0);
    }

    #[test]
    fn parse_mood_map_missing_mood() {
        let mut moods = all_moods();
        moods.retain(|&(k, _)| k != "god_mode");
        let meta = make_meta(&moods);
        let err = parse_mood_map(&meta).unwrap_err();
        assert!(
            err.to_string().contains("god_mode"),
            "error should name the missing mood: {err}"
        );
    }

    #[test]
    fn bad_json_returns_error() {
        let png = include_bytes!("../../../assets/faces/default/sheet.png");
        let result = load_from_bytes(png, b"not json");
        assert!(result.is_err());
    }

    #[test]
    fn bad_png_returns_error() {
        let json = include_bytes!("../../../assets/faces/default/face.json");
        let result = load_from_bytes(b"not a png", json);
        assert!(result.is_err());
    }

    #[test]
    fn load_default_assets() {
        let png = include_bytes!("../../../assets/faces/default/sheet.png");
        let json = include_bytes!("../../../assets/faces/default/face.json");
        let pack = load_from_bytes(png, json).unwrap();

        assert_eq!(pack.frames.len(), Mood::COUNT);
        assert_eq!(pack.frames_per_mood, 7);
        assert_eq!(pack.frame_width, 128);
        assert_eq!(pack.frame_height, 128);

        for (i, mood_frames) in pack.frames.iter().enumerate() {
            assert_eq!(mood_frames.len(), 7, "mood {i} should have 7 frames");
            for frame in mood_frames {
                assert_eq!(frame.width, 128);
                assert_eq!(frame.height, 128);
                assert_eq!(frame.data.len(), 128 * 128 * 4);
            }
        }
    }
}
