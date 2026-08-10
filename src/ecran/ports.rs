//! État des ports libres face à la pièce en main — `docs/conception/assembleur.md`
//! §8.4, §10.3.
//!
//! **Trois couleurs, trois sens.** Un port compatible n'est pas forcément
//! posable : le profil peut convenir et l'anti-collision refuser. La distinction
//! doit se voir *sans cliquer*, sinon l'utilisateur essaie et ne comprend pas.
//!
//! Ce module décide, il ne dessine pas (§10.9).

use crate::vaisseau::{Chantier, Composant};

/// Ce qu'un port libre vaut pour la pièce actuellement en main (§8.4).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Etat {
    /// Vert : la pose aboutirait. `Chantier::peut_poser` → `true`.
    Posable,
    /// Rouge : genre et profil conviennent, mais **quelque chose est dans le
    /// chemin** (ou le budget manque). C'est le cas que §8.5 devra expliquer.
    Encombre,
    /// Éteint : ni le genre ni le profil ne conviennent. La pièce n'a même pas
    /// de port de montage à présenter à celui-ci.
    Incompatible,
}

/// État de chaque port libre, dans l'ordre de `Chantier::libres`.
///
/// `montage` est l'indice du port de montage retenu sur la pièce en main
/// (§10.5). Il compte : changer de montage change l'orientation, donc
/// l'encombrement, donc les couleurs — c'est pourquoi il entre dans la clé de
/// cache de l'écran au même titre que la version du chantier.
///
/// **Ce calcul est le coûteux** : un `peut_poser` par port, et chacun teste la
/// collision contre toutes les pièces. §10.3 le chiffre à 2 700 distances
/// d'enveloppes sur une station de 45 pièces — pour un résultat qui ne change
/// pas tant qu'on ne touche ni au vaisseau ni à la pièce en main. À garder d'une
/// image à l'autre, jamais à refaire par frame.
pub fn etats(chantier: &Chantier, comp: &Composant, montage: usize) -> Vec<(u64, Etat)> {
    // Les ports que ce montage accepte : une seule interrogation, au lieu d'un
    // test de compatibilité par port.
    let compatibles = chantier.compatibles(comp, montage);
    chantier
        .libres()
        .iter()
        .map(|p| {
            let etat = if !compatibles.contains(&p.id) {
                Etat::Incompatible
            } else if chantier.peut_poser(p.id, comp, montage) {
                Etat::Posable
            } else {
                Etat::Encombre
            };
            (p.id, etat)
        })
        .collect()
}

/// État de chaque port libre pendant le **déplacement** du sous-arbre `id`
/// (§10.10).
///
/// Même sortie que [`etats`], autre question. Un déplacement n'est pas une pose :
/// c'est tout un bloc qui bouge, sa place d'origine se libère, et les ports que
/// le bloc lui-même expose ne sont pas des destinations. `peut_poser` — qui juge
/// une pièce seule contre tout le vaisseau — répondrait donc à côté, et peindrait
/// en vert des ports que `deplacer` refuse.
///
/// D'où l'interrogation de `Chantier::peut_deplacer`, qui partage ses contrôles
/// avec `deplacer` (même `plan_deplacement`).
pub fn etats_deplacement(chantier: &Chantier, id: u64, montage: usize) -> Vec<(u64, Etat)> {
    let Some(piece) = chantier.piece(id) else { return Vec::new() };
    let comp = piece.composant.clone();
    let compatibles = chantier.compatibles(&comp, montage);
    chantier
        .libres()
        .iter()
        .map(|p| {
            let etat = if !compatibles.contains(&p.id) {
                Etat::Incompatible
            } else if chantier.peut_deplacer(id, p.id, montage) {
                Etat::Posable
            } else {
                Etat::Encombre
            };
            (p.id, etat)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vaisseau::{Composant, Profil, VarianteModule, VariantePanneau};

    fn module() -> Composant {
        Composant::ModuleAxial { profil: Profil::P1, variante: VarianteModule::Standard, longueur: 2.0 }
    }
    fn panneau() -> Composant {
        Composant::PanneauSolaire {
            profil: Profil::P0,
            variante: VariantePanneau::RigideUS,
            longueur: 2.0,
            largeur: 1.0,
        }
    }

    /// Chantier où les **trois** états se rencontrent : deux modules courts
    /// clipsés bout à bout. Leurs enveloppes se chevauchent assez pour que les
    /// écoutilles restantes soient compatibles **et** inutilisables.
    ///
    /// ⚠️ Partagé délibérément par les deux tests ci-dessous. Le premier
    /// vérifiait l'accord avec le moteur sur un chantier de modules **longs**,
    /// où aucun port n'est encombré : il ne parcourait donc jamais la branche
    /// rouge, et un sabotage supprimant l'anti-collision le laissait vert. Un
    /// test d'accord n'est bon que sur des données qui exercent tous les cas.
    fn chantier_aux_trois_etats() -> (Chantier, Composant) {
        let court =
            Composant::ModuleAxial { profil: Profil::P1, variante: VarianteModule::Standard, longueur: 1.0 };
        let mut ch = Chantier::new();
        ch.racine(court.clone());
        let id = ch.libres()[0].id;
        assert!(ch.poser(id, court.clone(), 0), "la pose de départ doit réussir");
        (ch, court)
    }

    /// **Un port, un état, et le même que celui du moteur.** Le test ne recopie
    /// pas la règle : il redemande à `compatibles` et `peut_poser`, qui sont ce
    /// que la pose réelle consultera.
    #[test]
    fn chaque_port_recoit_l_etat_que_le_moteur_confirme() {
        let (ch, comp) = chantier_aux_trois_etats();
        for m in 0..comp.ports().len() {
            let e = etats(&ch, &comp, m);
            assert_eq!(e.len(), ch.libres().len(), "un état par port libre, ni plus ni moins");
            let compat = ch.compatibles(&comp, m);
            for (id, etat) in &e {
                match etat {
                    Etat::Incompatible => assert!(!compat.contains(id), "éteint alors que le port accepte"),
                    Etat::Posable => assert!(ch.peut_poser(*id, &comp, m), "vert alors que la pose échouerait"),
                    Etat::Encombre => {
                        assert!(compat.contains(id), "rouge alors que le port n'accepte même pas");
                        assert!(!ch.peut_poser(*id, &comp, m), "rouge alors que la pose aboutirait");
                    }
                }
            }
        }
    }

    /// **Les trois états sont atteignables.** Sans ce test, `Encombre` pourrait
    /// être une branche morte et la distinction de §8.4 ne servirait à rien : on
    /// n'aurait qu'à peindre deux couleurs.
    ///
    /// Le cas a été **cherché, pas deviné**. Une première rédaction enchaînait
    /// des modules de longueur 2 en espérant qu'un port finisse bloqué : aucun
    /// ne l'était, et le test est parti rouge. Un balayage (longueur × montage)
    /// a montré où le rouge vit vraiment : deux modules **courts** clipsés bout
    /// à bout : leurs enveloppes se chevauchent assez pour que les écoutilles
    /// restantes soient compatibles mais inutilisables.
    ///
    /// Le vert et le rouge sont relevés sur des **montages différents du même
    /// chantier** : aucun montage ne donne les deux à la fois, et l'exiger
    /// aurait été une contrainte inventée pour le test.
    #[test]
    fn les_trois_etats_sont_atteignables() {
        let (ch, court) = chantier_aux_trois_etats();
        let compte = |m: usize, cible: Etat| {
            etats(&ch, &court, m).into_iter().filter(|(_, s)| *s == cible).count()
        };
        let n = court.ports().len();
        assert!(
            (0..n).any(|m| compte(m, Etat::Encombre) > 0),
            "aucun port encombré sur aucun montage : le rouge de §8.4 ne se voit jamais"
        );
        assert!(
            (0..n).any(|m| compte(m, Etat::Posable) > 0),
            "aucun port posable : plus rien ne se construirait"
        );
        assert!(
            (0..n).any(|m| compte(m, Etat::Incompatible) > 0),
            "aucun port éteint : le troisième état ne se voit jamais"
        );
    }

    /// **Une pièce sans montage compatible éteint tout** — et n'allume surtout
    /// pas du rouge, qui voudrait dire « ça pourrait, mais c'est encombré ».
    #[test]
    fn une_piece_sans_montage_compatible_eteint_tous_les_ports() {
        let mut ch = Chantier::new();
        ch.racine(module());
        // Montage 0 du panneau contre des ports d'écoutille/surface de module :
        // s'il n'accepte rien, tout doit être éteint.
        let e = etats(&ch, &panneau(), 0);
        let compat = ch.compatibles(&panneau(), 0);
        if compat.is_empty() {
            assert!(
                e.iter().all(|(_, s)| *s == Etat::Incompatible),
                "des ports allumés alors qu'aucun n'accepte ce montage"
            );
        }
    }

    /// **Pendant un déplacement, le vert veut dire « `deplacer` aboutirait ».**
    ///
    /// C'est la garde contre le port qui ment : `peut_poser` jugerait la pièce
    /// **seule** contre tout le vaisseau, alors qu'un déplacement fait bouger un
    /// bloc et libère sa place d'origine. Les deux réponses diffèrent, et c'est
    /// celle de `deplacer` qui compte, puisque c'est elle qui s'exécutera.
    #[test]
    fn le_vert_dun_deplacement_est_celui_que_deplacer_confirme() {
        let mut ch = Chantier::new();
        ch.racine(module());
        let p0 = ch.libres()[0].id;
        assert!(ch.poser(p0, module(), 1));
        let b = ch.derniere_piece().expect("une pièce posée");

        let mut verts = 0;
        for m in 0..module().ports().len() {
            for (port, etat) in etats_deplacement(&ch, b, m) {
                match etat {
                    Etat::Posable => {
                        assert!(ch.peut_deplacer(b, port, m), "vert, mais le déplacement échouerait");
                        verts += 1;
                    }
                    Etat::Encombre => {
                        assert!(!ch.peut_deplacer(b, port, m), "rouge, mais le déplacement aboutirait");
                    }
                    Etat::Incompatible => {}
                }
            }
        }
        assert!(verts > 0, "aucune destination verte : la garde ne s'exerce pas");
    }

    /// **Les ports du bloc déplacé ne sont jamais des destinations.** Se poser
    /// sur sa propre descendance est un cycle, que `deplacer` refuse : l'écran
    /// ne doit pas les peindre en vert.
    #[test]
    fn le_bloc_deplace_nest_pas_sa_propre_destination() {
        let mut ch = Chantier::new();
        ch.racine(module());
        let p0 = ch.libres()[0].id;
        ch.poser(p0, module(), 1);
        let b = ch.derniere_piece().unwrap();
        let bloc = ch.sous_arbre(b);

        let siens: Vec<u64> =
            ch.libres().iter().filter(|p| bloc.contains(&p.origine)).map(|p| p.id).collect();
        assert!(!siens.is_empty(), "le bloc n'expose aucun port : le test ne prouve rien");
        for m in 0..module().ports().len() {
            for (port, etat) in etats_deplacement(&ch, b, m) {
                if siens.contains(&port) {
                    assert_ne!(etat, Etat::Posable, "un port du bloc déplacé est proposé comme destination");
                }
            }
        }
    }

    /// **Le montage entre dans le résultat.** Changer de port de montage change
    /// l'orientation de la pièce, donc l'encombrement, donc les couleurs — c'est
    /// ce qui oblige à le mettre dans la clé de cache, et pas seulement la
    /// version du chantier.
    #[test]
    fn changer_de_montage_peut_changer_les_couleurs() {
        let mut ch = Chantier::new();
        ch.racine(module());
        let id = ch.libres()[0].id;
        ch.poser(id, module(), 1);

        let n = module().ports().len();
        let par_montage: Vec<Vec<(u64, Etat)>> = (0..n).map(|m| etats(&ch, &module(), m)).collect();
        assert!(
            par_montage.windows(2).any(|w| w[0] != w[1]),
            "tous les montages donnent le même tableau : le montage ne compterait pas dans la clé"
        );
    }
}

