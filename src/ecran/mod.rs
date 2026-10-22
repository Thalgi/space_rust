//! Découpage du jeu en écrans : un accueil qui aiguille vers la vue système
//! complète (`Skymap`) ou la vue d'un astre isolé (`Objet`, pratique pour
//! travailler le rendu d'un seul corps). `main` se contente d'alterner entre eux.

mod accueil;
mod assembleur;
pub(crate) mod bandeau;
mod catalogue;
mod defilement;
mod clic;
mod designation;
mod enveloppes;
mod fiche;
mod fils;
mod icone;
mod marqueur;
pub(crate) mod liste;
mod galerie;
mod galerie_disques;
mod galerie_cometes;
mod galerie_etoiles;
mod objet;
mod prototypes;
mod palette;
mod panache;
mod parametres;
mod ports;
pub(crate) mod pixel;
mod selecteur;
mod skymap;
mod skymap_v2;
mod trace_orbite;
mod starmap;
mod station;
mod vignette;

pub use accueil::{Accueil, Cible};
pub use assembleur::VueAssembleur;
pub use galerie::Galerie;
pub use galerie_disques::GalerieDisques;
pub use galerie_cometes::GalerieCometes;
pub use galerie_etoiles::GalerieEtoiles;
pub use objet::Objet;
pub use parametres::Parametres;
pub use skymap::Skymap;
pub use skymap_v2::SkymapV2;
pub use starmap::{SortieStarmap, Starmap};
pub use station::{Categorie, VueStation};
