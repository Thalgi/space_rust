//! Découpage du jeu en écrans : un accueil qui aiguille vers la vue système
//! complète (`Skymap`) ou la vue d'un astre isolé (`Objet`, pratique pour
//! travailler le rendu d'un seul corps). `main` se contente d'alterner entre eux.

mod accueil;
mod briques;
mod galerie;
mod galerie_etoiles;
mod objet;
mod skymap;
mod starmap;
mod station;
mod vaisseaux;

pub use accueil::{Accueil, Cible};
pub use briques::Briques;
pub use galerie::Galerie;
pub use galerie_etoiles::GalerieEtoiles;
pub use objet::Objet;
pub use skymap::Skymap;
pub use starmap::{SortieStarmap, Starmap};
pub use station::VueStation;
pub use vaisseaux::Vaisseaux;
