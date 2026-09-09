use crate::manifest::{AnimationDefinition, AnimationSource, PetDefinition};
use serde::Serialize;
use std::{cmp::Ordering, collections::{BTreeMap, HashSet}, fs, path::{Component, Path, PathBuf}};

#[derive(Clone, Debug, Serialize)]
pub struct LoadedAnimation {
    #[serde(flatten)]
    pub definition: AnimationDefinition,
    pub frames: Vec<String>,
}
#[derive(Clone, Debug, Serialize)]
pub struct LoadedPet {
    #[serde(flatten)]
    pub definition: PetDefinition,
    pub clips: BTreeMap<String, LoadedAnimation>,
}
#[derive(Clone, Debug, Default, Serialize)]
pub struct Catalog { pub pets: Vec<LoadedPet>, pub warnings: Vec<String> }

// Compare digit runs as arbitrary-length integers: no integer overflow, stable ties.
pub fn natural_cmp(a: &str, b: &str) -> Ordering {
    let (aa, bb) = (a.as_bytes(), b.as_bytes());
    let (mut i, mut j) = (0, 0);
    while i < aa.len() && j < bb.len() {
        if aa[i].is_ascii_digit() && bb[j].is_ascii_digit() {
            let (start_i, start_j) = (i, j);
            while i < aa.len() && aa[i].is_ascii_digit() { i += 1; }
            while j < bb.len() && bb[j].is_ascii_digit() { j += 1; }
            let x = a[start_i..i].trim_start_matches('0');
            let y = b[start_j..j].trim_start_matches('0');
            let order = x.len().cmp(&y.len()).then_with(|| x.cmp(y));
            if order != Ordering::Equal { return order; }
        } else {
            let order = aa[i].cmp(&bb[j]);
            if order != Ordering::Equal { return order; }
            i += 1; j += 1;
        }
    }
    aa.len().cmp(&bb.len()).then_with(|| a.cmp(b))
}

pub fn safe_relative(root: &Path, relative: &str) -> Result<PathBuf, String> {
    // Reject Windows drive/UNC/backslash escapes even when validating on Linux.
    if relative.is_empty() || relative.contains('\\') || relative.contains(':') || Path::new(relative).components().any(|c| !matches!(c, Component::Normal(_))) {
        return Err(format!("unsafe relative path: {relative}"));
    }
    let path = root.join(relative).canonicalize().map_err(|e| format!("{relative}: {e}"))?;
    if !path.starts_with(root) { return Err(format!("path escapes character root: {relative}")); }
    Ok(path)
}
fn discover_frames(root: &Path, dir: &str, expected: Option<(u32, u32)>) -> Result<(Vec<String>, (u32, u32)), String> {
    let directory = safe_relative(root, dir)?;
    let mut files = Vec::new();
    for entry in fs::read_dir(&directory).map_err(|e| e.to_string())? {
        let path = entry.map_err(|e| e.to_string())?.path();
        if !path.extension().is_some_and(|s| s.eq_ignore_ascii_case("png")) { continue; }
        let canonical = path.canonicalize().map_err(|e| e.to_string())?;
        if !canonical.starts_with(root) { return Err(format!("frame escapes character root: {}", path.display())); }
        if !canonical.is_file() { continue; }
        files.push((path.file_name().ok_or("invalid frame name")?.to_string_lossy().into_owned(), canonical));
    }
    files.sort_by(|a,b| natural_cmp(&a.0, &b.0));
    if files.is_empty() { return Err(format!("{dir}: no PNG frames")); }
    // The first fallback frame supplies the canvas size for the entire pet.
    let mut reader = image::ImageReader::open(&files[0].1).map_err(|e| e.to_string())?;
    reader.set_format(image::ImageFormat::Png);
    let dimensions = reader.into_dimensions().map_err(|e| format!("{dir}: invalid PNG: {e}"))?;
    let (width, height) = expected.unwrap_or(dimensions);
    if width == 0 || height == 0 || width > 4096 || height > 4096 {
        return Err(format!("{dir}: PNG dimensions must be 1..4096"));
    }
    // Bound decoded memory to keep malformed or unexpectedly huge packs recoverable.
    let bytes = u64::from(width) * u64::from(height) * 4 * files.len() as u64;
    if bytes > 512 * 1024 * 1024 { return Err(format!("{dir}: decoded sequence exceeds 512 MiB")); }
    let frames = files.into_iter().map(|(_, path)| {
        let mut reader = image::ImageReader::open(&path).map_err(|e| e.to_string())?;
        reader.set_format(image::ImageFormat::Png);
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(width);
        limits.max_image_height = Some(height);
        reader.limits(limits);
        let frame = reader.decode().map_err(|e| format!("{}: invalid PNG: {e}", path.display()))?;
        if frame.width() != width || frame.height() != height { return Err(format!("{}: frame size must match fallback PNG {}x{}", path.display(), width, height)); }
        path.to_str().map(String::from).ok_or_else(|| "frame path is not valid Unicode".into())
    }).collect::<Result<Vec<_>, String>>()?;
    Ok((frames, (width, height)))
}
fn warning(warnings: &mut Vec<String>, message: String) { log::warn!("{message}"); warnings.push(message); }

pub fn load_pet(root: &Path, warnings: &mut Vec<String>) -> Result<LoadedPet, String> {
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    let manifest = safe_relative(&root, "pet.json")?;
    if fs::metadata(&manifest).map_err(|e| e.to_string())?.len() > 1024 * 1024 { return Err("pet.json exceeds 1 MiB".into()); }
    let mut definition: PetDefinition = serde_json::from_slice(&fs::read(manifest).map_err(|e| e.to_string())?).map_err(|e| format!("invalid JSON/manifest: {e}"))?;
    definition.validate()?;
    let mut clips = BTreeMap::new();
    let mut dimensions = None;
    let fallback = definition.animations.get_key_value(&definition.fallback_animation).ok_or("missing fallback animation")?;
    let animations = std::iter::once(fallback).chain(definition.animations.iter().filter(|(name, _)| *name != &definition.fallback_animation));
    for (name, animation) in animations {
        let result = match &animation.source {
            AnimationSource::PngSequence { dir } => discover_frames(&root, dir, dimensions),
            AnimationSource::Unsupported => Err("Unsupported animation source type".into()),
        };
        match result {
            Ok((frames, size)) => {
                dimensions = Some(size);
                definition.render.canvas_width = size.0;
                definition.render.canvas_height = size.1;
                log::info!("Animation {name}: {} frames", frames.len());
                clips.insert(name.clone(), LoadedAnimation { definition: animation.clone(), frames });
            }
            Err(e) => {
                warning(warnings, format!("Pet {}: skipped animation {name} - {e}", definition.id));
                if name == &definition.fallback_animation { return Err("fallback animation could not be loaded".into()); }
            }
        }
    }
    if !clips.contains_key(&definition.fallback_animation) { return Err("fallback animation could not be loaded".into()); }
    let total_frames: u64 = clips.values().map(|clip| clip.frames.len() as u64).sum();
    if total_frames * u64::from(definition.render.canvas_width) * u64::from(definition.render.canvas_height) * 5 > 512 * 1024 * 1024 {
        return Err("decoded pack (RGBA + alpha cache) exceeds 512 MiB; reduce frame count or resolution".into());
    }
    definition.animations.retain(|name, _| clips.contains_key(name));
    definition.behaviors.retain_mut(|behavior| {
        behavior.choose.retain(|choice| {
            let valid = clips.contains_key(&choice.animation) && choice.weight.is_finite() && choice.weight > 0.0;
            if !valid { warning(warnings, format!("Pet {} behavior {}: missing animation referenced by behavior or invalid weight: {}", definition.id, behavior.id, choice.animation)); }
            valid
        });
        if behavior.choose.is_empty() { warning(warnings, format!("Pet {}: skipped behavior {} - no valid weighted choices", definition.id, behavior.id)); return false; }
        true
    });
    Ok(LoadedPet { definition, clips })
}

pub fn scan(pets_dir: &Path) -> Result<Catalog, String> {
    let root = pets_dir.canonicalize().map_err(|e| e.to_string())?;
    let mut directories = Vec::new();
    let mut catalog = Catalog::default();
    for entry in fs::read_dir(&root).map_err(|e| e.to_string())? {
        match entry { Ok(e) if e.path().is_dir() => directories.push(e.path()), Ok(_) => {}, Err(e) => warning(&mut catalog.warnings, e.to_string()) }
    }
    directories.sort();
    log::info!("Found {} pet directories", directories.len());
    let mut ids = HashSet::new();
    for directory in directories {
        let result = directory.canonicalize().map_err(|e| e.to_string()).and_then(|p| {
            if !p.starts_with(&root) { return Err("character directory escapes pets root".into()); }
            load_pet(&p, &mut catalog.warnings)
        });
        match result {
            Ok(pet) if ids.insert(pet.definition.id.clone()) => { log::info!("Loaded pet: {}", pet.definition.id); catalog.pets.push(pet); }
            Ok(pet) => warning(&mut catalog.warnings, format!("Skipped pet: {} - duplicate id", pet.definition.id)),
            Err(e) => warning(&mut catalog.warnings, format!("Skipped pet: {} - {e}", directory.display())),
        }
    }
    log::info!("Reload completed: {} valid pets", catalog.pets.len());
    Ok(catalog)
}
