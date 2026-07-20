//! Glu entre le modèle de ports (`port.rs`) et l'assemblage (`assemblage.rs`) —
//! sous-étape 2c de `docs/stations_raccordement.md`.
//!
//! Trois fonctions composables, sans état :
//! - [`port_monde`] : repère **monde** d'un port d'un composant déjà placé ;
//! - [`poser`] : repère **monde** d'un enfant qui clipse un de ses ports sur un
//!   port hôte (chaînage — reste en `Repere`, couche construction) ;
//! - [`cuire`] : fige un composant placé en [`Piece`] (`Mat4`, couche cuite).
//!
//! Le chaînage se fait donc entièrement en `Repere` (rotation pure, exact), et
//! l'on ne « cuit » en `Mat4` qu'au moment de produire la `Piece` à dessiner.

use super::chantier::Chantier;
use super::{
    accoupler, Assembleur, Composant, EtatStation, GenrePort, Piece, Profil, Repere, Sorties,
    Station, StyleTreillis, Symetrie, VarianteAntenne, VarianteModule, VariantePanneau,
    VarianteRadiateur,
};
use macroquad::prelude::*;

/// Repère **monde** du port `idx` d'un composant dont le corps est à `corps`
/// (repère monde). Sert à récupérer un port hôte libre avant d'y clipser un
/// enfant.
///
/// Panique si `idx` dépasse le nombre de ports du composant (erreur de
/// programmation, comme un accès hors bornes).
pub fn port_monde(corps: Repere, comp: Composant, idx: usize) -> Repere {
    corps.compose(comp.ports()[idx].repere)
}

/// Repère **monde** d'un composant `enfant` qui vient clipser son port de
/// montage `montage_idx` sur le port hôte `hote` (déjà en monde). C'est
/// [`accoupler`] appliqué au port choisi de l'enfant : positions coïncidentes,
/// avants opposés, hauts alignés.
///
/// Reste en `Repere` pour pouvoir enchaîner (poser un petit-enfant sur un port
/// libre de l'enfant). La compatibilité genre/profil relève de l'appelant (le
/// générateur ne pioche que dans les ports compatibles, cf. `Port::compatible`).
pub fn poser(hote: Repere, enfant: Composant, montage_idx: usize) -> Repere {
    accoupler(hote, enfant.ports()[montage_idx].repere)
}

/// Fige un composant placé (repère monde `corps`) en une [`Piece`] prête au
/// rendu : la transformée est cuite en `Mat4` (couche cuite).
pub fn cuire(corps: Repere, comp: Composant) -> Piece {
    Piece::new(corps.to_mat4(), comp)
}

/// Produit le **groupe symétrique** de `Piece` d'un composant placé en `corps` :
/// chaque transformation de la symétrie (dans le repère du parent — `axe` pour
/// la radiale, `normale` du plan pour le miroir) est appliquée **à gauche** de
/// la transformée cuite de base.
///
/// C'est ici que le miroir prend tout son sens : `symetrie::transformations`
/// renvoie des `Mat4` qui peuvent porter une **réflexion** (déterminant −1),
/// impossible à encoder dans le `Repere`/`Quat` de la couche construction — d'où
/// la cuisson en `Mat4`. La première copie est toujours l'originale.
pub fn cuire_symetrie(
    corps: Repere,
    comp: Composant,
    sym: Symetrie,
    axe: Vec3,
    normale: Vec3,
) -> Vec<Piece> {
    let base = corps.to_mat4();
    sym.transformations(axe, normale)
        .into_iter()
        .map(|t| Piece::new(t * base, comp))
        .collect()
}

/// Démo 2e : deux modules axiaux mis bout-à-bout par leurs écoutilles, assemblés
/// en une [`Station`] prête à dessiner. C'est le cas de validation visuelle du
/// raccordement complet (ports → accouplement → cuisson → assemblage → rendu).
pub fn demo_deux_modules() -> Station {
    let a = Composant::ModuleAxial { profil: Profil::P1, variante: VarianteModule::Standard, longueur: 3.0 };
    let b = Composant::ModuleAxial { profil: Profil::P1, variante: VarianteModule::Standard, longueur: 2.0 };

    let a_monde = Repere::IDENTITE;
    let hote = port_monde(a_monde, a, 0); // écoutille +Z libre de A
    let b_monde = poser(hote, b, 1); // B clipsé par son écoutille −Z

    let mut asm = Assembleur::new();
    asm.ajouter(cuire(a_monde, a)).ajouter(cuire(b_monde, b));
    match asm.terminer() {
        EtatStation::Prete(s) => s,
        EtatStation::Vide => unreachable!("deux pièces posées"),
    }
}

/// Démo « les quatre nœuds côte à côte » : de gauche à droite 4 sorties (croix
/// plane), 6 sorties (croix 3D), T (plan XZ), tétraèdre. Chaque nœud a un module
/// clipsé sur chacun de ses ports. Montre les variantes et le branchement radial.
pub fn demo_station() -> Station {
    let module = Composant::ModuleAxial { profil: Profil::P1, variante: VarianteModule::Standard, longueur: 1.5 };
    let mut asm = Assembleur::new();

    let cas = [
        (-12.0_f32, Sorties::Quatre),
        (-4.0, Sorties::Six),
        (4.0, Sorties::T),
        (12.0, Sorties::Tetra),
    ];
    for (dx, sorties) in cas {
        let noeud = Composant::Noeud { profil: Profil::P1, sorties };
        let n_monde = Repere::new(vec3(dx, 0.0, 0.0), Quat::IDENTITY);
        asm.ajouter(cuire(n_monde, noeud));
        // Un module sur chaque **bras** (ports module) — pas sur les ports Surface.
        for (idx, port) in noeud.ports().iter().enumerate() {
            if matches!(port.genre, GenrePort::ModuleAxial | GenrePort::ModuleRadial) {
                let hote = port_monde(n_monde, noeud, idx);
                asm.ajouter(cuire(poser(hote, module, 1), module));
            }
        }
    }
    match asm.terminer() {
        EtatStation::Prete(s) => s,
        EtatStation::Vide => unreachable!("quatre nœuds + modules"),
    }
}

/// Démo poutres seules : une **grille 2 styles × 6 gabarits**. Ligne du haut =
/// section carrée, ligne du bas = triangulaire ; 6 tailles du P0 (fin) au P3
/// (épais). Pour inspecter la structure sans rien d'autre.
pub fn demo_poutres() -> Station {
    // 6 gabarits croissants : (profil = épaisseur, longueur).
    let tailles = [
        (Profil::P0, 3.0_f32),
        (Profil::P0, 4.0),
        (Profil::P1, 5.0),
        (Profil::P1, 6.5),
        (Profil::P2, 8.0),
        (Profil::P3, 9.0),
    ];
    let styles = StyleTreillis::TOUS;

    let mut asm = Assembleur::new();
    for (r, style) in styles.iter().enumerate() {
        let dy = (r as f32 - (styles.len() as f32 - 1.0) / 2.0) * -7.0;
        for (c, (profil, longueur)) in tailles.iter().enumerate() {
            let dx = (c as f32 - (tailles.len() as f32 - 1.0) / 2.0) * 4.5;
            let truss = Composant::Treillis { profil: *profil, longueur: *longueur, style: *style };
            asm.ajouter(cuire(Repere::new(vec3(dx, dy, 0.0), Quat::IDENTITY), truss));
        }
    }
    match asm.terminer() {
        EtatStation::Prete(s) => s,
        EtatStation::Vide => unreachable!("poutres"),
    }
}

/// Démo ossature : une poutre (treillis) avec un **module à chaque bout** et, sur
/// ses ports hôtes `Surface`, un **mélange panneau / radiateur / antenne** clipsé
/// par port — la factorisation en action. Assemblé *réellement* par ports
/// (`poser`). Première station type ISS.
pub fn demo_treillis() -> Station {
    let truss = Composant::Treillis { profil: Profil::P1, longueur: 9.0, style: StyleTreillis::Carre };
    let module = Composant::ModuleAxial { profil: Profil::P1, variante: VarianteModule::Standard, longueur: 2.0 };
    // Trois appendices différents, montés sur les **mêmes** ports hôtes Surface.
    let panneau = Composant::PanneauSolaire { profil: Profil::P0, variante: VariantePanneau::RigideUS, longueur: 3.0, largeur: 1.4 };
    let radiateur = Composant::Radiateur { profil: Profil::P0, variante: VarianteRadiateur::PanneauSimple, longueur: 2.6, largeur: 1.2 };
    let antenne = Composant::Antenne { profil: Profil::P0, variante: VarianteAntenne::ParaboleGG, taille: 1.2 };

    let t = Repere::IDENTITE;
    let mut asm = Assembleur::new();
    asm.ajouter(cuire(t, truss));
    let mut k = 0usize;
    for (i, port) in truss.ports().iter().enumerate() {
        let hote = port_monde(t, truss, i);
        match port.genre {
            GenrePort::ModuleAxial => {
                asm.ajouter(cuire(poser(hote, module, 1), module));
            }
            GenrePort::Surface => {
                // Chaque niveau (paire ±X) reçoit un type d'appendice différent.
                let app = match (k / 2) % 3 {
                    0 => panneau,
                    1 => radiateur,
                    _ => antenne,
                };
                asm.ajouter(cuire(poser(hote, app, 0), app));
                k += 1;
            }
            _ => {}
        }
    }
    match asm.terminer() {
        EtatStation::Prete(s) => s,
        EtatStation::Vide => unreachable!("ossature"),
    }
}

/// Démo constructeur : la **même ossature**, mais assemblée via [`Chantier`] en
/// piochant dans les ports libres (pas de placement à la main). Prouve le
/// bookkeeping des ports — le fondement du générateur.
pub fn demo_chantier() -> Station {
    let module = Composant::ModuleAxial { profil: Profil::P1, variante: VarianteModule::Coupole, longueur: 2.0 };
    let panneau = Composant::PanneauSolaire { profil: Profil::P0, variante: VariantePanneau::RigideUS, longueur: 3.0, largeur: 1.4 };
    let radiateur = Composant::Radiateur { profil: Profil::P0, variante: VarianteRadiateur::PanneauSimple, longueur: 2.6, largeur: 1.2 };
    let antenne = Composant::Antenne { profil: Profil::P0, variante: VarianteAntenne::ParaboleGG, taille: 1.2 };

    let mut ch = Chantier::new();
    ch.racine(Composant::Treillis { profil: Profil::P1, longueur: 9.0, style: StyleTreillis::Carre });
    // Un module coiffe un bout axial (une seule fois pour ne pas enchaîner).
    if let Some(i) = ch.libres().iter().position(|p| p.genre == GenrePort::ModuleAxial) {
        ch.poser(i, module, 1);
    }
    // Snapshot des ports Surface (fixe : les appendices n'en ajoutent pas). On
    // itère cette liste : une pose rejetée (collision) est simplement ignorée,
    // sans jamais reboucler sur le même port.
    let cibles: Vec<Vec3> = ch.libres().iter().filter(|p| p.genre == GenrePort::Surface).map(|p| p.repere.pos).collect();
    for (k, pos) in cibles.iter().enumerate() {
        if let Some(i) = ch.libres().iter().position(|p| p.genre == GenrePort::Surface && p.repere.pos.distance(*pos) < 1e-3) {
            let app = match k % 3 {
                0 => panneau,
                1 => radiateur,
                _ => antenne,
            };
            ch.poser(i, app, 0); // peut échouer (collision) → on passe
        }
    }
    match ch.terminer() {
        EtatStation::Prete(s) => s,
        EtatStation::Vide => unreachable!("chantier"),
    }
}

/// Démo habitats : les 6 variantes de module (standard, doré, hublots, labo,
/// gonflable, coupole) alignées côte à côte, chaînées bout à bout par paires.
pub fn demo_habitats() -> Station {
    let mut asm = Assembleur::new();
    let tous = VarianteModule::TOUS;
    let n = tous.len();
    for (i, v) in tous.iter().enumerate() {
        let dx = (i as f32 - (n as f32 - 1.0) / 2.0) * 3.2;
        let m = Composant::ModuleAxial { profil: Profil::P1, variante: *v, longueur: 3.0 };
        // Modules le long de Z (leur axe), espacés sur X — détails (hublots,
        // fenêtre…) sur +Y, visibles.
        asm.ajouter(cuire(Repere::new(vec3(dx, 0.0, 0.0), Quat::IDENTITY), m));
    }
    match asm.terminer() {
        EtatStation::Prete(s) => s,
        EtatStation::Vide => unreachable!("habitats"),
    }
}

/// Démo antennes : les 6 variantes (paraboles, cornets, fouets, réseau, hélice)
/// alignées côte à côte, pointant vers le haut.
pub fn demo_antennes() -> Station {
    let mut asm = Assembleur::new();
    let tous = VarianteAntenne::TOUS;
    let n = tous.len();
    for (i, v) in tous.iter().enumerate() {
        let dx = (i as f32 - (n as f32 - 1.0) / 2.0) * 3.0;
        let ant = Composant::Antenne { profil: Profil::P0, variante: *v, taille: 1.4 };
        let base = Repere::new(vec3(dx, -1.0, 0.0), Quat::from_rotation_arc(Vec3::Z, Vec3::Y));
        asm.ajouter(cuire(base, ant));
    }
    match asm.terminer() {
        EtatStation::Prete(s) => s,
        EtatStation::Vide => unreachable!("antennes"),
    }
}

/// Démo radiateurs : les **7 variantes** alignées côte à côte, déployées vers le
/// haut (+Y). Six technologies réelles + le LDR (gouttelettes) exotique.
pub fn demo_radiateurs() -> Station {
    let mut asm = Assembleur::new();
    let tous = VarianteRadiateur::TOUS;
    let n = tous.len();
    for (i, v) in tous.iter().enumerate() {
        let dx = (i as f32 - (n as f32 - 1.0) / 2.0) * 3.6;
        let rad = Composant::Radiateur { profil: Profil::P0, variante: *v, longueur: 3.0, largeur: 1.3 };
        let base = Repere::new(vec3(dx, -2.0, 0.0), Quat::from_rotation_arc(Vec3::Z, Vec3::Y));
        asm.ajouter(cuire(base, rad));
    }
    match asm.terminer() {
        EtatStation::Prete(s) => s,
        EtatStation::Vide => unreachable!("variantes de radiateurs"),
    }
}

/// Démo panneaux : **toutes les variantes** alignées côte à côte, chaque pale
/// déployée vers le haut (+Y) pour bien voir sa face. Vitrine des styles.
pub fn demo_panneaux() -> Station {
    let mut asm = Assembleur::new();
    let tous = VariantePanneau::TOUS;
    let n = tous.len();
    for (i, v) in tous.iter().enumerate() {
        let dx = (i as f32 - (n as f32 - 1.0) / 2.0) * 3.4;
        let aile = Composant::PanneauSolaire { profil: Profil::P0, variante: *v, longueur: 3.0, largeur: 1.4 };
        // Base posée en bas, pale déployée vers le haut (+Y).
        let base = Repere::new(vec3(dx, -2.0, 0.0), Quat::from_rotation_arc(Vec3::Z, Vec3::Y));
        asm.ajouter(cuire(base, aile));
    }
    match asm.terminer() {
        EtatStation::Prete(s) => s,
        EtatStation::Vide => unreachable!("variantes de panneaux"),
    }
}

#[cfg(test)]
mod tests {
    use super::super::Profil;
    use super::*;

    fn proche(a: Vec3, b: Vec3) -> bool {
        (a - b).length() < 1e-5
    }

    // 2c — le cas de validation : deux modules mis bout à bout par leurs
    // écoutilles axiales. On vérifie coïncidence + face-à-face + hauts alignés
    // au joint, puis que la cuisson conserve la position.
    #[test]
    fn deux_modules_bout_a_bout() {
        let a = Composant::ModuleAxial { profil: Profil::P1, variante: VarianteModule::Standard, longueur: 3.0 };
        let b = Composant::ModuleAxial { profil: Profil::P1, variante: VarianteModule::Standard, longueur: 2.0 };

        // A posé à l'origine ; son écoutille +Z (port 0) est le port hôte libre.
        let a_monde = Repere::IDENTITE;
        let hote = port_monde(a_monde, a, 0);

        // B clipse son écoutille −Z (port 1) sur le port hôte de A.
        let b_monde = poser(hote, b, 1);

        // Au joint : le port de montage de B en monde coïncide avec le port hôte.
        let joint = port_monde(b_monde, b, 1);
        assert!(proche(joint.pos, hote.pos), "coïncidence {:?} / {:?}", joint.pos, hote.pos);
        // Face-à-face : avants opposés.
        assert!(proche(joint.avant(), -hote.avant()), "face-à-face");
        // Roulis : hauts alignés.
        assert!(proche(joint.haut(), hote.haut()), "hauts alignés");

        // Géométrie attendue : avec les collerettes de docking, le port +Z de A
        // est en z=1.75 (1.5 + col 0.25) ; B se cale donc centré en z=3.0.
        assert!(proche(b_monde.pos, vec3(0.0, 0.0, 3.0)));

        // Cuisson : la Piece porte la transformée, son centre = la position monde.
        let piece_b = cuire(b_monde, b);
        assert!(proche(piece_b.centre(), b_monde.pos));
        assert_eq!(piece_b.composant, b);
    }

    // Chaînage : un 3e module posé sur le port libre de B garde les invariants.
    #[test]
    fn chainage_trois_modules() {
        let m = Composant::ModuleAxial { profil: Profil::P1, variante: VarianteModule::Standard, longueur: 2.0 };

        let a_monde = Repere::IDENTITE;
        let b_monde = poser(port_monde(a_monde, m, 0), m, 1);
        // Port hôte libre de B = son écoutille +Z (port 0), en monde.
        let hote_b = port_monde(b_monde, m, 0);
        let c_monde = poser(hote_b, m, 1);

        let joint2 = port_monde(c_monde, m, 1);
        assert!(proche(joint2.pos, hote_b.pos), "coïncidence joint 2");
        assert!(proche(joint2.avant(), -hote_b.avant()), "face-à-face joint 2");
    }

    // ---- 2d : symétrie cuite ----

    // Miroir : 2 copies ; l'originale a un déterminant positif, la réflexion un
    // déterminant négatif (la réflexion est bien présente) ; centres reflétés à
    // travers le plan de normale X.
    #[test]
    fn miroir_donne_reflexion() {
        let comp = Composant::ModuleAxial { profil: Profil::P1, variante: VarianteModule::Standard, longueur: 2.0 };
        let corps = Repere::new(vec3(5.0, 3.0, 0.0), Quat::IDENTITY);

        let pieces = cuire_symetrie(corps, comp, Symetrie::Miroir, Vec3::ZERO, Vec3::X);

        assert_eq!(pieces.len(), 2);
        assert!(pieces[0].transforme.determinant() > 0.0, "originale : rotation");
        assert!(pieces[1].transforme.determinant() < 0.0, "copie : réflexion");
        assert!(proche(pieces[0].centre(), vec3(5.0, 3.0, 0.0)));
        assert!(proche(pieces[1].centre(), vec3(-5.0, 3.0, 0.0)), "reflété sur X");
    }

    // Radiale(4) : 4 copies à 90° autour de Y, toutes à même rayon, toutes de
    // déterminant positif (rotations pures).
    #[test]
    fn radiale_quatre_a_90_degres() {
        let comp = Composant::ModuleAxial { profil: Profil::P1, variante: VarianteModule::Standard, longueur: 2.0 };
        let corps = Repere::new(vec3(2.0, 0.0, 0.0), Quat::IDENTITY);

        let pieces = cuire_symetrie(corps, comp, Symetrie::Radiale(4), Vec3::Y, Vec3::ZERO);

        assert_eq!(pieces.len(), 4);
        for p in &pieces {
            assert!((p.centre().length() - 2.0).abs() < 1e-5, "même rayon");
            assert!(p.transforme.determinant() > 0.0, "rotation pure");
        }
        // La 2e copie = +90° autour de Y : (2,0,0) → (0,0,−2).
        assert!(proche(pieces[1].centre(), vec3(0.0, 0.0, -2.0)));
    }

    // Un module clipsé sur un port **radial** d'un nœud : coïncidence + face-à-face.
    #[test]
    fn module_sur_noeud_radial() {
        let n = Composant::Noeud { profil: Profil::P1, sorties: Sorties::Six };
        let m = Composant::ModuleAxial { profil: Profil::P1, variante: VarianteModule::Standard, longueur: 2.0 };

        let n_monde = Repere::IDENTITE;
        let hote = port_monde(n_monde, n, 2); // port radial +X du nœud
        let m_monde = poser(hote, m, 1); // module clipsé par son écoutille −Z

        let joint = port_monde(m_monde, m, 1);
        assert!(proche(joint.pos, hote.pos), "coïncidence radiale");
        assert!(proche(joint.avant(), -hote.avant()), "face-à-face radial");
    }
}
