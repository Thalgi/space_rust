//! Les ~60 `preset_*`/`demo_*` figés, éclatés par famille de station — sans
//! rapport entre eux ni avec le moteur de génération procédural (`moteur.rs`).
//! `commun.rs` porte ce qui s'est avéré réellement partagé entre familles
//! (vérifié par `find_referencing_symbols`, pas supposé sur la position dans
//! l'ancien fichier — cf. `docs/suivi/dette_vaisseau.md`, Chantier 1).

mod commun;
mod demos;
mod elysium;
mod iss;
mod isv;
mod mir_tiangong;
mod stanford;
mod starship;

pub use commun::*;
pub use demos::*;
pub use elysium::*;
pub use iss::*;
pub use isv::*;
pub use mir_tiangong::*;
pub use stanford::*;
pub use starship::*;
