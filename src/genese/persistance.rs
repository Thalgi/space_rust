use super::Habitabilite;
use crate::planete::Apparence;
use serde::{Deserialize, Serialize};

const FICHIER_PRESETS: &str = "presets.json";
const FICHIER_EDITS: &str = "catalogue_edits.json";

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

/// Un preset du catalogue édité par l'utilisateur (clé = `id`, le nom d'origine).
/// On sérialise l'apparence complète + les métadonnées modifiées.
#[derive(Clone, Serialize, Deserialize)]
pub struct PresetEdit {
    pub id: String,
    pub nom: String,
    pub habitabilite: Habitabilite,
    pub rare: bool,
    pub apparence: Apparence,
}

/// Charge les edits du catalogue (liste vide si absent/illisible).
pub fn charger_edits() -> Vec<PresetEdit> {
    std::fs::read_to_string(FICHIER_EDITS)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// Écrit les edits du catalogue dans le JSON.
pub fn sauver_edits(edits: &[PresetEdit]) {
    if let Ok(s) = serde_json::to_string_pretty(edits) {
        let _ = std::fs::write(FICHIER_EDITS, s);
    }
}
