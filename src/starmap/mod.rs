//! Modèle de la Starmap (vue galactique) : voisinage stellaire + projection oblique.
//! Voir `CONCEPTION_STARMAP.md`. Le rendu (grille, tiges, glyphes) vit dans `rendu.rs`.

mod rendu;
pub use rendu::dessiner;

use macroquad::prelude::*;

/// Destination in-game d'une étoile : ce que la Skymap chargera au clic.
#[derive(Clone, Copy)]
pub enum Destination {
    Solaire,
    Proxima,
    AlphaCentauri,
    TauCeti,
    Graine(u64),
}

/// Une étoile du voisinage : identité + coordonnées galactiques (l, b, d) + type visuel.
pub struct Etoile {
    pub nom: &'static str,
    pub l: f32,    // longitude galactique (deg) — direction dans le plan
    pub b: f32,    // latitude galactique (deg) — au-dessus / en-dessous du plan
    pub d: f32,    // distance au Soleil (années-lumière)
    pub temp: f32, // température corps noir (K) -> couleur du glyphe
    pub rayon: f32, // rayon visuel du glyphe (repère de type, PAS la hauteur)
    pub dest: Destination,
    pub double: bool,   // système binaire -> un seul point + marqueur « double »
    pub decalage: Vec2, // léger décalage d'AFFICHAGE (px) pour dé-chevaucher deux glyphes proches
}

impl Etoile {
    /// Position 3D dans le repère galactique local (Soleil à l'origine), en al.
    /// x, y dans le plan ; z = hauteur hors du plan (la « tige »).
    pub fn position(&self) -> Vec3 {
        let l = self.l.to_radians();
        let b = self.b.to_radians();
        vec3(
            self.d * b.cos() * l.cos(),
            self.d * b.cos() * l.sin(),
            self.d * b.sin(),
        )
    }

    /// Classe spectrale approchée déduite de la température (repère pour le panneau info).
    pub fn classe(&self) -> &'static str {
        match self.temp {
            t if t >= 30000.0 => "O",
            t if t >= 10000.0 => "B",
            t if t >= 7500.0 => "A",
            t if t >= 6000.0 => "F",
            t if t >= 5200.0 => "G",
            t if t >= 3700.0 => "K",
            _ => "M",
        }
    }
}

/// Le voisinage stellaire (§4 de CONCEPTION_STARMAP.md). (l, b) approximatifs.
/// Proxima et Alpha Cen se superposent presque (4.25 vs 4.37 al) -> `decalage` d'affichage
/// opposé pour les rendre lisibles. Alpha Cen et Sirius sont binaires (`double`).
pub fn voisinage() -> Vec<Etoile> {
    use Destination::*;
    let z = Vec2::ZERO;
    vec![
        Etoile { nom: "Soleil",          l: 0.0,   b: 0.0,   d: 0.0,  temp: 5800.0, rayon: 7.0, dest: Solaire,       double: false, decalage: z },
        Etoile { nom: "Proxima Cen",     l: 313.0, b: -1.9,  d: 4.25, temp: 3040.0, rayon: 4.0, dest: Proxima,       double: false, decalage: vec2(-22.0, 14.0) },
        Etoile { nom: "Alpha Cen",       l: 316.0, b: -0.7,  d: 4.37, temp: 5790.0, rayon: 6.0, dest: AlphaCentauri, double: true,  decalage: vec2(10.0, -6.0) },
        Etoile { nom: "Barnard",         l: 31.0,  b: 14.0,  d: 5.96, temp: 3130.0, rayon: 4.0, dest: Graine(101),   double: false, decalage: z },
        Etoile { nom: "Wolf 359",        l: 244.0, b: 56.0,  d: 7.86, temp: 2800.0, rayon: 3.5, dest: Graine(359),   double: false, decalage: z },
        Etoile { nom: "Lalande 21185",   l: 185.0, b: 65.0,  d: 8.31, temp: 3550.0, rayon: 4.5, dest: Graine(211),   double: false, decalage: z },
        Etoile { nom: "Sirius",          l: 227.0, b: -8.9,  d: 8.60, temp: 9940.0, rayon: 6.5, dest: Graine(1),     double: true,  decalage: z },
        Etoile { nom: "Epsilon Eridani", l: 196.0, b: -48.0, d: 10.5, temp: 5080.0, rayon: 5.0, dest: Graine(82),    double: false, decalage: z },
        Etoile { nom: "Tau Ceti",        l: 173.0, b: -73.0, d: 11.9, temp: 5340.0, rayon: 5.0, dest: TauCeti,       double: false, decalage: z },
        Etoile { nom: "Epsilon Indi",    l: 336.0, b: -48.0, d: 11.9, temp: 4560.0, rayon: 4.5, dest: Graine(45),    double: false, decalage: z },
    ]
}

/// Projection oblique 2D fixe (§6.A) : le plan (x, y) devient un sol en diagonale
/// (base isométrique) ; z pousse le glyphe vers le haut, avec exagération `kz`.
pub struct Projection {
    pub origine: Vec2, // pixel de l'origine (le Soleil)
    pub echelle: f32,  // pixels par année-lumière
    pub kz: f32,       // exagération verticale (hauteur lisible)
}

impl Projection {
    pub fn project(&self, p: Vec3) -> Vec2 {
        // Base oblique : +x et +y forment un losange (~isométrique), z = hauteur (vers le haut).
        let ex = vec2(0.92, 0.42);
        let ey = vec2(-0.92, 0.42);
        self.origine + (p.x * ex + p.y * ey) * self.echelle + vec2(0.0, -p.z * self.kz * self.echelle)
    }

    /// Pied d'une étoile : sa projection au sol (z = 0).
    pub fn pied(&self, p: Vec3) -> Vec2 {
        self.project(vec3(p.x, p.y, 0.0))
    }
}
