mod gravite;
mod rendu;

use crate::astre::{Astre, Categorie, Foyer};
use crate::stellaire::ArbreStellaire;
use macroquad::prelude::*;

pub const G: f32 = 1.0; // constante gravitationnelle
const SOUS_PAS: usize = 4; // sous-pas de physique par frame (stabilité)

/// Mode de propagation des **planètes**.
/// - `SurRails` (défaut) : orbites de Kepler analytiques — stable, déterministe.
/// - `NCorps` : intégration gravitationnelle — dynamique émergente « bac à sable ».
/// Les étoiles et les lunes restent toujours analytiques, quel que soit le mode.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ModePhysique {
    SurRails,
    NCorps,
}

/// Le système : un ensemble d'astres soumis à la gravité mutuelle.
pub struct Systeme {
    astres: Vec<Box<dyn Astre>>,
    /// Lumière de secours (pos, couleur) utilisée quand il n'y a pas d'étoile
    /// (ex. vue d'une planète isolée). Ignorée dès qu'une étoile est présente.
    lumiere_manuelle: Option<(Vec3, Vec3)>,
    /// Temps de simulation cumulé (secondes) : sert à l'évaluation analytique `f(t)`.
    temps: f64,
    /// Mode de propagation des planètes (voir `ModePhysique`).
    mode: ModePhysique,
    /// Arbre stellaire hiérarchique (systèmes multiples). Absent = étoile unique fixe.
    arbre: Option<ArbreStellaire>,
    /// Vue par défaut suggérée : (astre à focaliser, distance caméra). Utilisé pour
    /// cadrer d'emblée la zone planétaire d'un système type S (trop étalé sinon).
    vue: Option<(usize, f32)>,
}

impl Systeme {
    pub fn new() -> Self {
        Self {
            astres: Vec::new(),
            lumiere_manuelle: None,
            temps: 0.0,
            mode: ModePhysique::SurRails,
            arbre: None,
            vue: None,
        }
    }

    /// Installe l'arbre stellaire hiérarchique (systèmes multiples).
    pub fn definir_arbre(&mut self, arbre: ArbreStellaire) {
        self.arbre = Some(arbre);
    }

    /// Définit la vue par défaut : focaliser l'astre `idx` à la distance `dist`.
    pub fn definir_vue(&mut self, idx: usize, dist: f32) {
        self.vue = Some((idx, dist));
    }

    /// Vue par défaut suggérée, si présente : (astre à focaliser, distance caméra).
    pub fn vue(&self) -> Option<(usize, f32)> {
        self.vue
    }

    /// Rayon englobant approximatif (unités monde), pour cadrer la caméra sur un
    /// système dont l'étendue varie beaucoup (mono-étoile compact ↔ multiple large).
    pub fn rayon_englobant(&self) -> f32 {
        let mut r: f32 = 1.0;
        for a in &self.astres {
            r = r.max(a.corps().position.length());
            for p in a.orbite() {
                r = r.max(p.length());
            }
        }
        if let Some(arbre) = &self.arbre {
            for poly in arbre.orbites_etoiles(self.temps) {
                for p in &poly {
                    r = r.max(p.length());
                }
            }
        }
        r
    }

    /// Mode de propagation courant.
    pub fn mode(&self) -> ModePhysique {
        self.mode
    }

    /// Change le mode. Le passage vers N-corps amorce les vitesses des planètes
    /// depuis leur orbite analytique (hand-off) ; le retour sur rails resnappe
    /// naturellement à la frame suivante. Idempotent (no-op si mode identique).
    pub fn regler_mode(&mut self, m: ModePhysique) {
        if m == self.mode {
            return;
        }
        if m == ModePhysique::NCorps {
            // Amorce chaque planète depuis son foyer (étoile hôte ou barycentre).
            let pos: Vec<Vec3> = self.astres.iter().map(|a| a.corps().position).collect();
            let t = self.temps;
            for a in &mut self.astres {
                let f = match a.foyer() {
                    Some(Foyer::Etoile(i)) => pos.get(i).copied().unwrap_or(Vec3::ZERO),
                    _ => Vec3::ZERO, // Barycentre / pas de foyer
                };
                a.amorcer_ncorps(f, Vec3::ZERO, t); // vitesse du foyer approximée à 0 (sandbox)
            }
        }
        self.mode = m;
    }

    /// Définit une lumière directionnelle de secours (sans étoile dans la scène).
    pub fn set_lumiere(&mut self, pos: Vec3, couleur: Vec3) {
        self.lumiere_manuelle = Some((pos, couleur));
    }

    pub(crate) fn lumiere_secours(&self) -> Option<(Vec3, Vec3)> {
        self.lumiere_manuelle
    }

    /// Ajoute un astre et renvoie son index (utile pour rattacher des lunes).
    pub fn ajouter(&mut self, a: Box<dyn Astre>) -> usize {
        self.astres.push(a);
        self.astres.len() - 1
    }

    /// Nombre de lunes déjà attachées à l'astre `parent`. Sert à `ajouter_lune`
    /// pour placer chaque nouvelle lune sur un créneau orbital croissant (système
    /// emboîté, sans chevauchement) plutôt qu'à un rayon aléatoire indépendant.
    pub fn nb_lunes(&self, parent: usize) -> usize {
        self.astres.iter().filter(|a| a.parent() == Some(parent)).count()
    }

    /// Position d'un astre par index (origine si invalide).
    pub fn position(&self, idx: usize) -> Vec3 {
        self.astres
            .get(idx)
            .map(|a| a.corps().position)
            .unwrap_or(Vec3::ZERO)
    }

    /// Sélection au rayon : renvoie l'astre touché le plus proche (hors ceinture).
    pub fn pick(&self, origine: Vec3, dir: Vec3) -> Option<usize> {
        let mut best: Option<(f32, usize)> = None;
        for (i, a) in self.astres.iter().enumerate() {
            if a.categorie() == Categorie::Asteroide {
                continue;
            }
            let centre = a.corps().position;
            let rayon = a.corps().rayon.max(0.3) * 1.4; // marge pour cliquer facilement
            let oc = centre - origine;
            let tca = oc.dot(dir);
            if tca < 0.0 {
                continue; // derrière la caméra
            }
            let d2 = oc.length_squared() - tca * tca;
            let rr = rayon * rayon;
            if d2 <= rr {
                let t = tca - (rr - d2).sqrt();
                if best.map_or(true, |(bt, _)| t < bt) {
                    best = Some((t, i));
                }
            }
        }
        best.map(|(_, i)| i)
    }

    pub fn update(&mut self, dt: f32) {
        self.temps += dt as f64;
        let t = self.temps;

        // Étoiles : positions issues de l'arbre stellaire hiérarchique (barycentres
        // composés). TOUJOURS analytique, quel que soit le mode -> binaires/multiples
        // stables. Étoile unique = pas d'arbre -> reste à sa position (origine).
        if let Some(arbre) = self.arbre.take() {
            arbre.evaluer(t, &mut self.astres);
            self.arbre = Some(arbre);
        }

        match self.mode {
            // Planètes « sur rails » : chacune orbite son foyer (étoile hôte S-type
            // ou barycentre P-type). Étoiles déjà repositionnées juste au-dessus.
            ModePhysique::SurRails => {
                let pos: Vec<Vec3> = self.astres.iter().map(|a| a.corps().position).collect();
                for a in &mut self.astres {
                    if a.categorie() == Categorie::Planete {
                        let f = match a.foyer() {
                            Some(Foyer::Etoile(i)) => pos.get(i).copied().unwrap_or(Vec3::ZERO),
                            _ => Vec3::ZERO, // Barycentre
                        };
                        a.maj_rail(f, t);
                    }
                }
            }
            // Planètes en N-corps : intégration (elles ressentent les étoiles mobiles).
            ModePhysique::NCorps => {
                let h = dt / SOUS_PAS as f32;
                for _ in 0..SOUS_PAS {
                    self.gravite(h);
                }
            }
        }

        // Animation propre de chaque astre (éruptions du soleil, etc.).
        for a in &mut self.astres {
            a.update(dt);
        }

        // Lunes : orbite analytique autour de leur parent (positions courantes).
        let pos: Vec<Vec3> = self.astres.iter().map(|a| a.corps().position).collect();
        for a in &mut self.astres {
            if let Some(p) = a.parent() {
                a.orbiter_autour(pos[p], dt);
            }
        }
    }

    /// Transmet les réglages d'éruptions à l'étoile.
    pub fn reglages_etoile(&mut self, freq: f32, forme: f32, puissance: f32, alea: f32) {
        for a in &mut self.astres {
            if a.categorie() == Categorie::Etoile {
                a.set_eruptions(freq, forme, puissance, alea);
            }
        }
    }
}
