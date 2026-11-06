//! **La genèse V2** : un système se *planifie* (des données pures, tirées de
//! flux de hasard nommés) puis s'*instancie*. Conception :
//! `docs/conception/genese_v2.md` ; journal : `docs/suivi/genese_v2.md`.
//!
//! La genèse V1 (`crate::genese`) reste en place, **gelée** : les Skymaps V1
//! et V2 tournent dessus, la V3 tourne sur celle-ci.

pub mod alea;
pub mod apparences;
pub mod banc;
pub mod instancier;
pub mod physique;
pub mod plan;
pub mod planifier;
