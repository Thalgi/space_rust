//! **Statistiques de vol** (Lot A, `docs/conception/trajectoires.md` §6) :
//! une base par classe de vaisseau, modifiée par les pièces de propulsion et
//! de réservoir effectivement assemblées. Pas de simulation masse/poussée/
//! Isp par pièce — décision actée, pas un manque (`trajectoires.md` §4,
//! décision 2).
//!
//! Volontairement limité au **Starship** aujourd'hui : l'ISV a sa propre
//! mécanique de saut interstellaire, hors périmètre de ce système
//! (`trajectoires.md` §4, décision 6) ; le Jupitris et le cargo sans nom
//! n'ont pas encore de design à mesurer (`conception/concept_jeu.md` §7).
//!
//! Écrit et testé pour son propre compte, pas encore câblé au jeu : c'est le
//! Lot B (`trajectoires.md` §6) qui consommera `calculer` pour donner une
//! vitesse/portée à un vaisseau en transit. D'ici là, la cible binaire ne
//! l'appelle nulle part — `dead_code` ne sait pas ce qu'est un test
//! (`methode.md`), d'où l'`allow` du module entier plutôt qu'un vrai manque.
#![allow(dead_code)]

use super::{Composant, Station};

/// Classe de vaisseau dont ce module connaît les statistiques de vol.
/// Un seul variant aujourd'hui, volontairement — voir l'en-tête du module.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ClasseVaisseau {
    Starship,
}

/// Statistiques de vol d'un vaisseau assemblé.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct StatsVol {
    /// Vitesse de croisière, en unités de monde par seconde de simulation.
    pub vitesse: f32,
    /// Portée, en unités de monde.
    pub portee: f32,
    /// Capacité de fret. Pas de piece générique de soute pour le Starship
    /// aujourd'hui (`NacelleCargo`/`RatelierCargo` sont propres à l'ISV) —
    /// reste donc fixe à la base de la classe, aucune pièce ne la modifie.
    /// Simplification explicite, pas un oubli.
    pub capacite_fret: f32,
}

/// Base d'un Starship. **Chiffres provisoires** — bouche-trou nommé (comme
/// `engin::ECHELLE_ISS`), à recalibrer sur un premier trajet joué, pas avant.
const BASE_VITESSE_STARSHIP: f32 = 4.0;
const BASE_PORTEE_STARSHIP: f32 = 40.0;
const BASE_FRET_STARSHIP: f32 = 20.0;

/// Un moteur principal (axial pour un `Propulseur`, tout `Raptor`) ajoute
/// ceci à la vitesse. Une grappe RCS (attitude, pas propulsion) n'ajoute
/// rien — voir [`VariantePropulseur::axial`](super::VariantePropulseur::axial).
const MODIF_VITESSE_MOTEUR: f32 = 0.6;

/// Par unité de longueur de [`Composant::Reservoir`].
const MODIF_PORTEE_PAR_LONGUEUR_RESERVOIR: f32 = 3.0;

impl ClasseVaisseau {
    fn base(self) -> StatsVol {
        match self {
            ClasseVaisseau::Starship => StatsVol {
                vitesse: BASE_VITESSE_STARSHIP,
                portee: BASE_PORTEE_STARSHIP,
                capacite_fret: BASE_FRET_STARSHIP,
            },
        }
    }
}

/// Contribution à la vitesse d'une pièce : moteurs principaux seulement, pas
/// les grappes d'attitude (RCS). Pas un `match` exhaustif sur `Composant` —
/// volontairement : ce calcul ne concerne que la propulsion, un `_ => 0.0`
/// pour tout le reste du catalogue n'est pas un oubli qui pourrait passer
/// inaperçu (contrairement à `Composant::cout`, qui doit compter chaque
/// pièce).
fn modificateur_vitesse(composant: &Composant) -> f32 {
    match composant {
        Composant::Propulseur { variante, .. } if variante.axial() => MODIF_VITESSE_MOTEUR,
        Composant::Raptor { .. } => MODIF_VITESSE_MOTEUR,
        _ => 0.0,
    }
}

/// Contribution à la portée d'une pièce : réservoirs, proportionnelle à leur
/// longueur.
fn modificateur_portee(composant: &Composant) -> f32 {
    match composant {
        Composant::Reservoir { longueur, .. } => longueur * MODIF_PORTEE_PAR_LONGUEUR_RESERVOIR,
        _ => 0.0,
    }
}

/// Calcule les statistiques de vol d'un vaisseau assemblé : la base de sa
/// classe, plus les modificateurs de ses pièces de propulsion/réservoir.
pub fn calculer(classe: ClasseVaisseau, station: &Station) -> StatsVol {
    let base = classe.base();
    let vitesse = base.vitesse
        + station.pieces().iter().map(|p| modificateur_vitesse(&p.composant)).sum::<f32>();
    let portee = base.portee
        + station.pieces().iter().map(|p| modificateur_portee(&p.composant)).sum::<f32>();
    StatsVol { vitesse, portee, capacite_fret: base.capacite_fret }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vaisseau::{Piece, Profil, VarianteModule, VariantePropulseur, VarianteRaptor};
    use macroquad::prelude::*;

    fn piece(comp: Composant) -> Piece {
        Piece::new(Mat4::IDENTITY, comp)
    }

    fn module_neutre() -> Composant {
        Composant::ModuleAxial { profil: Profil::P1, variante: VarianteModule::Standard, longueur: 1.0 }
    }

    fn station(pieces: Vec<Composant>) -> Station {
        Station::depuis_pieces(pieces.into_iter().map(piece).collect()).expect("au moins une pièce")
    }

    // Un vaisseau sans pièce de propulsion/réservoir retombe exactement sur
    // la base de sa classe — la pièce neutre ne doit rien ajouter.
    #[test]
    fn sans_piece_pertinente_les_stats_sont_la_base_de_la_classe() {
        let s = station(vec![module_neutre()]);
        let stats = calculer(ClasseVaisseau::Starship, &s);
        assert_eq!(stats.vitesse, BASE_VITESSE_STARSHIP);
        assert_eq!(stats.portee, BASE_PORTEE_STARSHIP);
        assert_eq!(stats.capacite_fret, BASE_FRET_STARSHIP);
    }

    // Un réservoir de plus allonge la portée, proportionnellement à sa
    // longueur, sans toucher à la vitesse.
    #[test]
    fn un_reservoir_allonge_la_portee_sans_changer_la_vitesse() {
        let s = station(vec![
            module_neutre(),
            Composant::Reservoir { profil: Profil::P1, longueur: 5.0, cage: false },
        ]);
        let stats = calculer(ClasseVaisseau::Starship, &s);
        assert_eq!(stats.portee, BASE_PORTEE_STARSHIP + 5.0 * MODIF_PORTEE_PAR_LONGUEUR_RESERVOIR);
        assert_eq!(stats.vitesse, BASE_VITESSE_STARSHIP);
    }

    // Un Raptor (moteur du Starship) augmente la vitesse.
    #[test]
    fn un_raptor_augmente_la_vitesse() {
        let s = station(vec![
            module_neutre(),
            Composant::Raptor { profil: Profil::P0, variante: VarianteRaptor::Atmospherique, rayon: 0.5 },
        ]);
        let stats = calculer(ClasseVaisseau::Starship, &s);
        assert_eq!(stats.vitesse, BASE_VITESSE_STARSHIP + MODIF_VITESSE_MOTEUR);
    }

    // Un propulseur générique **axial** (moteur principal) augmente la
    // vitesse comme un Raptor.
    #[test]
    fn un_propulseur_axial_augmente_la_vitesse() {
        let s = station(vec![
            module_neutre(),
            Composant::Propulseur { profil: Profil::P1, variante: VariantePropulseur::TuyereCloche, taille: 2.0 },
        ]);
        let stats = calculer(ClasseVaisseau::Starship, &s);
        assert_eq!(stats.vitesse, BASE_VITESSE_STARSHIP + MODIF_VITESSE_MOTEUR);
    }

    // Une grappe RCS (attitude, pas propulsion principale) n'ajoute rien à
    // la vitesse — le garde-fou `variante.axial()` doit vraiment filtrer.
    #[test]
    fn une_grappe_rcs_najoute_rien_a_la_vitesse() {
        let s = station(vec![
            module_neutre(),
            Composant::Propulseur { profil: Profil::P1, variante: VariantePropulseur::GrappeRcs, taille: 1.0 },
        ]);
        let stats = calculer(ClasseVaisseau::Starship, &s);
        assert_eq!(stats.vitesse, BASE_VITESSE_STARSHIP);
    }
}
