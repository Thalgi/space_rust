use serde::{Deserialize, Serialize};

const FICHIER_PRESETS: &str = "presets.json";

/// Un système sauvegardé : un nom + la graine qui le reconstruit à l'identique.
#[derive(Clone, Serialize, Deserialize)]
pub struct PresetSauve {
    pub nom: String,
    pub graine: u64,
}

/// Charge les presets depuis le JSON (liste vide si absent/illisible).
pub fn charger_presets() -> Vec<PresetSauve> {
    std::fs::read_to_string(FICHIER_PRESETS)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// Écrit les presets dans le JSON.
pub fn sauver_presets(presets: &[PresetSauve]) {
    if let Ok(s) = serde_json::to_string_pretty(presets) {
        let _ = std::fs::write(FICHIER_PRESETS, s);
    }
}
