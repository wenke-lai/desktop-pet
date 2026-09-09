use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PetDefinition {
    pub schema_version: u32,
    pub id: String,
    pub display_name: String,
    pub render: RenderConfig,
    pub fallback_animation: String,
    pub animations: BTreeMap<String, AnimationDefinition>,
    pub behaviors: Vec<BehaviorDefinition>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RenderConfig {
    // Derived from PNG frames by the loader, never read from pet.json.
    #[serde(skip_deserializing)]
    pub canvas_width: u32,
    #[serde(skip_deserializing)]
    pub canvas_height: u32,
    pub scale: f64,
    pub anchor_x: f64,
    pub anchor_y: f64,
}
impl RenderConfig {
    pub fn validate(&self) -> Result<(), String> {
        if !self.scale.is_finite() || self.scale <= 0.0 || self.scale > 10.0 {
            return Err("render.scale must be > 0 and <= 10".into());
        }
        if !self.anchor_x.is_finite() || !self.anchor_y.is_finite() || !(0.0..=1.0).contains(&self.anchor_x) || !(0.0..=1.0).contains(&self.anchor_y) {
            return Err("anchor coordinates must be 0..1".into());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AnimationDefinition {
    pub source: AnimationSource,
    pub fps: f64,
    #[serde(rename = "loop")]
    pub looping: bool,
    pub priority: i32,
    pub interruptible: bool,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AnimationSource {
    PngSequence { dir: String },
    #[serde(other)]
    Unsupported,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BehaviorDefinition {
    pub id: String,
    pub trigger: TriggerDefinition,
    #[serde(default)]
    pub only_when: OnlyWhen,
    #[serde(default)]
    pub cooldown_ms: u32,
    pub choose: Vec<WeightedAnimation>,
}
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum OnlyWhen { #[default] Any, Fallback }
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TriggerDefinition {
    Timer { min_ms: u32, max_ms: u32 },
    CursorDistanceEnter { distance_px: f64, reset_distance_px: f64 },
    Hover { dwell_ms: u32 },
    Click { button: MouseButton },
    DragStart,
    DragEnd,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MouseButton { Left, Right, Middle }
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WeightedAnimation { pub animation: String, pub weight: f64 }

impl PetDefinition {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1 { return Err("unsupported schema_version (expected 1)".into()); }
        if self.id.trim().is_empty() || self.display_name.trim().is_empty() { return Err("id and display_name must not be empty".into()); }
        self.render.validate()?;
        if !self.animations.contains_key(&self.fallback_animation) { return Err("missing fallback animation".into()); }
        for (name, animation) in &self.animations {
            if name.trim().is_empty() { return Err("empty animation name".into()); }
            if !animation.fps.is_finite() || animation.fps <= 0.0 { return Err(format!("Animation {name}: fps must be > 0")); }
        }
        let mut ids = HashSet::new();
        for behavior in &self.behaviors {
            if behavior.id.trim().is_empty() || !ids.insert(&behavior.id) { return Err("empty or duplicate behavior id".into()); }
            match behavior.trigger {
                TriggerDefinition::Timer { min_ms, max_ms } if min_ms == 0 || max_ms < min_ms => return Err(format!("Behavior {}: invalid timer interval", behavior.id)),
                TriggerDefinition::CursorDistanceEnter { distance_px, reset_distance_px } if !distance_px.is_finite() || !reset_distance_px.is_finite() || distance_px < 0.0 || reset_distance_px <= distance_px => return Err(format!("Behavior {}: reset_distance_px must exceed distance_px >= 0", behavior.id)),
                _ => {}
            }
        }
        Ok(())
    }
}
