//! Palette paramétrique des géantes gazeuses (V2, phase 1 — voir
//! CONCEPTION_GAZEUSES_V2.md § 4).
//!
//! Contrat des trois couleurs d'`Apparence` pour les gazeuses :
//! - `couleur2` = **belt** (ceintures sombres)
//! - `couleur3` = **zone** (bandes claires)
//! - `couleur`  = **accent** (filaments chauds, traînées, cœurs de tempêtes)
//!
//! Toutes les teintes de détail du shader sont DÉRIVÉES de ces entrées (plus
//! `tache_couleur` et `g_pole`) par opérations relatives -> plus aucune couleur
//! codée en dur : une géante bleue a des filaments bleus, une naine brune un
//! équateur sombre. Le tableau part en uniform `vec3 gaz_pal[8]` (`materiau.rs`).

use super::apparence::Apparence;
use macroquad::prelude::*;

/// Rôles des entrées de `gaz_pal` (miroir des usages dans planete.frag.glsl) :
/// 0 = fond des zones claires (ex `mix(couleur3, ivoire, 0.3)`)
/// 1 = clair du preset : flocons, ovales blancs, équateur (ex ivoire/blanc)
/// 2 = filaments sombres des ceintures (ex « chocolat »)
/// 3 = filaments chauds des ceintures (ex « saumon »/« ocre »)
/// 4 = collier/sillage crème de la grande tache (ex crème absolu)
/// 5 = ceinture hôte de la tache (ex brique absolu)
/// 6 = bord de la grande tache (ex beige rosé absolu)
/// 7 = cellules claires des pôles (ex `g_pole * olive`)
pub const GAZ_PAL_N: usize = 8;

/// Désature vers la luminance (t = 0 inchangé, 1 gris).
fn desaturer(c: Vec3, t: f32) -> Vec3 {
    let l = c.dot(vec3(0.299, 0.587, 0.114));
    c.lerp(Vec3::splat(l), t)
}

/// Assombrit en re-saturant légèrement (évite le gris boueux des `c * k`).
fn foncer(c: Vec3, t: f32) -> Vec3 {
    let sombre = c * (1.0 - t);
    // Re-saturation : on éloigne un peu du gris de même luminance.
    let l = sombre.dot(vec3(0.299, 0.587, 0.114));
    (sombre + (sombre - Vec3::splat(l)) * 0.35).clamp(Vec3::ZERO, Vec3::ONE)
}

/// Éclaircit vers le blanc (désature naturellement, comme les glaces d'ammoniac).
fn eclaircir(c: Vec3, t: f32) -> Vec3 {
    c.lerp(Vec3::ONE, t)
}

/// Dérive les 8 teintes de la palette gazeuse depuis l'apparence.
pub fn gaz_palette(a: &Apparence) -> [Vec3; GAZ_PAL_N] {
    let belt = a.couleur2;
    let zone = a.couleur3;
    let accent = a.couleur;
    let clair = eclaircir(zone, 0.7); // blanc teinté du preset
    let collier = eclaircir(desaturer(zone, 0.5), 0.55);
    [
        eclaircir(zone, 0.25),                       // 0 fond des zones
        clair,                                       // 1 flocons / ovales / équateur
        foncer(belt, 0.45),                          // 2 filaments sombres
        belt.lerp(eclaircir(accent, 0.25), 0.55),    // 3 filaments chauds (accent)
        collier,                                     // 4 collier / sillage
        belt.lerp(a.tache_couleur, 0.6),             // 5 ceinture hôte de la tache
        a.tache_couleur.lerp(collier, 0.65),         // 6 bord de la grande tache
        a.g_pole.lerp(eclaircir(zone, 0.25), 0.4),   // 7 cellules polaires claires
    ]
}
