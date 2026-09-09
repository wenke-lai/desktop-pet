use pet_core::{content::{load_pet, natural_cmp, scan}, geometry::{self, Point, Size, WorkArea}, settings::Settings};
use serde_json::{json, Value};
use std::{fs, path::Path};

fn manifest() -> Value {
    json!({ "schema_version": 1, "id": "sample", "display_name": "Sample",
      "render": { "scale": 1.0, "anchor_x": 0.5, "anchor_y": 0.9 },
      "fallback_animation": "breathing", "animations": {
        "breathing": { "source": { "type": "png_sequence", "dir": "animations/base" }, "fps": 24, "loop": true, "priority": 0, "interruptible": true },
        "arbitrary-action": { "source": { "type": "png_sequence", "dir": "animations/other" }, "fps": 30, "loop": false, "priority": 50, "interruptible": false }
      }, "behaviors": [{ "id": "respond", "trigger": { "type": "click", "button": "left" }, "choose": [{ "animation": "arbitrary-action", "weight": 1 }] }] })
}
fn pack(root: &Path, value: &Value) {
    for dir in ["base", "other"] {
        fs::create_dir_all(root.join("animations").join(dir)).unwrap();
        for name in ["10.png", "2.png", "1.png", "11.png"] {
            image::RgbaImage::from_pixel(2, 2, image::Rgba([40, 150, 90, 128])).save(root.join("animations").join(dir).join(name)).unwrap();
        }
    }
    fs::write(root.join("pet.json"), serde_json::to_vec(value).unwrap()).unwrap();
}
#[test]
fn valid_manifest_discovers_alpha_pngs_with_natural_sort() {
    let temp = tempfile::tempdir().unwrap(); pack(temp.path(), &manifest());
    fs::write(temp.path().join("animations/base/ignore.jpg"), b"not an image").unwrap();
    let pet = load_pet(temp.path(), &mut vec![]).unwrap();
    let frames = &pet.clips["breathing"].frames;
    let names: Vec<_> = frames.iter().map(|f| Path::new(f).file_name().unwrap().to_str().unwrap()).collect();
    assert_eq!(names, ["1.png", "2.png", "10.png", "11.png"]);
    assert_eq!(pet.definition.fallback_animation, "breathing");
    assert_eq!((pet.definition.render.canvas_width, pet.definition.render.canvas_height), (2, 2));
}
#[test]
fn png_dimensions_replace_legacy_manifest_dimensions_and_reach_the_frontend() {
    let temp = tempfile::tempdir().unwrap(); let mut data = manifest();
    data["render"]["canvas_width"] = json!(512);
    data["render"]["canvas_height"] = json!(512);
    data["render"]["scale"] = json!(0.5);
    pack(temp.path(), &data);
    for dir in ["base", "other"] {
        for entry in fs::read_dir(temp.path().join("animations").join(dir)).unwrap() {
            image::RgbaImage::new(8, 4).save(entry.unwrap().path()).unwrap();
        }
    }
    let pet = load_pet(temp.path(), &mut vec![]).unwrap();
    let render = &serde_json::to_value(&pet).unwrap()["render"];
    assert_eq!(render["canvas_width"], 8);
    assert_eq!(render["canvas_height"], 4);
    assert_eq!(render["scale"], 0.5);
    assert_eq!(render["anchor_x"], 0.5);
    assert_eq!(render["anchor_y"], 0.9);
}
#[test]
fn fallback_sets_dimensions_before_alphabetically_earlier_optional_animation() {
    let temp = tempfile::tempdir().unwrap(); pack(temp.path(), &manifest());
    for entry in fs::read_dir(temp.path().join("animations/other")).unwrap() {
        image::RgbaImage::new(1, 1).save(entry.unwrap().path()).unwrap();
    }
    let mut warnings = vec![];
    let pet = load_pet(temp.path(), &mut warnings).unwrap();
    assert_eq!((pet.definition.render.canvas_width, pet.definition.render.canvas_height), (2, 2));
    assert_eq!(pet.clips.len(), 1);
    assert!(pet.clips.contains_key("breathing"));
    assert!(pet.definition.behaviors.is_empty());
    assert!(warnings.iter().any(|w| w.contains("frame size must match fallback PNG")));
}
#[test]
fn oversized_inferred_dimensions_are_rejected() {
    let temp = tempfile::tempdir().unwrap(); pack(temp.path(), &manifest());
    image::RgbaImage::new(4097, 1).save(temp.path().join("animations/base/1.png")).unwrap();
    let mut warnings = vec![];
    assert!(load_pet(temp.path(), &mut warnings).is_err());
    assert!(warnings.iter().any(|w| w.contains("PNG dimensions must be 1..4096")));
}
#[test]
fn invalid_json_is_skipped_while_other_pack_loads() {
    let temp = tempfile::tempdir().unwrap(); pack(&temp.path().join("good"), &manifest());
    fs::create_dir(temp.path().join("broken")).unwrap(); fs::write(temp.path().join("broken/pet.json"), "{bad").unwrap();
    let catalog = scan(temp.path()).unwrap(); assert_eq!(catalog.pets.len(), 1);
    assert!(catalog.warnings.iter().any(|s| s.contains("invalid JSON")));
}
#[test]
fn missing_fallback_is_rejected() {
    let temp = tempfile::tempdir().unwrap(); let mut data = manifest(); data["fallback_animation"] = json!("absent"); pack(temp.path(), &data);
    assert!(load_pet(temp.path(), &mut vec![]).unwrap_err().contains("fallback"));
}
#[test]
fn missing_reference_and_nonpositive_weights_are_removed_with_warning() {
    let temp = tempfile::tempdir().unwrap(); let mut data = manifest();
    data["behaviors"][0]["choose"] = json!([{ "animation": "absent", "weight": 1 }, { "animation": "breathing", "weight": 0 }, { "animation": "breathing", "weight": -10 }]);
    pack(temp.path(), &data); let mut warnings = vec![];
    let pet = load_pet(temp.path(), &mut warnings).unwrap();
    assert!(pet.definition.behaviors.is_empty()); assert!(warnings.iter().any(|s| s.contains("no valid weighted choices")));
}
#[test]
fn natural_sort_handles_padding_and_arbitrarily_large_numbers() {
    let mut values = vec!["11.png", "0002.png", "1.png", "10.png", "999999999999999999999999.png", "1000000000000000000000000.png"];
    values.sort_by(|a,b| natural_cmp(a,b));
    assert_eq!(values, ["1.png", "0002.png", "10.png", "11.png", "999999999999999999999999.png", "1000000000000000000000000.png"]);
}
#[test]
fn traversal_absolute_and_windows_escape_paths_are_rejected() {
    for dir in ["../../secret", "/tmp", "C:/secret", "..\\secret", "\\\\server\\share"] {
        let temp = tempfile::tempdir().unwrap(); let mut data = manifest(); data["animations"]["breathing"]["source"]["dir"] = json!(dir); pack(temp.path(), &data);
        let mut warnings = vec![]; assert!(load_pet(temp.path(), &mut warnings).is_err());
        assert!(warnings.iter().any(|w| w.contains("unsafe relative path")));
    }
}
#[cfg(unix)]
#[test]
fn canonicalized_directory_and_frame_symlinks_cannot_escape() {
    use std::os::unix::fs::symlink;
    let temp = tempfile::tempdir().unwrap(); let outside = tempfile::tempdir().unwrap(); pack(temp.path(), &manifest());
    image::RgbaImage::new(2,2).save(outside.path().join("frame.png")).unwrap();
    symlink(outside.path().join("frame.png"), temp.path().join("animations/base/12.png")).unwrap();
    assert!(load_pet(temp.path(), &mut vec![]).is_err());
    let linked = tempfile::tempdir().unwrap(); symlink(temp.path(), linked.path().join("escape")).unwrap();
    assert!(scan(linked.path()).unwrap().pets.is_empty());
}
#[test]
fn zero_and_negative_fps_are_rejected() {
    for fps in [0, -1] { let temp = tempfile::tempdir().unwrap(); let mut data = manifest(); data["animations"]["breathing"]["fps"] = json!(fps); pack(temp.path(), &data); assert!(load_pet(temp.path(), &mut vec![]).unwrap_err().contains("fps")); }
}
#[test]
fn unsupported_optional_source_is_ignored() {
    let temp = tempfile::tempdir().unwrap(); let mut data = manifest(); data["animations"]["arbitrary-action"]["source"] = json!({ "type": "video", "file": "movie.mp4" }); pack(temp.path(), &data);
    let mut warnings = vec![]; let pet = load_pet(temp.path(), &mut warnings).unwrap();
    assert_eq!(pet.clips.len(), 1); assert!(warnings.iter().any(|w| w.contains("Unsupported animation source type")));
}
#[test]
fn corrupt_or_mismatched_png_and_empty_sequences_are_rejected() {
    for mode in 0..3 {
        let temp = tempfile::tempdir().unwrap(); pack(temp.path(), &manifest());
        let base = temp.path().join("animations/base");
        match mode {
            0 => fs::write(base.join("1.png"), b"broken").unwrap(),
            1 => image::RgbaImage::new(1,1).save(base.join("1.png")).unwrap(),
            _ => { for e in fs::read_dir(&base).unwrap() { fs::remove_file(e.unwrap().path()).unwrap(); } }
        }
        assert!(load_pet(temp.path(), &mut vec![]).is_err());
    }
}
#[test]
fn duplicate_pet_ids_are_skipped() {
    let temp = tempfile::tempdir().unwrap(); pack(&temp.path().join("a"), &manifest()); pack(&temp.path().join("b"), &manifest());
    assert_eq!(scan(temp.path()).unwrap().pets.len(), 1);
}
#[test]
fn settings_roundtrip_atomic_replace_and_corrupt_recovery() {
    let temp = tempfile::tempdir().unwrap(); let path = temp.path().join("settings.json");
    let mut settings = Settings::default(); settings.active_pet = Some("example".into()); settings.last_position = Some(Point { x: -1800.0, y: -200.0 }); settings.save(&path).unwrap();
    settings.scale = 1.5; settings.save(&path).unwrap(); let loaded = Settings::load(&path);
    assert_eq!(loaded.last_position, settings.last_position); assert_eq!(loaded.scale, 1.5);
    fs::write(&path, "oops").unwrap(); assert_eq!(Settings::load(&path).scale, 1.0);
}
#[test]
fn geometry_preserves_anchor_and_clamps_to_negative_monitor_work_area() {
    let area = WorkArea { origin: Point { x: -1920.0, y: -200.0 }, size: Size { width: 1920.0, height: 1040.0 }, scale: 1.5 };
    let size = Size { width: 768.0, height: 576.0 }; let normalized = Point { x: 0.5, y: 0.9 };
    let anchor = Point { x: -400.0, y: 700.0 };
    let origin = geometry::origin_from_anchor(anchor, size, normalized);
    assert_eq!(geometry::anchor_from_origin(origin, size, normalized), anchor);
    let clamped = geometry::clamp_origin(Point { x: 1000.0, y: 900.0 }, size, area);
    assert_eq!(clamped, Point { x: -768.0, y: 264.0 });
    assert_eq!(geometry::nearest_area(Point { x: 4000.0, y: 2000.0 }, &[area]).unwrap().origin, area.origin);
}
#[test]
fn invalid_trigger_ranges_and_duplicate_behavior_ids_fail() {
    for trigger in [json!({"type":"timer","min_ms":0,"max_ms":3}), json!({"type":"cursor_distance_enter","distance_px":180,"reset_distance_px":100})] {
        let temp = tempfile::tempdir().unwrap(); let mut data = manifest(); data["behaviors"][0]["trigger"] = trigger; pack(temp.path(), &data); assert!(load_pet(temp.path(), &mut vec![]).is_err());
    }
}
