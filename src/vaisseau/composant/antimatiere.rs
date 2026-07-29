//! **Bloc propulsion à antimatière**, deux briques chaînées : le réacteur
//! (cuve, bobines EM, tuyauterie, pièges à antiprotons) en amont, la tuyère
//! (buse + cage de confinement, jet de plasma) en aval.

use crate::vaisseau::peintre::Peintre;
use crate::vaisseau::{GenrePort, Port, Profil, Repere};
use macroquad::prelude::*;
use std::f32::consts::{PI, TAU};


// --- Tuyère ----------------------------------------------------------------

/// Unique écoutille axiale de montage en tête (avant +Z, vers le corps
/// porteur) ; le moteur pousse dans le sens opposé (−Z).
pub(super) fn moteur_ports(profil: Profil) -> Vec<Port> {
    vec![Port::new(Repere::new(Vec3::ZERO, Quat::IDENTITY), GenrePort::ModuleAxial, profil)]
}

pub(super) fn moteur_dessiner<P: Peintre>(p: &mut P, taille: f32) {
        // Silhouette VASIMR **gonflée** : corps de confinement magnétique
        // le long de −Z, anneaux de bobines cuivre, cœur d'annihilation
        // émissif, tuyère magnétique évasée et long jet de plasma.
        let metal = Color::new(0.62, 0.64, 0.68, 1.0);
        let sombre = Color::new(0.20, 0.22, 0.26, 1.0);
        let clair = Color::new(0.80, 0.82, 0.86, 1.0);
        let cuivre = Color::new(0.66, 0.45, 0.28, 1.0);
        let coeur = Color::new(0.90, 0.55, 1.0, 1.0); // annihilation e⁺/e⁻ (violet)
        let (d, w, h) = (Vec3::NEG_Z, Vec3::X, Vec3::Y);
        let t = taille;
        // Collier structurel clair en tête (côté montage).
        p.cylindre(Vec3::ZERO - d * (t * 0.04), d * (t * 0.10), t * 0.42, clair);
        // Corps de confinement magnétique (fût central sombre).
        p.cylindre(Vec3::ZERO, d * (t * 1.10), t * 0.22, sombre);
        // Cœur d'annihilation émissif au centre du fût.
        p.cylindre(d * (t * 0.28), d * (t * 0.72), t * 0.11, coeur);
        // Six anneaux de bobines de confinement (plus qu'un VASIMR).
        for k in 0..6 {
            let z = t * (0.14 + 0.16 * k as f32);
            p.cylindre(d * (z - t * 0.05), d * (z + t * 0.05), t * 0.36, cuivre);
        }
        // **Buse de sortie** : court cylindre (col) au bout du corps.
        p.cylindre(d * (t * 1.02), d * (t * 1.24), t * 0.20, metal);
        // **Structure de stabilisation finale** : deux cercles ouverts
        // (anneaux polygonaux, non pleins) tenus par **4 tiges** — le
        // dernier étage où le plasma est encore contraint avant de partir.
        let rr = t * 0.30; // rayon des anneaux
        let (z1, z2) = (t * 1.34, t * 1.60); // positions axiales des cercles
        let seg = 20usize; // finesse du cercle
        let fil = t * 0.028; // section du fil d'anneau
        for &z in &[z1, z2] {
            let c = d * z;
            for i in 0..seg {
                let a0 = TAU * i as f32 / seg as f32;
                let a1 = TAU * (i + 1) as f32 / seg as f32;
                let p0 = c + (w * a0.cos() + h * a0.sin()) * rr;
                let p1 = c + (w * a1.cos() + h * a1.sin()) * rr;
                p.cylindre(p0, p1, fil, metal);
            }
        }
        // 4 tiges longitudinales : elles courent depuis la **buse du
        // propulseur** (ancrage) jusqu'au second cercle, tenant les deux
        // anneaux au passage.
        let z0 = t * 1.06; // ancrage sur la buse
        for k in 0..4 {
            let a = TAU * k as f32 / 4.0;
            let dir = w * a.cos() + h * a.sin();
            p.cylindre(d * z0 + dir * rr, d * z2 + dir * rr, fil * 1.4, metal);
            // Patte de fixation radiale sur la buse.
            p.cylindre(d * z0 + dir * rr, d * z0 + dir * (t * 0.20), fil * 1.4, metal);
        }
}

pub(super) fn moteur_cout() -> f32 {
    11.0
}

pub(super) fn moteur_rayon_local(taille: f32) -> f32 {
    taille * 1.62
}

/// Masse déployée vers l'arrière (−Z), comme un propulseur axial : la sphère
/// est décalée à mi-corps, sinon elle mordrait sur les voisins.
pub(super) fn moteur_englobant(taille: f32) -> (Vec3, f32) {
    (Vec3::NEG_Z * (taille * 0.7), taille * 1.05)
}

// --- Réacteur --------------------------------------------------------------

/// Montage vers la tuyère en −Z (base) ; chaînage d'alimentation en tête +Z.
pub(super) fn reacteur_ports(profil: Profil, taille: f32) -> Vec<Port> {
    let lb = taille * 0.95;
    vec![
        Port::new(Repere::new(Vec3::ZERO, Quat::from_rotation_y(PI)), GenrePort::ModuleAxial, profil),
        Port::new(Repere::new(vec3(0.0, 0.0, lb), Quat::IDENTITY), GenrePort::ModuleAxial, profil),
    ]
}

pub(super) fn reacteur_dessiner<P: Peintre>(p: &mut P, taille: f32) {
        // Bloc d'injection/confinement en amont de la tuyère : cuve sombre,
        // bobines électromagnétiques (cryostat), tuyauterie, injecteur et
        // pièges à antiprotons. Base en Z=0 (côté tuyère), corps vers +Z.
        let t = taille;
        let rb = t * 0.40; // rayon de la cuve
        let lb = t * 0.95; // longueur de la cuve
        let dark = Color::new(0.14, 0.15, 0.18, 1.0);
        let cuivre = Color::new(0.66, 0.45, 0.28, 1.0);
        let metal = Color::new(0.55, 0.57, 0.62, 1.0);
        let clair = Color::new(0.80, 0.82, 0.86, 1.0);
        let lueur = Color::new(0.42, 0.66, 0.95, 1.0);
        // Cuve réacteur sombre.
        p.cylindre(Vec3::ZERO, vec3(0.0, 0.0, lb), rb, dark);
        // Collier de jonction au bloc moteur (base −Z) + liseré de
        // confinement émissif (rappel du plasma de la tuyère).
        p.cylindre(Vec3::ZERO, vec3(0.0, 0.0, t * 0.05), rb * 1.05, metal);
        p.cylindre(vec3(0.0, 0.0, t * 0.01), vec3(0.0, 0.0, t * 0.03), rb * 1.01, lueur);
        // 4 bobines électromagnétiques : anneau de cuivre serré entre deux
        // flasques métal (aspect cryostat de bobine supraconductrice).
        for k in 0..4 {
            let z = t * (0.20 + 0.19 * k as f32);
            p.cylindre(vec3(0.0, 0.0, z - t * 0.05), vec3(0.0, 0.0, z + t * 0.05), rb * 1.14, cuivre);
            for s in [-1.0_f32, 1.0] {
                let zf = z + s * t * 0.058;
                p.cylindre(vec3(0.0, 0.0, zf - t * 0.012), vec3(0.0, 0.0, zf + t * 0.012), rb * 1.18, metal);
            }
        }
        // Tuyauterie : 3 conduites longitudinales (hors des bobines) avec
        // vanne médiane et coudes de raccord vers la cuve.
        for s in 0..3 {
            let a = TAU * s as f32 / 3.0 + 0.5;
            let dir = vec3(a.cos(), a.sin(), 0.0);
            let (z0, z1) = (t * 0.10, lb - t * 0.10);
            let p0 = dir * (rb * 1.22) + Vec3::Z * z0;
            let p1 = dir * (rb * 1.22) + Vec3::Z * z1;
            p.cylindre(p0, p1, t * 0.03, metal);
            p.cube(dir * (rb * 1.22) + Vec3::Z * (t * 0.55), Vec3::splat(t * 0.07), clair); // vanne
            p.cylindre(p0, dir * rb + Vec3::Z * z0, t * 0.025, metal); // coude bas
            p.cylindre(p1, dir * rb + Vec3::Z * z1, t * 0.025, metal); // coude haut
        }
        // Tête : dôme d'obturation + injecteur central, flanqué de deux
        // pièges à antiprotons (petites cuves sphériques) alimentés.
        p.cone(vec3(0.0, 0.0, lb), Vec3::Z, rb, rb * 0.45, t * 0.14, dark);
        p.sphere(vec3(0.0, 0.0, lb + t * 0.14), t * 0.10, metal);
        for s in [-1.0_f32, 1.0] {
            let c = vec3(s * rb * 0.70, 0.0, lb + t * 0.02);
            p.sphere(c, t * 0.13, clair);
            p.cylindre(c, vec3(s * rb * 0.35, 0.0, lb - t * 0.05), t * 0.02, metal);
        }
}

pub(super) fn reacteur_cout() -> f32 {
    14.0
}

pub(super) fn reacteur_rayon_local(taille: f32) -> f32 {
    taille * 1.2
}

/// Masse déployée vers +Z : sphère à mi-corps.
pub(super) fn reacteur_englobant(taille: f32) -> (Vec3, f32) {
    (Vec3::Z * (taille * 0.5), taille * 0.75)
}
