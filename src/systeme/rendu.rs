use super::Systeme;
use crate::astre::{CameraInfo, Categorie, Foyer};
use macroquad::prelude::*;

impl Systeme {
    pub fn draw(
        &mut self,
        cam: &CameraInfo,
        orbites_planetes: bool,
        orbites_etoiles: bool,
        montrer_zone: bool,
    ) {
        // L'étoile sert de source de lumière pour éclairer les planètes.
        let etoile = self
            .astres
            .iter()
            .find(|a| a.categorie() == Categorie::Etoile);
        // Étoile présente -> elle éclaire. Sinon, lumière de secours (planète isolée).
        let secours = self.lumiere_secours();
        let light = etoile
            .map(|a| a.corps().position)
            .or(secours.map(|(p, _)| p))
            .unwrap_or(Vec3::ZERO);
        let light_color = etoile
            .and_then(|a| a.lumiere())
            .or(secours.map(|(_, c)| c))
            .unwrap_or(Vec3::ONE);

        // Zone habitable — multi-étoile :
        //  - CIRCUMSTELLAIRE (type S) : la HZ propre de chaque étoile, centrée sur elle.
        //  - CIRCUMBINAIRE (type P) : la HZ de la luminosité TOTALE autour du barycentre,
        //    dessinée seulement si elle tombe nettement hors des orbites stellaires.
        if montrer_zone {
            let vert = Color::new(0.3, 0.9, 0.4, 0.45);
            let mut l_tot = 0.0f32;
            let mut nb_et = 0usize;
            let mut r_max = 0.0f32;
            for a in self.astres.iter() {
                if a.categorie() != Categorie::Etoile {
                    continue;
                }
                if let Some((interne, externe)) = a.zone_viable() {
                    let c = a.corps().position;
                    dessiner_cercle(c, interne, vert);
                    dessiner_cercle(c, externe, vert);
                }
                if let Some(l) = a.luminosite() {
                    l_tot += l;
                }
                nb_et += 1;
                r_max = r_max.max(a.corps().position.length());
            }
            if nb_et >= 2 {
                let (i, o) = crate::etoile::zone_habitable(l_tot);
                let (iw, ow) = (i * crate::etoile::UA, o * crate::etoile::UA);
                if iw > 2.0 * r_max {
                    let cyan = Color::new(0.3, 0.85, 0.85, 0.4);
                    dessiner_cercle(Vec3::ZERO, iw, cyan);
                    dessiner_cercle(Vec3::ZERO, ow, cyan);
                }
            }
        }

        // Trajectoires des PLANÈTES (ellipses pleines), centrées sur LEUR foyer
        // (étoile hôte S-type, ou barycentre P-type).
        if orbites_planetes {
            let col = Color::new(0.4, 0.5, 0.75, 0.6);
            for a in &self.astres {
                let pts = a.orbite();
                if pts.is_empty() {
                    continue;
                }
                let centre = match a.foyer() {
                    Some(Foyer::Etoile(i)) => {
                        self.astres.get(i).map_or(Vec3::ZERO, |s| s.corps().position)
                    }
                    Some(Foyer::Barycentre) => Vec3::ZERO,
                    None => light,
                };
                for i in 0..pts.len() {
                    draw_line_3d(centre + pts[i], centre + pts[(i + 1) % pts.len()], col);
                }
            }
        }

        // Orbites des ÉTOILES (systèmes multiples) : pointillés, teinte stellaire.
        if orbites_etoiles {
            if let Some(arbre) = &self.arbre {
                let col_et = Color::new(0.85, 0.8, 0.55, 0.32); // plus discret
                for poly in arbre.orbites_etoiles(self.temps) {
                    dessiner_pointilles(&poly, col_et);
                }
            }
        }

        // Éclairage multi-source : jusqu'à 4 étoiles ([0] = primaire = light).
        let mut lights_pos = [Vec3::ZERO; 4];
        let mut lights_color = [Vec3::ZERO; 4];
        let mut nb = 0usize;
        for a in self.astres.iter() {
            if nb >= 4 {
                break;
            }
            if a.categorie() == Categorie::Etoile {
                if let Some(col) = a.lumiere() {
                    lights_pos[nb] = a.corps().position;
                    lights_color[nb] = col;
                    nb += 1;
                }
            }
        }
        if nb == 0 {
            // Aucune étoile : lumière de secours en source unique.
            lights_pos[0] = light;
            lights_color[0] = light_color;
        }

        let mut c = *cam;
        c.light_pos = light;
        c.light_color = light_color;
        c.lights_pos = lights_pos;
        c.lights_color = lights_color;
        for a in &mut self.astres {
            a.draw(&c);
        }
    }
}

/// Trace une polyligne fermée en pointillés fins : on ne dessine qu'une courte
/// fraction (`FRAC`) de chaque segment -> tirets courts, espacés régulièrement.
fn dessiner_pointilles(poly: &[Vec3], col: Color) {
    const FRAC: f32 = 0.4; // longueur du tiret (fraction du segment) ; le reste = vide
    let n = poly.len();
    for i in 0..n {
        let a = poly[i];
        let b = poly[(i + 1) % n];
        draw_line_3d(a, a + (b - a) * FRAC, col);
    }
}

/// Trace un cercle dans le plan xz (orbite ou zone habitable).
fn dessiner_cercle(centre: Vec3, r: f32, col: Color) {
    let n = 96;
    let mut prec = centre + vec3(r, 0.0, 0.0);
    for i in 1..=n {
        let a = i as f32 / n as f32 * std::f32::consts::TAU;
        let p = centre + vec3(r * a.cos(), 0.0, r * a.sin());
        draw_line_3d(prec, p, col);
        prec = p;
    }
}
