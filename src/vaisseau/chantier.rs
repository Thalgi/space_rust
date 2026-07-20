//! Constructeur incrémental de station qui suit les **ports hôtes libres** —
//! le fondement du générateur (`docs/stations_procedurales.md`, §7).
//!
//! Idée : au lieu d'assembler à la main, on tient à jour la liste des ports
//! hôtes libres (en monde). La grammaire du générateur pilote :
//! 1. choisir un port libre compatible (`compatibles`) ;
//! 2. y clipser un composant (`poser`) — le port hôte est **consommé** et les
//!    ports restants de l'enfant deviennent **libres** à leur tour ;
//! 3. répéter jusqu'à épuisement du budget / de la grammaire, puis `terminer`.
//!
//! Les transformées sont cuites en `Mat4` dans les `Piece` (couche cuite), mais
//! le chaînage garde le repère monde en `Repere` (couche construction).

use super::{accoupler, cuire, Budget, Composant, EtatStation, GenrePort, Piece, Profil, Repere, Station};

/// Tolérance de recouvrement : deux sphères englobantes ne déclenchent une
/// collision que si la distance des centres passe sous `FACTEUR × (r1 + r2)`.
/// < 1 pour autoriser l'adjacence de docking (les composants voisins se touchent).
const FACTEUR_COLLISION: f32 = 0.85;

/// Un port hôte **libre**, en coordonnées monde, prêt à recevoir un enfant.
#[derive(Clone, Copy, Debug)]
pub struct PortLibre {
    pub repere: Repere,
    pub genre: GenrePort,
    pub profil: Profil,
    /// Indice de la pièce qui expose ce port (exemptée de l'anti-collision).
    pub origine: usize,
}

/// Constructeur de station à bookkeeping des ports libres.
#[derive(Default)]
pub struct Chantier {
    pieces: Vec<Piece>,
    libres: Vec<PortLibre>,
    budget: Option<Budget>,
}

impl Chantier {
    pub fn new() -> Self {
        Self::default()
    }

    /// Constructeur avec plafond de complexité (chaque pose dépense `cout()`).
    pub fn avec_budget(total: f32) -> Self {
        Self { budget: Some(Budget::new(total)), ..Self::default() }
    }

    /// Ports hôtes libres actuels (lecture). **Les indices ne sont valides que
    /// jusqu'à la prochaine `poser`/`racine`** (retrait par `swap_remove`).
    pub fn libres(&self) -> &[PortLibre] {
        &self.libres
    }

    pub fn nb_pieces(&self) -> usize {
        self.pieces.len()
    }

    /// Pose le composant **racine** à l'origine ; tous ses ports deviennent
    /// libres. Renvoie `false` si le budget ne le couvre pas.
    pub fn racine(&mut self, comp: Composant) -> bool {
        if !self.payer(comp) {
            return false;
        }
        let corps = Repere::IDENTITE;
        self.pieces.push(cuire(corps, comp));
        let idx = self.pieces.len() - 1;
        self.ajouter_libres(corps, comp, None, idx);
        true
    }

    /// Clipse `comp` (par son port de montage `montage_idx`) sur le port libre
    /// d'indice `hote_idx`. Vérifie **compatibilité** (genre + profil) puis
    /// **budget**. Renvoie `true` si posé ; le port hôte est alors consommé et
    /// les autres ports de l'enfant ajoutés aux libres.
    pub fn poser(&mut self, hote_idx: usize, comp: Composant, montage_idx: usize) -> bool {
        let hote = match self.libres.get(hote_idx) {
            Some(h) => *h,
            None => return false,
        };
        let ports = comp.ports();
        let montage = match ports.get(montage_idx) {
            Some(p) => *p,
            None => return false,
        };
        if !hote.genre.compatible(montage.genre) || !hote.profil.compatible(montage.profil) {
            return false;
        }
        let corps = accoupler(hote.repere, montage.repere);
        // Anti-collision : rejette si l'enfant recouvre trop une pièce **autre**
        // que son hôte direct (qu'il est censé toucher au joint).
        if self.collision(corps, comp, hote.origine) {
            return false;
        }
        if !self.payer(comp) {
            return false;
        }
        self.pieces.push(cuire(corps, comp));
        self.libres.swap_remove(hote_idx); // port consommé
        let idx = self.pieces.len() - 1;
        self.ajouter_libres(corps, comp, Some(montage_idx), idx);
        true
    }

    /// Indices des ports libres compatibles avec le port `montage_idx` de `comp`
    /// (genre + profil). Sert à la grammaire pour choisir où poser.
    pub fn compatibles(&self, comp: Composant, montage_idx: usize) -> Vec<usize> {
        let ports = comp.ports();
        let m = match ports.get(montage_idx) {
            Some(p) => *p,
            None => return Vec::new(),
        };
        self.libres
            .iter()
            .enumerate()
            .filter(|(_, h)| h.genre.compatible(m.genre) && h.profil.compatible(m.profil))
            .map(|(i, _)| i)
            .collect()
    }

    /// Publie la station (immuable). `Vide` si rien n'a été posé.
    pub fn terminer(self) -> EtatStation {
        match Station::depuis_pieces(self.pieces) {
            Some(s) => EtatStation::Prete(s),
            None => EtatStation::Vide,
        }
    }

    // ---- interne ----

    fn payer(&mut self, comp: Composant) -> bool {
        match &mut self.budget {
            Some(b) => b.depenser(comp.cout()),
            None => true,
        }
    }

    /// L'enfant (`comp` placé en `corps`) recouvre-t-il trop une pièce **autre**
    /// que son hôte `hote_piece` ? Sphères englobantes géométriques + tolérance.
    fn collision(&self, corps: Repere, comp: Composant, hote_piece: usize) -> bool {
        let (c_local, r) = comp.englobant_local();
        let centre = corps.transforme_point(c_local);
        self.pieces.iter().enumerate().any(|(i, p)| {
            if i == hote_piece {
                return false;
            }
            let (pc, pr) = p.composant.englobant_local();
            let pcentre = p.transforme.transform_point3(pc);
            centre.distance(pcentre) < FACTEUR_COLLISION * (r + pr)
        })
    }

    /// Ajoute les ports de `comp` (placé en `corps`, pièce d'indice `origine`)
    /// comme libres, sauf celui consommé (`sauf`).
    fn ajouter_libres(&mut self, corps: Repere, comp: Composant, sauf: Option<usize>, origine: usize) {
        for (i, p) in comp.ports().into_iter().enumerate() {
            if Some(i) == sauf {
                continue;
            }
            let monde = corps.compose(p.repere);
            self.libres.push(PortLibre { repere: monde, genre: p.genre, profil: p.profil, origine });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{StyleTreillis, VarianteModule, VariantePanneau};
    use super::*;

    fn module() -> Composant {
        Composant::ModuleAxial { profil: Profil::P1, variante: VarianteModule::Standard, longueur: 2.0 }
    }
    fn panneau() -> Composant {
        Composant::PanneauSolaire { profil: Profil::P0, variante: VariantePanneau::RigideUS, longueur: 2.0, largeur: 1.0 }
    }

    #[test]
    fn chantier_vide_donne_vide() {
        assert_eq!(Chantier::new().terminer(), EtatStation::Vide);
    }

    #[test]
    fn racine_expose_ses_ports() {
        let mut ch = Chantier::new();
        assert!(ch.racine(module()));
        assert_eq!(ch.nb_pieces(), 1);
        assert_eq!(ch.libres().len(), 6); // 2 écoutilles axiales + 4 montages Surface
    }

    #[test]
    fn poser_consomme_hote_et_ajoute_enfant() {
        let mut ch = Chantier::new();
        ch.racine(module());
        assert!(ch.poser(0, module(), 1)); // clipse un module par son écoutille −Z
        assert_eq!(ch.nb_pieces(), 2);
        // 6 (racine) − 1 (consommé) + 5 (ports restants de l'enfant) = 10
        assert_eq!(ch.libres().len(), 10);
    }

    #[test]
    fn poser_refuse_incompatible() {
        let mut ch = Chantier::new();
        ch.racine(module()); // ports ModuleAxial
        assert!(!ch.poser(0, panneau(), 0)); // port 0 = écoutille axiale → incompatible
        assert_eq!(ch.nb_pieces(), 1);
        assert_eq!(ch.libres().len(), 6);
    }

    #[test]
    fn poser_refuse_hote_hors_bornes() {
        let mut ch = Chantier::new();
        ch.racine(module());
        assert!(!ch.poser(9, module(), 1));
        assert_eq!(ch.nb_pieces(), 1);
    }

    #[test]
    fn budget_limite_les_poses() {
        // module coûte 5 ; budget 7 → racine (5) ok, 2 restant < 5 → pose refusée.
        let mut ch = Chantier::avec_budget(7.0);
        assert!(ch.racine(module()));
        assert!(!ch.poser(0, module(), 1));
        assert_eq!(ch.nb_pieces(), 1);
    }

    #[test]
    fn compatibles_liste_les_ports() {
        let mut ch = Chantier::new();
        ch.racine(module());
        assert_eq!(ch.compatibles(module(), 1).len(), 2); // 2 ports axiaux compatibles
        assert_eq!(ch.compatibles(panneau(), 0).len(), 4); // 4 montages Surface radiaux
    }

    #[test]
    fn collision_rejette_recouvrement() {
        let mut ch = Chantier::new();
        ch.racine(module()); // module long 2 → ports Surface ±X, ±Y à r=1
        let grand = Composant::PanneauSolaire {
            profil: Profil::P0,
            variante: VariantePanneau::RigideUS,
            longueur: 7.0,
            largeur: 1.5,
        };
        // Grand panneau sur +X : OK (hôte = module, exempté de la collision).
        let ix = ch.libres().iter().position(|p| p.repere.pos.x > 0.9).unwrap();
        assert!(ch.poser(ix, grand, 0));
        // Grand panneau sur +Y : recouvre le précédent (non-hôte) → rejeté.
        let iy = ch.libres().iter().position(|p| p.repere.pos.y > 0.9).unwrap();
        assert!(!ch.poser(iy, grand, 0), "recouvrement d'un voisin → rejeté");
    }

    #[test]
    fn treillis_accueille_appendices_via_chantier() {
        let mut ch = Chantier::new();
        ch.racine(Composant::Treillis { profil: Profil::P1, longueur: 6.0, style: StyleTreillis::Carre });
        // Docke un panneau sur chaque port Surface libre (ils ne prolifèrent pas).
        let mut poses = 0;
        while let Some(i) = ch.libres().iter().position(|p| p.genre == GenrePort::Surface) {
            assert!(ch.poser(i, panneau(), 0));
            poses += 1;
        }
        assert!(poses >= 2, "au moins une paire de montages");
        assert!(matches!(ch.terminer(), EtatStation::Prete(_)));
    }
}
