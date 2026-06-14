//! LANSwitch core: types, validation, native command building, discovery, and
//! preset persistence. Shared by the unprivileged app (`src-tauri`) and the
//! privileged helper (`helper`).

pub mod commands;
pub mod discover;
pub mod presets;
pub mod types;
pub mod validate;

pub use types::*;
pub use validate::{prefix_to_mask, validate_apply, validate_interface_name, ValidationError};

/// The default presets shipped with the app, embedded at compile time so a
/// first run always has something useful even before the user customizes.
pub const DEFAULT_PRESETS_JSON: &str = include_str!("../../ui/presets.default.json");
