//! Générateur procédural de stations, posé sur le [`Chantier`]
//! (`docs/conception/stations.md`, Partie A §6–7). Une **grammaire** pilote le
//! constructeur : choix d'une ossature, puis habillage des ports libres, le tout
//! borné par un budget et rendu déterministe par une graine.
//!
//! Découpé en `moteur` (le générateur procédural réel, point d'entrée
//! [`generer`]) et `presets` (les ~60 vaisseaux/vitrines figés, sans rapport
//! avec le moteur — cf. `docs/suivi/dette_vaisseau.md`, Chantier 1).

mod moteur;
mod presets;
#[cfg(test)]
// Ces tests affirment des choses sur des CONSTANTES, et clippy y voit une
// condition toujours vraie. Elle l'est — aujourd'hui. Le jour où quelqu'un
// abaisse la constante, l'assertion devient fausse et le test tombe : c'est
// précisément son rôle, garder la valeur ET dire pourquoi elle compte.
#[allow(clippy::assertions_on_constants)]
mod tests;

pub use moteur::*;
pub use presets::*;
