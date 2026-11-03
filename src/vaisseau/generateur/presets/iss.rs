use crate::vaisseau::montage::{cuire, port_monde, poser};
use crate::vaisseau::{
    Assembleur, Composant, EtatStation, GenrePort, Profil, Repere, StyleTreillis,
    VarianteAntenne, VarianteCaisson, VarianteCharge, VarianteModule,
    VariantePanneau, VariantePropulseur, VarianteRadiateur,
};
use macroquad::prelude::*;
use super::commun::{appendice_sur_module, hub6, porter_vers};

// Indices des ports d'un nœud Six : 0=+Z, 1=−Z, 2=+X, 3=−X, 4=+Y, 5=−Y. Le boom
// Z1 et la poutre visent ces faces par index ; le reste du preset docke par
// **direction monde** (`porter_vers`), robuste au basculement des nœuds.
pub(crate) const HUB_X_PLUS: usize = 2;
pub(crate) const HUB_X_MOINS: usize = 3;
pub(crate) const HUB_Y_PLUS: usize = 4;

pub(crate) fn hab(v: VarianteModule) -> Composant {
    Composant::ModuleAxial { profil: Profil::P1, variante: v, longueur: 3.0 }
}

/// Pose `enfant` (par son port `montage`) sur le port `idx` d'un hôte placé en
/// `hote_monde`, l'ajoute à l'assemblage et renvoie son repère monde.
pub(crate) fn poser_sur(
    asm: &mut Assembleur,
    hote_monde: Repere,
    hote: &Composant,
    idx: usize,
    enfant: &Composant,
    montage: usize,
) -> Repere {
    let m = poser(port_monde(hote_monde, hote, idx), enfant, montage);
    asm.ajouter(cuire(m, enfant));
    m
}

/// Petits arrays russes (bleus) sur les flancs ±X d'un module.
pub(crate) fn arrays_russes(asm: &mut Assembleur, corps: Repere, comp: &Composant) {
    for p in comp.ports() {
        if p.genre != GenrePort::Surface || p.repere.avant().x.abs() < 0.9 {
            continue;
        }
        let pan = Composant::PanneauSolaire {
            profil: Profil::P0,
            variante: VariantePanneau::RusseBleu,
            longueur: 3.0,
            largeur: 1.1,
        };
        asm.ajouter(cuire(poser(corps.compose(p.repere), &pan, 0), &pan));
    }
}

/// Preset : **reproduction de l'ISS**, assemblée à la main (référence pour juger
/// ce qu'il manque au générateur, cf. `docs/suivi/stations.md` Partie B). Poutre déportée
/// au **zénith** par un boom Z1 (elle ne traverse plus le cœur) ; segment US
/// habité (Destiny→Harmony + Columbus/Kibo + grappe Node3/Cupola + nez PMA) et
/// segment russe (FGB→SM + petits arrays + nœud MRM) fore/aft ; Sas Quest, PMM
/// et radiateurs au nadir sur le cœur.
pub fn preset_iss() -> EtatStation {
    let mut asm = Assembleur::new();

    // Unity (Node 1) — cœur.
    let hub = hub6();
    let hub_m = Repere::IDENTITE;
    asm.ajouter(cuire(hub_m, &hub));

    // ===== Poutre au zénith via boom Z1 : Unity(+Y) → Z1 → nœud S0 → poutre ±X.
    // Arrays sur la moitié externe (z < 0), radiateurs sur la moitié interne. =====
    let z1 = Composant::Treillis { profil: Profil::P1, longueur: 3.0, style: StyleTreillis::Carre };
    let z1m = poser_sur(&mut asm, hub_m, &hub, HUB_Y_PLUS, &z1, 0);
    let s0 = hub6();
    let s0m = poser_sur(&mut asm, z1m, &z1, 1, &s0, 1);
    for axe in [HUB_X_PLUS, HUB_X_MOINS] {
        let truss = Composant::Treillis { profil: Profil::P2, longueur: 15.0, style: StyleTreillis::Carre };
        let tm = poser_sur(&mut asm, s0m, &s0, axe, &truss, 0);
        for p in truss.ports() {
            if p.genre != GenrePort::Surface {
                continue;
            }
            let z = p.repere.pos.z; // vers S0 (inboard) ou vers l'extérieur
            let app = if z < -3.5 {
                Composant::PanneauSolaire { profil: Profil::P0, variante: VariantePanneau::RigideUS, longueur: 6.5, largeur: 2.0 }
            } else if z > 3.5 {
                Composant::Radiateur { profil: Profil::P0, variante: VarianteRadiateur::AccordeonATCS, longueur: 3.5, largeur: 1.5 }
            } else if p.repere.pos.x > 0.0 {
                // Zone médiane de poutre : équipements exposés (type ELC),
                // sur une seule face pour ne pas la surcharger.
                Composant::Caisson { profil: Profil::P0, variante: VarianteCaisson::Ossature, longueur: 1.6, largeur: 1.1 }
            } else {
                continue;
            };
            asm.ajouter(cuire(poser(tm.compose(p.repere), &app, 0), &app));
        }
    }

    // ===== Segment US (aft, −Z) : Node1 → Destiny → Harmony, ramifié. =====
    let node1 = hub6();
    let n1 = porter_vers(&mut asm, hub_m, &hub, Vec3::NEG_Z, &node1, 1);
    let lab = hab(VarianteModule::Labo);
    let labm = porter_vers(&mut asm, n1, &node1, Vec3::NEG_Z, &lab, 1);
    let node2 = hub6();
    let n2 = porter_vers(&mut asm, labm, &lab, Vec3::NEG_Z, &node2, 1);
    // Columbus (tribord) et Kibō (bâbord) latéraux sur Harmony.
    porter_vers(&mut asm, n2, &node2, Vec3::X, &hab(VarianteModule::Labo), 1);
    porter_vers(&mut asm, n2, &node2, Vec3::NEG_X, &hab(VarianteModule::Hublots), 1);
    // Module avant + nez de docking PMA/IDA (adaptateur conique P1→P0).
    let av = hab(VarianteModule::Hublots);
    let avm = porter_vers(&mut asm, n2, &node2, Vec3::NEG_Z, &av, 1);
    let nez = Composant::Adaptateur { grand: Profil::P1, petit: Profil::P0, longueur: 1.2 };
    porter_vers(&mut asm, avm, &av, Vec3::NEG_Z, &nez, 0);

    // Grappe Tranquility (Node3) sous Node1 : Cupola (nadir), BEAM (tribord),
    // PMM/Leonardo (bâbord).
    let node3 = hub6();
    let n3 = porter_vers(&mut asm, n1, &node1, Vec3::NEG_Y, &node3, 1);
    porter_vers(&mut asm, n3, &node3, Vec3::NEG_Y, &hab(VarianteModule::Coupole), 1);
    porter_vers(&mut asm, n3, &node3, Vec3::X, &hab(VarianteModule::Gonflable), 1);
    porter_vers(&mut asm, n3, &node3, Vec3::NEG_X, &hab(VarianteModule::Standard), 1);

    // ===== Segment russe (fore, +Z) : Zarya → Zvezda (arrays) → nœud + MRM. =====
    let fgb = hab(VarianteModule::Coeur);
    let fgbm = porter_vers(&mut asm, hub_m, &hub, Vec3::Z, &fgb, 1);
    arrays_russes(&mut asm, fgbm, &fgb);
    let sm = hab(VarianteModule::Coeur);
    let smm = porter_vers(&mut asm, fgbm, &fgb, Vec3::Z, &sm, 1);
    arrays_russes(&mut asm, smm, &sm);
    let rn = hub6();
    let rnm = porter_vers(&mut asm, smm, &sm, Vec3::Z, &rn, 1);
    for dir in [Vec3::Y, Vec3::NEG_Y, Vec3::Z] {
        porter_vers(&mut asm, rnm, &rn, dir, &hab(VarianteModule::Dore), 1);
    }

    // ===== Sur le cœur : Sas Quest (tribord) + radiateurs nadir sur modules. =====
    porter_vers(&mut asm, hub_m, &hub, Vec3::X, &hab(VarianteModule::Sas), 1);
    let radia = Composant::Radiateur { profil: Profil::P0, variante: VarianteRadiateur::PanneauSimple, longueur: 2.6, largeur: 1.2 };
    appendice_sur_module(&mut asm, labm, &lab, Vec3::NEG_Y, &radia);
    appendice_sur_module(&mut asm, smm, &sm, Vec3::NEG_Y, &radia);

    asm.terminer()
}

/// Pose `enfant` sur la **face** de `hote` dont l'avant monde vise `dir`.
/// Contrairement à `porter_vers`, on cible ici les ports **hôtes `Surface`** :
/// c'est ce qui permet d'habiller les cinq faces d'un caisson, par exemple le
/// bus d'un satellite.
pub(crate) fn sur_face(
    asm: &mut Assembleur,
    hote_monde: Repere,
    hote: &Composant,
    dir: Vec3,
    enfant: &Composant,
) -> Option<Repere> {
    let ports = hote.ports();
    let cible = ports.iter().enumerate().find(|(i, p)| {
        // L'index 0 d'un appendice/porteur est son propre montage : on l'ignore.
        *i > 0
            && p.genre == GenrePort::Surface
            && (hote_monde.rot * p.repere.avant()).normalize_or_zero().dot(dir) > 0.9
    })?;
    let w = poser(hote_monde.compose(cible.1.repere), enfant, 0);
    asm.ajouter(cuire(w, enfant));
    Some(w)
}

/// Preset : **satellite de communication** géostationnaire, bâti avec le même
/// vocabulaire que les stations. Bus **parallélépipédique** (`Caisson` fermé),
/// deux grandes ailes solaires opposées, une parabole principale vers la Terre
/// encadrée de deux antennes secondaires, radiateurs sur les faces froides et
/// propulsion électrique de maintien à poste.
pub fn preset_comsat() -> EtatStation {
    let mut asm = Assembleur::new();
    let bus = Composant::Caisson {
        profil: Profil::P1,
        variante: VarianteCaisson::Ferme,
        longueur: 2.6,
        largeur: 2.0,
    };
    let bm = Repere::IDENTITE;
    asm.ajouter(cuire(bm, &bus));

    // Ailes solaires : les deux grandes faces opposées.
    for dir in [Vec3::X, Vec3::NEG_X] {
        let aile = Composant::PanneauSolaire {
            profil: Profil::P0,
            variante: VariantePanneau::RigideUS,
            longueur: 6.0,
            largeur: 1.5,
        };
        sur_face(&mut asm, bm, &bus, dir, &aile);
    }
    // Parabole principale vers la Terre + deux antennes secondaires.
    let grande = Composant::Antenne {
        profil: Profil::P0,
        variante: VarianteAntenne::ParaboleGG,
        taille: 1.5,
    };
    sur_face(&mut asm, bm, &bus, Vec3::Z, &grande);
    for (dir, v) in [
        (Vec3::Y, VarianteAntenne::ParaboleOffset),
        (Vec3::NEG_Y, VarianteAntenne::Cornets),
    ] {
        let a = Composant::Antenne { profil: Profil::P0, variante: v, taille: 0.9 };
        sur_face(&mut asm, bm, &bus, dir, &a);
    }
    // Radiateur et propulsion de maintien à poste sur les faces restantes.
    let radia = Composant::Radiateur {
        profil: Profil::P0,
        variante: VarianteRadiateur::PanneauSimple,
        longueur: 2.2,
        largeur: 1.1,
    };
    sur_face(&mut asm, bm, &bus, Vec3::Y, &radia);
    let tuyere = Composant::Propulseur {
        profil: Profil::P0,
        variante: VariantePropulseur::EffetHall,
        taille: 0.8,
    };
    sur_face(&mut asm, bm, &bus, Vec3::NEG_Y, &tuyere);

    asm.terminer()
}

/// Preset : **sonde interplanétaire**. Bus à ossature, grande parabole de
/// liaison, propulsion ionique, palette d'instruments et **perche de
/// magnétomètre** — la perche déportée est la signature d'une sonde.
pub fn preset_sonde() -> EtatStation {
    let mut asm = Assembleur::new();
    let bus = Composant::Caisson {
        profil: Profil::P1,
        variante: VarianteCaisson::Ossature,
        longueur: 2.2,
        largeur: 1.8,
    };
    let bm = Repere::IDENTITE;
    asm.ajouter(cuire(bm, &bus));

    // Grande parabole de liaison, à l'avant.
    let hga = Composant::Antenne {
        profil: Profil::P0,
        variante: VarianteAntenne::ParaboleGG,
        taille: 1.8,
    };
    sur_face(&mut asm, bm, &bus, Vec3::Z, &hga);
    // Ailes solaires modestes : loin du Soleil, on ne peut pas compter dessus.
    for dir in [Vec3::X, Vec3::NEG_X] {
        let aile = Composant::PanneauSolaire {
            profil: Profil::P0,
            variante: VariantePanneau::RollOut,
            longueur: 4.2,
            largeur: 1.1,
        };
        sur_face(&mut asm, bm, &bus, dir, &aile);
    }
    // Palette d'instruments sur une face, propulsion ionique de l'autre.
    let palette = Composant::ChargeUtile {
        profil: Profil::P0,
        variante: VarianteCharge::Palette,
        longueur: 1.6,
        largeur: 1.0,
    };
    sur_face(&mut asm, bm, &bus, Vec3::NEG_Y, &palette);
    let moteur = Composant::Propulseur {
        profil: Profil::P0,
        variante: VariantePropulseur::IoniqueGrille,
        taille: 1.0,
    };
    sur_face(&mut asm, bm, &bus, Vec3::Y, &moteur);

    // Perche de magnétomètre : un treillis fin, éloigné du bus pour fuir ses
    // perturbations magnétiques, avec le capteur au bout.
    let perche = Composant::Treillis {
        profil: Profil::P0,
        longueur: 5.0,
        style: StyleTreillis::Triangulaire,
    };
    let pm = poser(bm.compose(bus.ports()[0].repere), &perche, 0);
    asm.ajouter(cuire(pm, &perche));
    // Tête de capteur au bout : montage axial (index 1 = extrémité libre de la
    // poutre), le seul genre de port qu'accepte le bout d'un treillis.
    let capteur = Composant::ModuleAxial {
        profil: Profil::P0,
        variante: VarianteModule::Standard,
        longueur: 0.6,
    };
    poser_sur(&mut asm, pm, &perche, 1, &capteur, 0);

    asm.terminer()
}
