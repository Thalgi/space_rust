//! Arbre stellaire hiérarchique : décrit un système à plusieurs étoiles comme un
//! arbre de **paires** (chaque paire = deux sous-arbres orbitant leur barycentre
//! commun). Couvre toutes les topologies stables :
//!
//! - binaire        `(A·B)`
//! - trinaire        `((A·B)·C)`            (paire serrée + compagnon lointain)
//! - quadruple 2+2   `((A·B)·(C·D))`        (deux paires)
//! - quadruple 3+1   `(((A·B)·C)·D)`        (triple + compagnon lointain)
//!
//! L'arbre est **déployé** en une structure plate évaluable en O(n) chaque frame :
//! on descend racine → feuilles, en composant les orbites (une paire serrée dont le
//! barycentre tourne lui-même autour d'un compagnon lointain = deux mouvements
//! superposés, obtenus naturellement par la récursion).

use crate::astre::Astre;
use crate::orbite::Orbite;
use macroquad::prelude::*;

/// Variante d'une étoile-feuille (couronne / type particulier). Extensible aux
/// autres types d'astres à mesure qu'on en ajoute.
#[derive(Clone, Copy)]
#[allow(dead_code)] // certaines variantes ne sont pas encore utilisées par un preset
pub enum Variante {
    Normale,
    Jets,    // étoile à neutrons / protoétoile (jets bipolaires)
    Vent,    // Wolf-Rayet / supergéante bleue (vent stellaire)
    Pulsar,  // jets tournants + flash
    Magnetar,
}

/// Étoile-feuille de l'arbre. La **masse** sert au calcul des barycentres.
#[derive(Clone, Copy)]
pub struct Feuille {
    pub rayon: f32,
    pub couleur: Vec3,
    pub luminosite: f32,
    pub masse: f32,
    pub variante: Variante,
    /// Rémanent compact (pulsar/magnétar/étoile à neutrons) : pas de zone habitable.
    /// Distinct de `variante` car l'étoile à neutrons partage `Jets` avec T Tauri.
    pub remnant: bool,
}

impl Feuille {
    pub fn new(rayon: f32, couleur: Vec3, luminosite: f32, masse: f32) -> Self {
        Self { rayon, couleur, luminosite, masse, variante: Variante::Normale, remnant: false }
    }
    /// Fixe une variante (jets, vent, pulsar…).
    pub fn variante(mut self, v: Variante) -> Self {
        self.variante = v;
        self
    }
    /// Marque (ou non) la feuille comme rémanent compact sans zone habitable.
    pub fn avec_remnant(mut self, remnant: bool) -> Self {
        self.remnant = remnant;
        self
    }
}

/// Nœud de l'arbre : une étoile, ou une paire de sous-arbres.
pub enum Noeud {
    Etoile(Feuille),
    Paire {
        a: Box<Noeud>,
        b: Box<Noeud>,
        sep: f32,   // demi-grand axe RELATIF a↔b (en UA)
        e: f32,     // excentricité
        incl: f32,  // inclinaison du plan orbital (rad)
        phase: f32, // anomalie moyenne de départ (rad)
    },
}

impl Noeud {
    pub fn etoile(f: Feuille) -> Noeud {
        Noeud::Etoile(f)
    }
    pub fn paire(a: Noeud, b: Noeud, sep: f32, e: f32, incl: f32, phase: f32) -> Noeud {
        Noeud::Paire { a: Box::new(a), b: Box::new(b), sep, e, incl, phase }
    }
    /// Masse totale du sous-arbre (somme des feuilles).
    pub fn masse(&self) -> f32 {
        match self {
            Noeud::Etoile(f) => f.masse,
            Noeud::Paire { a, b, .. } => a.masse() + b.masse(),
        }
    }
}

/// Nœud déployé : orbite autour du barycentre parent + éventuelle étoile associée.
struct NoeudDeploye {
    parent: Option<usize>,  // index dans `noeuds` (garanti < index courant)
    orbite: Option<Orbite>, // None = racine (barycentre à l'origine)
    astre: Option<usize>,   // index de l'étoile dans `Systeme.astres` (feuille)
}

/// Arbre déployé : liste plate en ordre topologique (parent avant enfants).
pub struct ArbreStellaire {
    noeuds: Vec<NoeudDeploye>,
}

impl ArbreStellaire {
    pub fn new() -> Self {
        Self { noeuds: Vec::new() }
    }

    /// Ajoute un nœud déployé, renvoie son index. `parent` doit déjà exister.
    pub fn ajouter(&mut self, parent: Option<usize>, orbite: Option<Orbite>, astre: Option<usize>) -> usize {
        self.noeuds.push(NoeudDeploye { parent, orbite, astre });
        self.noeuds.len() - 1
    }

    /// Évalue les positions à l'instant `t` (racine → feuilles) et les écrit dans
    /// les étoiles correspondantes. O(n).
    pub fn evaluer(&self, t: f64, astres: &mut [Box<dyn Astre>]) {
        let mut pos = vec![Vec3::ZERO; self.noeuds.len()];
        for (i, nd) in self.noeuds.iter().enumerate() {
            let base = match nd.parent {
                Some(p) => pos[p], // déjà calculé (ordre topologique)
                None => Vec3::ZERO,
            };
            let local = match &nd.orbite {
                Some(o) => o.position(t),
                None => Vec3::ZERO,
            };
            pos[i] = base + local;
            if let Some(ai) = nd.astre {
                if let Some(a) = astres.get_mut(ai) {
                    a.corps_mut().position = pos[i];
                }
            }
        }
    }

    /// Polylignes (en coordonnées monde) des orbites de chaque étoile-feuille autour
    /// de son barycentre parent, à l'instant `t`. Pour tracer les orbites stellaires.
    pub fn orbites_etoiles(&self, t: f64) -> Vec<Vec<Vec3>> {
        let mut pos = vec![Vec3::ZERO; self.noeuds.len()];
        let mut sorties = Vec::new();
        for (i, nd) in self.noeuds.iter().enumerate() {
            let base = match nd.parent {
                Some(p) => pos[p],
                None => Vec3::ZERO,
            };
            let local = match &nd.orbite {
                Some(o) => o.position(t),
                None => Vec3::ZERO,
            };
            pos[i] = base + local;
            // Étoile (feuille) avec une orbite -> ellipse centrée sur le barycentre parent.
            if nd.astre.is_some() {
                if let (Some(p), Some(o)) = (nd.parent, nd.orbite.as_ref()) {
                    let centre = pos[p];
                    sorties.push(o.polyligne(96).into_iter().map(|pt| centre + pt).collect());
                }
            }
        }
        sorties
    }
}
