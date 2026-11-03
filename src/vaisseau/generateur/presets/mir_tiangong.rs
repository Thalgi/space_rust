use crate::vaisseau::montage::cuire;
use crate::vaisseau::{
    Assembleur, Composant, EtatStation, Profil, Repere,
    VarianteAntenne, VarianteCharge, VarianteModule,
    VariantePanneau,
};
use macroquad::prelude::*;
use super::commun::{appendice_sur_module, hub6, porter_vers};
use crate::vaisseau::generateur::moteur::nez_docking;

/// Module habitat de longueur libre (les stations réelles n'ont pas des modules
/// tous identiques).
pub(crate) fn hab_l(v: VarianteModule, longueur: f32) -> Composant {
    Composant::ModuleAxial { profil: Profil::P1, variante: v, longueur }
}

/// Paire d'ailes solaires opposées sur les flancs d'un module, selon l'axe monde
/// `axe`. Les stations russes et chinoises en portent de part et d'autre de
/// presque chaque module — c'est une bonne part de leur silhouette.
#[allow(clippy::too_many_arguments)]
pub(crate) fn paire_ailes(
    asm: &mut Assembleur,
    corps: Repere,
    comp: Composant,
    axe: Vec3,
    variante: VariantePanneau,
    longueur: f32,
    largeur: f32,
) {
    for dir in [axe, -axe] {
        let pan = Composant::PanneauSolaire { profil: Profil::P0, variante, longueur, largeur };
        appendice_sur_module(asm, corps, &comp, dir, &pan);
    }
}

/// Vaisseau amarré (Soyouz-TM, Progress-M) : petit corps à deux ailes. Mir en
/// portait presque toujours un à chaque port axial — sur les vues de référence
/// ils font partie intégrante de la silhouette.
pub(crate) fn vaisseau_amarre(
    asm: &mut Assembleur,
    hote_monde: Repere,
    hote: Composant,
    dir: Vec3,
    axe_ailes: Vec3,
    variante: VariantePanneau,
) {
    let corps = Composant::ModuleAxial {
        profil: Profil::P0,
        variante: VarianteModule::Dore,
        longueur: 2.2,
    };
    let vm = porter_vers(asm, hote_monde, &hote, dir, &corps, 1);
    paire_ailes(asm, vm, corps, axe_ailes, variante, 2.6, 0.9);
}

/// Preset : **reproduction de Mir** en configuration finale (1996–2001).
///
/// Topologie caractéristique : le cœur DOS-7 porte à l'avant un **nœud
/// sphérique à 5 ports** — un port axial (Soyouz/Progress) et **quatre radiaux**
/// occupés par Kvant-2, Kristall, Spektr et Priroda, d'où la « croix » de
/// modules si reconnaissable. Kvant-1, lui, est amarré à l'**arrière** du cœur
/// (et non au nœud) et offre le second port Soyouz. Module d'amarrage navette
/// au bout de Kristall, ailes solaires sur presque chaque module.
pub fn preset_mir() -> EtatStation {
    /// Cote réelle (mètres) → unités monde. Le profil P1 (rayon 1 U) représente
    /// le rayon du module central, dont le diamètre réel est 4,15 m ; les
    /// longueurs ci-dessous sont donc les cotes publiées, à l'échelle.
    fn cote(metres: f32) -> f32 {
        metres / 2.075
    }

    let mut asm = Assembleur::new();
    let bleu = VariantePanneau::RusseBleu;
    // Grandes ailes de Mir : ~10,6 m × 3,9 m. Elles sont presque aussi longues
    // que les modules — c'est très marqué sur les vues de référence.
    let (aile_l, aile_w) = (cote(10.6), cote(3.9));

    // Cœur DOS-7 (13,13 m), deux grandes ailes latérales + antennes arrière.
    let coeur = hab_l(VarianteModule::Coeur, cote(13.13));
    let cm = Repere::IDENTITE;
    asm.ajouter(cuire(cm, &coeur));
    paire_ailes(&mut asm, cm, coeur.clone(), Vec3::Y, bleu, aile_l, aile_w);
    let antenne = Composant::Antenne {
        profil: Profil::P0,
        variante: VarianteAntenne::ParaboleOffset,
        taille: 0.9,
    };
    appendice_sur_module(&mut asm, cm, &coeur, Vec3::X, &antenne);

    // Nœud sphérique avant (5 ports libres) + Soyouz-TM au port axial.
    let noeud = hub6();
    let nm = porter_vers(&mut asm, cm, &coeur, Vec3::Z, &noeud, 1);
    vaisseau_amarre(&mut asm, nm, noeud.clone(), Vec3::Z, Vec3::Y, bleu);

    // La croix autour du nœud, dans la disposition du schéma d'assemblage :
    // Priroda au zénith, Kristall au nadir, Kvant-2 et Spektr sur les flancs.

    // Priroda : le seul module **sans aile** (il tournait sur batteries).
    let priroda = hab_l(VarianteModule::Labo, cote(11.90));
    porter_vers(&mut asm, nm, &noeud, Vec3::Y, &priroda, 1);

    // Kristall : **sans aile non plus** en configuration finale — les siennes
    // ont été transférées sur Kvant-1. Il porte le module d'amarrage navette
    // (4,70 m × Ø 2,20), un vrai module étroit et non un simple nez.
    let kristall = hab_l(VarianteModule::Labo, cote(11.90));
    let kri = porter_vers(&mut asm, nm, &noeud, Vec3::NEG_Y, &kristall, 1);
    let so = Composant::ModuleAxial {
        profil: Profil::P0,
        variante: VarianteModule::Standard,
        longueur: cote(4.70),
    };
    let som = porter_vers(&mut asm, kri, &kristall, Vec3::NEG_Y, &so, 1);
    porter_vers(&mut asm, som, &so, Vec3::NEG_Y, &nez_docking(), 0);

    // Kvant-2 : le plus long des modules (13,73 m).
    let kvant2 = hab_l(VarianteModule::Hublots, cote(13.73));
    let kv2 = porter_vers(&mut asm, nm, &noeud, Vec3::X, &kvant2, 1);
    paire_ailes(&mut asm, kv2, kvant2, Vec3::Y, bleu, aile_l, aile_w);

    // Spektr : **quatre** ailes (deux paires croisées), sa signature.
    let spektr = hab_l(VarianteModule::Standard, cote(9.10));
    let spe = porter_vers(&mut asm, nm, &noeud, Vec3::NEG_X, &spektr, 1);
    paire_ailes(&mut asm, spe, spektr.clone(), Vec3::Y, bleu, cote(8.6), cote(3.4));
    paire_ailes(&mut asm, spe, spektr, Vec3::Z, bleu, cote(7.0), cote(3.0));

    // Kvant-1 à l'arrière du cœur : tonneau court (5,80 m), mais porteur des
    // grandes ailes reprises de Kristall. Progress-M à son port arrière.
    let kvant1 = hab_l(VarianteModule::Dore, cote(5.80));
    let kv1 = porter_vers(&mut asm, cm, &coeur, Vec3::NEG_Z, &kvant1, 1);
    paire_ailes(&mut asm, kv1, kvant1.clone(), Vec3::Y, bleu, aile_l, aile_w);
    vaisseau_amarre(&mut asm, kv1, kvant1, Vec3::NEG_Z, Vec3::Y, bleu);

    asm.terminer()
}

/// Preset : **reproduction de Tiangong** (configuration en T, depuis 2022).
///
/// Le cœur **Tianhe** porte à l'avant un nœud dont deux ports **radiaux
/// opposés** reçoivent les laboratoires **Wentian** et **Mengtian** — d'où le T
/// caractéristique. Le nœud offre aussi un port avant et un port **nadir**
/// (Shenzhou habité) ; l'arrière du cœur reçoit le cargo Tianzhou. Les ailes des
/// laboratoires sont nettement plus grandes que celles du cœur (27 m contre
/// 12,6 m dans la réalité), ce qui fait beaucoup de la silhouette.
pub fn preset_tiangong() -> EtatStation {
    let mut asm = Assembleur::new();
    let ailes = VariantePanneau::RollOut; // grandes ailes souples, sombres

    // Tianhe : cœur + ailes moyennes.
    let tianhe = hab_l(VarianteModule::Standard, 5.0);
    let thm = Repere::IDENTITE;
    asm.ajouter(cuire(thm, &tianhe));
    paire_ailes(&mut asm, thm, tianhe.clone(), Vec3::Y, ailes, 4.5, 1.2);
    // Port arrière : cargo Tianzhou.
    porter_vers(&mut asm, thm, &tianhe, Vec3::NEG_Z, &nez_docking(), 0);

    // Nœud avant : port axial (Shenzhou / télescope) + port nadir habité.
    let noeud = hub6();
    let nm = porter_vers(&mut asm, thm, &tianhe, Vec3::Z, &noeud, 1);
    porter_vers(&mut asm, nm, &noeud, Vec3::Z, &nez_docking(), 0);
    porter_vers(&mut asm, nm, &noeud, Vec3::NEG_Y, &nez_docking(), 0);

    // La barre du T : les deux laboratoires, avec leurs grandes ailes.
    let wentian = hab_l(VarianteModule::Labo, 5.0);
    let wm = porter_vers(&mut asm, nm, &noeud, Vec3::X, &wentian, 1);
    paire_ailes(&mut asm, wm, wentian.clone(), Vec3::Y, ailes, 7.5, 1.6);
    // Sas EVA au bout de Wentian.
    porter_vers(&mut asm, wm, &wentian, Vec3::X, &hab_l(VarianteModule::Sas, 2.0), 1);

    let mengtian = hab_l(VarianteModule::Labo, 5.0);
    let mm = porter_vers(&mut asm, nm, &noeud, Vec3::NEG_X, &mengtian, 1);
    paire_ailes(&mut asm, mm, mengtian.clone(), Vec3::Y, ailes, 7.5, 1.6);
    // Sas cargo au bout de Mengtian, et sa **plateforme exposée** — Mengtian
    // en porte une, qui accueille des charges utiles hors pression.
    porter_vers(&mut asm, mm, &mengtian, Vec3::NEG_X, &hab_l(VarianteModule::Sas, 2.0), 1);
    let palette = Composant::ChargeUtile {
        profil: Profil::P0,
        variante: VarianteCharge::Palette,
        longueur: 2.6,
        largeur: 1.4,
    };
    appendice_sur_module(&mut asm, mm, &mengtian, Vec3::Z, &palette);

    asm.terminer()
}
