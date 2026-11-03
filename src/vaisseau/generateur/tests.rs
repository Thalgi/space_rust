use crate::vaisseau::{
    Assembleur, Composant, EtatStation, Profil, Repere, StyleTreillis, ogive_rayon, BOUCLIER_ELANCEMENT, JUPE_COUVERTURE, VarianteModule, VarianteRaptor,
};
use macroquad::prelude::*;
use std::f32::consts::{FRAC_PI_2, PI, TAU};

// Ces tests affirment des choses sur des CONSTANTES, et clippy y voit une
// condition toujours vraie. Elle l'est — aujourd'hui. Le jour où quelqu'un
// abaisse la constante, l'assertion devient fausse et le test tombe : c'est
// précisément son rôle, garder la valeur ET dire pourquoi elle compte.
use super::moteur::*;
use super::presets::*;

    fn nb(etat: &EtatStation) -> usize {
        etat.doit_dessiner().map(|s| s.nb_pieces()).unwrap_or(0)
    }

    // ================= Starship : la grappe de six Raptor ==================
    //
    // Les cotes viennent du reel (`STARSHIP_COURONNE_*`), donc les tests ne
    // verifient pas des nombres mais des **rapports geometriques** : ce qui
    // rentre, ce qui ne se touche pas, ce qui affleure. Un rayon recopie a la
    // main ne dirait rien -- il verifierait sa propre copie.

    /// Demi-encombrement **lateral** d'un moteur pose : le rayon de sa capsule.
    /// Lu sur `enveloppe_locale`, donc les cannelures comptent -- et les moteurs
    /// n'etant jamais tournes, ce rayon local est aussi le rayon monde.
    fn rayon_lateral(c: &Composant) -> f32 {
        c.enveloppe_locale().rayon
    }

    /// Distance d'un moteur a l'axe du vaisseau.
    fn distance_axe(r: &Repere) -> f32 {
        r.pos.truncate().length()
    }

    // **Six moteurs, trois de chaque.** C'est l'anatomie (§D.4) ; s'en ecarter
    // n'est plus un Starship.
    #[test]
    fn le_starship_porte_trois_raptor_de_chaque_sorte() {
        let m = starship_moteurs();
        assert_eq!(m.len(), 6, "il faut six moteurs");
        let compte = |v: VarianteRaptor| {
            m.iter()
                .filter(|(c, _)| matches!(c, Composant::Raptor { variante, .. } if *variante == v))
                .count()
        };
        assert_eq!(compte(VarianteRaptor::Atmospherique), 3, "trois RSL au centre");
        assert_eq!(compte(VarianteRaptor::Vide), 3, "trois RVac en peripherie");
    }

    // ⚠️ **Rien ne depasse de la jupe.** Le RVac passe a ~8 cm du bord : c'est
    // serre sur l'engin reel aussi, et c'est exactement pour ca que ca se teste.
    // Elargir une cloche ou ecarter la couronne sans le voir ferait sortir les
    // moteurs de la silhouette.
    #[test]
    fn les_moteurs_tiennent_dans_la_jupe() {
        let coque = STARSHIP_RAYON.rayon();
        for (c, r) in starship_moteurs() {
            let bord = distance_axe(&r) + rayon_lateral(&c);
            assert!(
                bord <= coque,
                "un moteur deborde : bord a {bord:.4} pour une jupe de {coque:.4}"
            );
        }
    }

    // **Aucun moteur n'en traverse un autre.** Les deux couronnes se recouvrent
    // en z (le RVac descend plus bas que le RSL), donc seul l'ecart *lateral*
    // les separe.
    //
    // ⚠️ Ne pas confondre avec l'ecart **radial** entre les deux anneaux, qui ne
    // vaut que 0,057 U : ce n'est pas la distance entre deux moteurs. Decalees
    // de 60°, les couronnes laissent 1,19 U entre un RSL et son RVac voisin pour
    // 0,83 U requis. C'est le contact *dans* une couronne qui est le vrai risque
    // -- trois RSL trop serres se traversent bien avant de gener un RVac.
    #[test]
    fn les_moteurs_ne_se_traversent_pas() {
        let m = starship_moteurs();
        for i in 0..m.len() {
            for j in (i + 1)..m.len() {
                let (ci, ri) = &m[i];
                let (cj, rj) = &m[j];
                let ecart = (ri.pos.truncate() - rj.pos.truncate()).length();
                let besoin = rayon_lateral(ci) + rayon_lateral(cj);
                assert!(
                    ecart >= besoin,
                    "moteurs {i} et {j} imbriques : {ecart:.4} d'entraxe pour {besoin:.4} requis"
                );
            }
        }
    }

    // **Les six sorties sont dans le meme plan, et c'est le cul du vaisseau.**
    // Un Raptor crache vers −Z et son repere est au **milieu** de sa hauteur :
    // oublier la demi-hauteur ferait descendre la moitie du moteur sous le
    // vaisseau, qui ne mesurerait plus 50 m.
    #[test]
    fn les_sorties_de_tuyere_sont_toutes_au_cul() {
        let cul = starship_cul();
        for (c, r) in starship_moteurs() {
            let Composant::Raptor { variante, rayon, .. } = c else {
                panic!("un moteur qui n'est pas un Raptor");
            };
            let sortie = r.pos.z - variante.hauteur(rayon) * 0.5;
            assert!(
                (sortie - cul).abs() < 1e-5,
                "sortie de tuyere a {sortie:.4} au lieu du cul {cul:.4}"
            );
        }
    }

    // ⚠️ **La jupe couvre au moins deux tiers de chaque moteur.** C'est la
    // demande explicite, et elle se mesure : un lanceur ne montre pas ses
    // moteurs a nu. Le moteur le plus **court** est celui qui contraint --
    // a bord de jupe donne, c'est lui qui ressort le premier.
    #[test]
    fn la_jupe_couvre_les_deux_tiers_de_chaque_moteur() {
        let cul = starship_cul();
        let bord = cul + STARSHIP_JUPE_BORD;
        let haut = cul + STARSHIP_JUPE_HAUT;
        for (c, r) in starship_moteurs() {
            let Composant::Raptor { variante, rayon, .. } = c else { unreachable!() };
            let h = variante.hauteur(rayon);
            let (bas_m, haut_m) = (r.pos.z - h * 0.5, r.pos.z + h * 0.5);
            // Recouvrement des deux segments [bord, haut] et [bas_m, haut_m].
            let couvert = (haut_m.min(haut) - bas_m.max(bord)).max(0.0);
            let part = couvert / h;
            assert!(
                part >= JUPE_COUVERTURE - 1e-5,
                "{variante:?} couvert a {:.1} % au lieu de {:.1} % attendus",
                part * 100.0,
                JUPE_COUVERTURE * 100.0
            );
            // ...et il en reste quelque chose a voir : une jupe qui avale tout
            // reviendrait a ne pas avoir modele les moteurs.
            assert!(part < 1.0, "{variante:?} entierement avale par la jupe");
        }
    }

    // Les moteurs doivent tenir dans la **paroi interieure** de la jupe, pas
    // dans son rayon nominal : la paroi rentre, et un RVac passe a quelques
    // centimetres. Lu a la source (`jupe_rayon_interieur`).
    #[test]
    fn les_moteurs_tiennent_dans_la_paroi_de_jupe() {
        let dedans = crate::vaisseau::jupe_rayon_interieur(STARSHIP_RAYON);
        for (c, r) in starship_moteurs() {
            let bord = distance_axe(&r) + rayon_lateral(&c);
            assert!(
                bord <= dedans,
                "un moteur touche la paroi de jupe : {bord:.4} pour {dedans:.4}"
            );
        }
    }

    // ⚠️ **Les 50 m sont les 50 m.** La jupe est prise DANS la hauteur, pas
    // ajoutee dessous : coque + jupe doivent rendre exactement `STARSHIP_HAUTEUR`
    // du plan de sortie a la pointe. C'est l'erreur qu'on vient de corriger --
    // avant, les moteurs pendaient sous une coque deja haute de 50 m.
    #[test]
    fn le_starship_mesure_toujours_cinquante_metres() {
        let total = STARSHIP_JUPE_HAUT + STARSHIP_FUT + STARSHIP_NEZ;
        assert!(
            (total - STARSHIP_HAUTEUR).abs() < 1e-4,
            "hauteur hors-tout {total:.4} au lieu de {STARSHIP_HAUTEUR:.4}"
        );
        // Et rien ne descend sous le plan de sortie.
        let cul = starship_cul();
        for (c, r) in starship_moteurs() {
            let Composant::Raptor { variante, rayon, .. } = c else { unreachable!() };
            assert!(
                r.pos.z - variante.hauteur(rayon) * 0.5 >= cul - 1e-5,
                "une tuyere depasse sous le vaisseau"
            );
        }
    }

    // **Chaque couronne est reguliere** : trois moteurs a 120°, tous a la meme
    // distance de l'axe et a la meme cote. C'est ce que `Symetrie::Radiale(3)`
    // garantit -- le test tient la garantie, pas l'implementation.
    #[test]
    fn chaque_couronne_est_reguliere() {
        for (v, attendu) in [
            (VarianteRaptor::Atmospherique, STARSHIP_COURONNE_RSL),
            (VarianteRaptor::Vide, STARSHIP_COURONNE_RVAC),
        ] {
            let couronne: Vec<Repere> = starship_moteurs()
                .into_iter()
                .filter(|(c, _)| matches!(c, Composant::Raptor { variante, .. } if *variante == v))
                .map(|(_, r)| r)
                .collect();
            assert_eq!(couronne.len(), 3);
            for r in &couronne {
                assert!(
                    (distance_axe(r) - attendu).abs() < 1e-5,
                    "{v:?} a {:.4} de l'axe au lieu de {attendu:.4}",
                    distance_axe(r)
                );
                assert!(
                    (r.pos.z - couronne[0].pos.z).abs() < 1e-5,
                    "{v:?} : les moteurs d'une couronne ne sont pas a la meme cote"
                );
            }
            // Trois angles a 120° : les entraxes deux a deux sont tous egaux.
            let d = |a: &Repere, b: &Repere| (a.pos.truncate() - b.pos.truncate()).length();
            let (d01, d12, d20) = (
                d(&couronne[0], &couronne[1]),
                d(&couronne[1], &couronne[2]),
                d(&couronne[2], &couronne[0]),
            );
            assert!(
                (d01 - d12).abs() < 1e-4 && (d12 - d20).abs() < 1e-4,
                "{v:?} : couronne irreguliere ({d01:.4}, {d12:.4}, {d20:.4})"
            );
        }
    }

    // **Les RVac ne sont pas alignes derriere les RSL.** Decales d'un demi-pas,
    // le cul montre six directions ; alignes, il n'en montrerait que trois et
    // perdrait sa lecture. C'est le seul des six tests qui porte sur l'aspect
    // plutot que sur la mecanique -- d'ou son nom.
    #[test]
    fn les_deux_couronnes_sont_dephasees() {
        let angle = |v: VarianteRaptor| -> f32 {
            let r = starship_moteurs()
                .into_iter()
                .find(|(c, _)| matches!(c, Composant::Raptor { variante, .. } if *variante == v))
                .expect("couronne absente")
                .1;
            r.pos.y.atan2(r.pos.x)
        };
        let ecart = (angle(VarianteRaptor::Vide) - angle(VarianteRaptor::Atmospherique)).abs();
        // Modulo le pas de 120°, l'ecart doit valoir un demi-pas (60°).
        let pas = TAU / 3.0;
        let reste = ecart.rem_euclid(pas);
        let demi = pas * 0.5;
        assert!(
            (reste - demi).abs() < 1e-4,
            "dephasage de {:.1}° au lieu de {:.1}° : les RVac s'alignent derriere les RSL",
            reste.to_degrees(),
            demi.to_degrees()
        );
    }

    // ============================ Les 4 volets =============================

    // **Quatre volets, deux paires.** C'est l'anatomie (§D.4) : deux courts a
    // l'avant, deux longs a l'arriere.
    #[test]
    fn le_starship_porte_quatre_volets_en_deux_paires() {
        let v = starship_volets();
        assert_eq!(v.len(), 4, "il faut quatre volets");
        let mut envergures: Vec<f32> = v
            .iter()
            .map(|(c, _)| match c {
                Composant::Volet { longueur, .. } => *longueur,
                _ => panic!("ce n'est pas un volet"),
            })
            .collect();
        envergures.sort_by(f32::total_cmp);
        assert!(
            (envergures[0] - envergures[1]).abs() < 1e-5,
            "les deux volets avant n'ont pas la meme envergure"
        );
        assert!(
            (envergures[2] - envergures[3]).abs() < 1e-5,
            "les deux volets arriere n'ont pas la meme envergure"
        );
        assert!(
            envergures[2] > envergures[1] * 1.2,
            "les volets arriere doivent etre nettement plus grands que les avant"
        );
    }

    // **Les paires sont opposees.** Deux volets d'une meme paire sont dos a dos
    // autour de l'axe : c'est ce qui equilibre l'engin, et un ecart qui derive
    // ne se verrait pas a l'oeil sur un rendu de trois quarts.
    #[test]
    fn les_volets_vont_par_paires_opposees() {
        let v = starship_volets();
        for paire in v.chunks(2) {
            let (a, b) = (paire[0].1.pos, paire[1].1.pos);
            assert!((a.z - b.z).abs() < 1e-5, "une paire n'est pas a la meme cote");
            // Opposes : leur somme en XY est nulle.
            let somme = (a.truncate() + b.truncate()).length();
            assert!(somme < 1e-5, "paire non opposee, somme {somme:.4}");
        }
    }

    // **Les volets sont poses SUR la coque**, pas dedans ni en l'air : leur
    // charniere est exactement sur le rayon du fut.
    #[test]
    fn les_charnieres_sont_sur_la_peau() {
        let r = STARSHIP_RAYON.rayon();
        for (_, rep) in starship_volets() {
            let d = rep.pos.truncate().length();
            assert!(
                (d - r).abs() < 1e-5,
                "charniere a {d:.4} de l'axe pour une coque de {r:.4}"
            );
        }
    }

    // ⚠️ **Le volet s'ecarte vers l'exterieur, jamais vers l'interieur.** Le
    // signe de la rotation est l'erreur classique de ce genre de pose : a
    // l'envers, les quatre volets rentrent dans le reservoir et on ne voit
    // plus rien. Le test suit le **bout** du volet, seul temoin du sens.
    #[test]
    fn les_volets_secartent_vers_lexterieur() {
        for (c, rep) in starship_volets() {
            let Composant::Volet { longueur, braquage, .. } = c else { unreachable!() };
            let bout_local = crate::vaisseau::volet_bout(longueur, braquage);
            let bout = rep.pos + rep.rot * bout_local;
            assert!(
                bout.truncate().length() > rep.pos.truncate().length(),
                "un volet rentre dans la coque au lieu de s'en ecarter"
            );
        }
    }

    // ⚠️ **La corde suit l'axe du vaisseau, l'envergure en sort.**
    //
    // C'est l'orientation elle-meme, et rien d'autre ne la tenait : le volet est
    // dessine corde sur `X` et epaisseur sur `Z`, si bien qu'avec la seule
    // rotation d'azimut la corde s'enroulait **autour** de la coque et
    // l'epaisseur se couchait le long du vaisseau -- un volet pose a plat, dans
    // le mauvais sens. Aucun test ne le voyait : celui de la hauteur mesurait
    // `pos.z ± corde/2` sans regarder la rotation, donc il passait au vert quelle
    // que soit l'orientation. Il a fallu le voir a l'ecran.
    #[test]
    fn la_corde_des_volets_suit_laxe_du_vaisseau() {
        for (_, rep) in starship_volets() {
            let radial = vec3(rep.pos.x, rep.pos.y, 0.0).normalize();
            let corde = rep.rot * Vec3::X;
            let envergure = rep.rot * Vec3::Y;
            let epaisseur = rep.rot * Vec3::Z;
            // Non signee, et c'est **volontaire** : les deux volets d'une paire
            // sont en miroir, donc leurs cordes pointent vers des bouts opposes
            // du vaisseau. Ce qui doit s'accorder, c'est la face tuilee -- et
            // c'est `les_faces_tuilees_dune_paire_regardent_du_meme_cote` qui le
            // tient, pas une assertion de signe posee ici au hasard.
            assert!(
                corde.z.abs() > 0.999,
                "la corde ne suit pas l'axe du vaisseau : {corde:?}"
            );
            assert!(
                envergure.dot(radial) > 0.999,
                "l'envergure ne sort pas de la coque : {envergure:?}"
            );
            // L'epaisseur est donc tangentielle : ni radiale, ni axiale.
            assert!(
                epaisseur.z.abs() < 1e-3 && epaisseur.dot(radial).abs() < 1e-3,
                "l'epaisseur n'est pas tangentielle : {epaisseur:?}"
            );
        }
    }

    // ⚠️ **Les faces tuilees d'une paire regardent du meme cote.**
    //
    // C'est le defaut vu a l'ecran : le volet du cote `−Y` sortait retourne. Une
    // paire de volets est un **miroir** par rapport au plan du ventre, pas une
    // rotation -- sans demi-tour sur le second, l'un montre ses tuiles noires et
    // l'autre son inox, ce qui saute aux yeux sur l'engin monte et qu'aucune
    // garde de position ne pouvait voir.
    #[test]
    fn les_faces_tuilees_dune_paire_regardent_du_meme_cote() {
        let v = starship_volets();
        for paire in v.chunks(2) {
            // La face au vent est dessinee en `−Z` local (`coque::volet_dessiner`).
            let n0 = paire[0].1.rot * -Vec3::Z;
            let n1 = paire[1].1.rot * -Vec3::Z;
            assert!(
                n0.dot(n1) > 0.999,
                "les deux volets d'une paire tuilent des cotes opposes : {n0:?} contre {n1:?}"
            );
        }
    }

    // ⚠️ **Les volets restent sur la partie CYLINDRIQUE.**
    //
    // Leur charniere est posee au rayon du fut ; au-dela du raccord, l'ogive se
    // retrecit et la charniere flotterait au-dessus du vide. Le volet avant
    // etait a 18,2 U pour un raccord a 17,3 : il mordait sur le nez, et ca se
    // voyait de profil bien avant qu'un test ne le dise.
    #[test]
    fn les_volets_restent_sur_la_partie_cylindrique() {
        let cul = starship_cul();
        let raccord = cul + STARSHIP_HAUTEUR - STARSHIP_NEZ;
        let bord_jupe = cul + STARSHIP_JUPE_BORD;
        for (c, rep) in starship_volets() {
            let Composant::Volet { corde, .. } = c else { unreachable!() };
            let (bas, haut) = (rep.pos.z - corde * 0.5, rep.pos.z + corde * 0.5);
            assert!(
                haut <= raccord,
                "un volet mord sur l'ogive : {haut:.3} au-dessus du raccord {raccord:.3}"
            );
            assert!(
                bas >= bord_jupe,
                "un volet descend sous la jupe : {bas:.3} sous {bord_jupe:.3}"
            );
        }
    }

    // ⚠️ **Le bardage et les volets tuilent le MEME cote.**
    //
    // C'est la garde qui manquait a l'etape 4/5 : retourner les quatre volets
    // ensemble passait au vert, faute d'une reference de ventre. Le bardage la
    // fournit -- il regarde `+X` par construction -- et un engin qui porte ses
    // tuiles d'un cote et ses volets tuiles de l'autre rougit desormais.
    #[test]
    fn le_bardage_et_les_volets_tuilent_le_meme_cote() {
        // ⚠️ La reference est `STARSHIP_VENTRE`, pas le bardage. Les comparer
        // l'un a l'autre ne disait que leur **accord** : les retourner tous
        // ensemble passait au vert, et l'engin montrait son dos.
        let attendu = vec3(STARSHIP_VENTRE.cos(), STARSHIP_VENTRE.sin(), 0.0);
        let (_, rep_b) = starship_bardage();
        let ventre = rep_b.rot * Vec3::X;
        assert!(
            ventre.dot(attendu) > 0.999,
            "le bardage tuile {ventre:?} au lieu du ventre {attendu:?}"
        );
        for (_, rep) in starship_volets() {
            let face = rep.rot * -Vec3::Z;
            assert!(
                face.dot(attendu) > 0.999,
                "un volet tuile {face:?} au lieu du ventre {attendu:?}"
            );
        }
    }

    // Le ventre doit rester **a 90° des charnieres** : sur l'axe des volets, le
    // bardage passerait sous eux et la nappe serait coupee en deux.
    #[test]
    fn le_ventre_est_a_angle_droit_des_charnieres() {
        let ventre = vec3(STARSHIP_VENTRE.cos(), STARSHIP_VENTRE.sin(), 0.0);
        for (_, rep) in starship_volets() {
            let radial = vec3(rep.pos.x, rep.pos.y, 0.0).normalize();
            assert!(
                radial.dot(ventre).abs() < 1e-3,
                "un volet est plante dans le ventre : {radial:?} contre {ventre:?}"
            );
        }
    }

    // Le bardage couvre **exactement le fut** : pas la jupe (deja en inox nu et
    // d'un autre diametre utile), pas l'ogive (qui se retrecit, un manchon
    // cylindrique y flotterait).
    #[test]
    fn le_bardage_couvre_exactement_le_fut() {
        let cul = starship_cul();
        let (c, rep) = starship_bardage();
        let Composant::BardageTuiles { longueur, .. } = c else { unreachable!() };
        let (bas, haut) = (rep.pos.z - longueur * 0.5, rep.pos.z + longueur * 0.5);
        assert!(
            (bas - (cul + STARSHIP_JUPE_HAUT)).abs() < 1e-4,
            "le bardage ne part pas du haut de la jupe"
        );
        // ⚠️ Il ne s'arrete plus au raccord : il monte jusqu'a la pointe.
        let Composant::BardageTuiles { nez, .. } = c else { unreachable!() };
        assert!(
            (haut + nez - (cul + STARSHIP_HAUTEUR)).abs() < 1e-4,
            "le bardage ne monte pas jusqu'a la pointe"
        );
    }

    // ⚠️ **Ce que `braquage` veut dire**, et rien ne le disait.
    //
    // Sa documentation annoncait « 0 = replie contre la coque » alors que le
    // code fait l'inverse : 0 laisse le volet **droit**. Le commentaire a menti
    // pendant toute la pose des volets sans qu'un test s'en apercoive.
    #[test]
    fn le_braquage_part_de_zero_et_penche_vers_le_ventre() {
        let l = 3.0;
        let droit = crate::vaisseau::volet_bout(l, 0.0);
        assert!(droit.z.abs() < 1e-5, "a braquage nul le volet doit etre droit");
        assert!((droit.y - l).abs() < 1e-5, "a braquage nul il est tendu sur toute son envergure");

        let braque = crate::vaisseau::volet_bout(l, 1.0);
        assert!(braque.z < -1e-3, "a braquage plein le bout penche vers le ventre");
        assert!(braque.y < droit.y, "et se rapproche donc de la coque");

        // **C'est une rotation, pas un glissement** : le bout reste a
        // l'envergure de sa charniere quel que soit le braquage.
        for b in [0.0, 0.25, 0.5, 0.75, 1.0] {
            let d = crate::vaisseau::volet_bout(l, b).length();
            assert!((d - l).abs() < 1e-4, "braquage {b} : envergure {d:.4} au lieu de {l}");
        }
    }

    // Et le Starship porte les siens **droits** : c'est sa position au sol.
    #[test]
    fn les_volets_du_starship_sont_droits() {
        for (c, rep) in starship_volets() {
            let Composant::Volet { longueur, braquage, .. } = c else { unreachable!() };
            assert_eq!(braquage, 0.0, "les volets doivent etre droits au repos");
            // Le bout est alors purement radial : ni axial, ni tangentiel.
            let bout = rep.pos + rep.rot * crate::vaisseau::volet_bout(longueur, braquage);
            assert!(
                (bout.z - rep.pos.z).abs() < 1e-4,
                "un volet droit ne doit pas deriver le long du vaisseau"
            );
        }
    }

    // ⚠️ **Le vaisseau se montre DEBOUT, moteurs en bas.**
    //
    // Il est bati le long de `+Z` comme toute piece du parc, mais expose l'axe
    // sur `+Y`. La rotation ne s'applique qu'a l'assemblage : le reste des tests
    // raisonne dans le repere du vaisseau, ou les cotes se lisent.
    #[test]
    fn le_starship_se_dresse_moteurs_en_bas() {
        let debout = starship_debout();
        let axe = debout.rot * Vec3::Z;
        assert!(axe.dot(Vec3::Y) > 0.999, "le nez ne pointe pas vers le haut : {axe:?}");

        // Et sur l'engin monte. ⚠️ On compare des **origines** de pieces, pas
        // des etendues : le repere de la coque est en son milieu, si bien qu'un
        // hublot pose plus haut que lui sans rien depasser. Ce test n'affirme
        // donc que ce qu'une origine peut dire -- ce qui est en bas, et que
        // l'engin est bien plus haut que large.
        let etat = preset_starship();
        let station = etat.doit_dessiner().expect("station prete");
        let mut bas: Option<(f32, String)> = None;
        let (mut min, mut max) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
        for p in station.pieces() {
            let o = p.transforme.w_axis.truncate();
            let nom = p.composant.nom().to_string();
            if bas.as_ref().is_none_or(|(b, _)| o.y < *b) {
                bas = Some((o.y, nom));
            }
            min = min.min(o);
            max = max.max(o);
        }
        let (_, nom_bas) = bas.expect("des pieces");
        assert_eq!(nom_bas, "RAPTOR", "la piece la plus basse devrait etre un moteur");
        let etendue = max - min;
        assert!(
            etendue.y > etendue.x * 3.0 && etendue.y > etendue.z * 3.0,
            "le vaisseau est couche : etendue {etendue:?}"
        );
    }

    // ⚠️ **Les hublots sont sur l'OGIVE, et a l'OPPOSE du ventre.**
    //
    // Deux corrections vues a l'ecran. Sur le fut, ce seraient des hublots de
    // reservoir ; cote ventre, ce serait une baie vitree percee dans le
    // bouclier thermique, a l'endroit exact qui prend le plasma.
    #[test]
    fn les_hublots_sont_sur_le_nez_a_loppose_du_ventre() {
        let ventre = vec3(STARSHIP_VENTRE.cos(), STARSHIP_VENTRE.sin(), 0.0);
        let raccord = starship_cul() + STARSHIP_HAUTEUR - STARSHIP_NEZ;
        let pointe = starship_cul() + STARSHIP_HAUTEUR;
        let h = starship_hublots();
        assert_eq!(h.len(), 2, "deux hublots");
        for (_, rep) in &h {
            // Sur le nez : au-dessus du raccord, sous la pointe.
            assert!(
                rep.pos.z > raccord && rep.pos.z < pointe,
                "un hublot n'est pas sur l'ogive : z = {:.3} hors de [{raccord:.3}, {pointe:.3}]",
                rep.pos.z
            );
            // Cote dos : dos au ventre.
            let radial = vec3(rep.pos.x, rep.pos.y, 0.0).normalize();
            assert!(
                radial.dot(ventre) < -0.7,
                "un hublot est du cote tuile : {radial:?} contre un ventre {ventre:?}"
            );
        }
        assert!(h[0].1.pos.distance(h[1].1.pos) > 1e-3, "les deux hublots sont au meme endroit");
    }

    // ⚠️ **Le vitrage epouse la peau de l'ogive.**
    //
    // Deux choses qu'un placement radial rate sur un nez : le hublot se pose au
    // rayon de la **courbe** (et non a celui du fut, ou il flotterait), et sa
    // normale s'incline vers la pointe au lieu de sortir a plat. Les deux sont
    // relues sur `ogive_rayon`, la fonction que la coque dessine.
    #[test]
    fn les_hublots_epousent_la_peau_de_logive() {
        let r_base = STARSHIP_RAYON.rayon();
        let raccord = starship_cul() + STARSHIP_HAUTEUR - STARSHIP_NEZ;
        for (_, rep) in starship_hublots() {
            let dh = rep.pos.z - raccord;
            let attendu = ogive_rayon(r_base, STARSHIP_NEZ, STARSHIP_NEZ - dh);
            // ⚠️ **Projection sur l'azimut**, pas distance a l'axe. Les deux
            // hublots sont ecartes tangentiellement, ce qui eloigne de l'axe
            // sans rien dire du rayon ; tolerer cet ecart rendait le test
            // aveugle -- poser le hublot au rayon du fut (2,000 au lieu de
            // 1,865) passait au vert. La composante radiale, elle, est exacte :
            // le decalage tangentiel lui est perpendiculaire.
            let a = STARSHIP_VENTRE + PI;
            let dir = vec3(a.cos(), a.sin(), 0.0);
            let radiale = rep.pos.truncate().extend(0.0).dot(dir);
            assert!(
                (radiale - attendu).abs() < 1e-3,
                "hublot a {radiale:.3} de l'axe pour une peau a {attendu:.3}"
            );
            assert!(
                attendu < r_base,
                "a cette hauteur l'ogive devrait etre plus etroite que le fut"
            );
            // ⚠️ **Et assez large pour les porter.** Plus haut sur le nez, la
            // peau se retrecit vite : vers 90 % de la fleche elle ne fait plus
            // que 0,44 U, moins que la demi-paire (0,53). Les hublots
            // s'enrouleraient autour de la pointe. La hauteur exacte reste un
            // choix -- ceci n'en fixe que la limite physique.
            let demi_paire = HUBLOT_ECART * 0.5 + HUBLOT_RAYON;
            assert!(
                demi_paire < attendu * 0.9,
                "l'ogive n'a que {attendu:.3} de rayon pour une paire large de {demi_paire:.3}"
            );
            // La normale sort **et** penche vers la pointe.
            let n = rep.rot * Vec3::Z;
            let radial = vec3(rep.pos.x, rep.pos.y, 0.0).normalize();
            assert!(n.dot(radial) > 0.5, "le vitrage ne regarde pas dehors : {n:?}");
            assert!(
                n.z > 0.05,
                "le vitrage ne suit pas la pente de l'ogive : {n:?}"
            );
        }
    }

    // Les volets tiennent dans la **hauteur du vaisseau** : un volet avant qui
    // depasserait la pointe serait visible de tres loin.
    #[test]
    fn les_volets_restent_dans_la_hauteur_du_vaisseau() {
        let cul = starship_cul();
        let pointe = cul + STARSHIP_HAUTEUR;
        for (c, rep) in starship_volets() {
            let Composant::Volet { longueur, corde, braquage } = c else { unreachable!() };
            // Etendue **reelle** : les huit coins du caisson, tournes comme la
            // piece l'est. Mesurer `pos.z ± corde/2` reviendrait a supposer
            // l'orientation au lieu de la lire.
            let bout = crate::vaisseau::volet_bout(longueur, braquage);
            let r = crate::vaisseau::volet_rayon_capsule(corde);
            let mut bas = f32::MAX;
            let mut haut = f32::MIN;
            for pied in [Vec3::ZERO, bout] {
                for dx in [-r, r] {
                    for dz in [-r, r] {
                        let z = (rep.pos + rep.rot * (pied + vec3(dx, 0.0, dz))).z;
                        bas = bas.min(z);
                        haut = haut.max(z);
                    }
                }
            }
            assert!(bas >= cul, "un volet descend sous le cul ({bas:.3} < {cul:.3})");
            assert!(haut <= pointe, "un volet depasse la pointe ({haut:.3} > {pointe:.3})");
        }
    }

    // Le preset **pose vraiment** les six moteurs : la coque plus six pieces.
    // Sans ce test, `starship_moteurs` pourrait etre juste et n'etre appelee
    // par personne.
    #[test]
    fn le_preset_starship_assemble_coque_et_moteurs() {
        let etat = preset_starship();
        assert_eq!(nb(&etat), 15, "coque, jupe, 6 moteurs, 4 volets, 2 hublots et le bardage");
    }

    #[test]
    fn generer_produit_une_station() {
        let etat = generer(&ParamsStation { graine: 1, complexite: 2, style: Style::Historique, ossature: None });
        assert!(matches!(etat, EtatStation::Prete(_)));
        assert!(nb(&etat) >= 3, "au moins une ossature garnie");
    }

    /// Sommets cuits d'un état, dans l'ordre d'émission. C'est **ce que l'écran
    /// montre** — la seule mesure qui ne laisse rien passer.
    fn sommets(etat: &EtatStation) -> Vec<Vec3> {
        let mut v = Vec::new();
        if let Some(s) = etat.doit_dessiner() {
            let mut b = crate::vaisseau::maillage::Batisseur::new();
            for p in s.pieces() {
                b.poser_transforme(p.transforme);
                p.composant.dessiner(&mut b);
            }
            for lot in b.terminer() {
                v.extend(lot.vertices.iter().map(|x| x.position));
            }
        }
        v
    }

    // **Le déterminisme du générateur**, sur lequel repose l'idée même de
    // « graine » — et bientôt la sauvegarde d'un assemblage
    // (`conception/assembleur.md` §6.4 : rejouer une liste de poses doit
    // reproduire la géométrie au sommet près).
    //
    // ⚠️ Ce test comparait le **nombre de pièces**. Deux stations entièrement
    // différentes passaient donc au vert dès qu'elles avaient le même compte :
    // il prétendait garantir le déterminisme et ne garantissait rien
    // (`conception/assembleur.md` §5.4). Il mesure maintenant, du plus grossier
    // au plus fin : les pièces (composant + transformée), puis les sommets
    // cuits. Et il boucle sur plusieurs graines, styles et complexités — le
    // déterminisme est réclamé pour toutes, pas pour la seule qui était testée.
    #[test]
    fn generer_est_deterministe() {
        for graine in [0u64, 1, 42, 7919] {
            for style in Style::TOUS {
                for complexite in 1..=4u8 {
                    let p = ParamsStation { graine, complexite, style, ossature: None };
                    let (a, b) = (generer(&p), generer(&p));
                    let cas = format!("graine {graine} / {} / cplx {complexite}", style.nom());

                    let (pa, pb) = (a.doit_dessiner(), b.doit_dessiner());
                    assert_eq!(pa.is_some(), pb.is_some(), "{cas} : un tirage vide, l'autre non");
                    let (Some(pa), Some(pb)) = (pa, pb) else { continue };

                    assert_eq!(pa.nb_pieces(), pb.nb_pieces(), "{cas} : pièces en nombre différent");
                    for (i, (x, y)) in pa.pieces().iter().zip(pb.pieces()).enumerate() {
                        assert_eq!(x.transforme, y.transforme, "{cas} : pièce {i} mal placée");
                        assert!(x.composant == y.composant, "{cas} : pièce {i} d'un autre composant");
                    }
                    // Et jusqu'au bout : la géométrie cuite, sommet par sommet.
                    assert_eq!(sommets(&a), sommets(&b), "{cas} : géométrie cuite divergente");
                }
            }
        }
    }

    // Sur une même graine/style, une station complexe a **plus** de pièces.
    //
    // ⚠️ L'assertion était `grande >= petite`, que l'égalité satisfait : « la
    // complexité n'influe pas » passait donc au vert dans un test qui s'appelle
    // « la complexité influe ». Resserrée en `>`, et balayée sur 20 graines × 3
    // styles — sur une seule graine, le `>` ne dit rien de la tendance, il dit
    // seulement que ce tirage-là est monté.
    //
    // Le `>` strict est-il tenable ? Mesuré sur **600 combinaisons**
    // (200 graines × 3 styles) avant de le figer : zéro contre-exemple, et
    // l'écart le plus serré est de **33 pièces**. Ce n'est donc pas une
    // inégalité qu'on frôle, c'est une propriété franche du générateur.
    #[test]
    fn complexite_influe_sur_le_nombre_de_pieces() {
        for graine in 0..20u64 {
            for style in Style::TOUS {
                let a = |c| ParamsStation { graine, complexite: c, style, ossature: None };
                let petite = nb(&generer(&a(1)));
                let grande = nb(&generer(&a(4)));
                assert!(
                    grande > petite,
                    "graine {graine} / {} : cplx 4 → {grande} pièces, cplx 1 → {petite}",
                    style.nom()
                );
            }
        }
    }

    #[test]
    fn ossature_forcee_est_respectee_et_finie() {
        for oss in [Ossature::Iss, Ossature::Mir] {
            for g in 0..10u64 {
                let etat = generer(&ParamsStation { graine: g, complexite: 3, style: Style::Historique, ossature: Some(oss) });
                assert!(matches!(etat, EtatStation::Prete(_)), "{oss:?} graine {g}");
            }
        }
    }

    #[test]
    fn presets_iss_et_mir_produisent_des_stations() {
        assert!(nb(&preset_iss()) >= 5);
        assert!(nb(&preset_mir()) >= 4);
    }

    // Le repli doit amener le bras **le long de l'épine**, et le déploiement le
    // remettre radial — c'est tout l'intérêt du geste. Les deux bras pivotent
    // autour d'axes opposés, donc se rabattent du même côté, pas en ciseaux.
    #[test]
    fn le_repli_couche_les_bras_le_long_de_lepine() {
        for dehors in [Vec3::X, Vec3::NEG_X] {
            let axe = axe_pivot(dehors);
            // Déployé : le bras reste radial.
            let d0 = Quat::from_axis_angle(axe, 0.0) * dehors;
            assert!((d0 - dehors).length() < 1e-5);
            // Replié : le bras est aligné sur l'épine (+Z du repère de grappe),
            // et du **bon côté** — pas rabattu vers les moteurs.
            let d1 = Quat::from_axis_angle(axe, FRAC_PI_2) * dehors;
            assert!(
                (d1 - Vec3::Z).length() < 1e-5,
                "bras {dehors:?} replié vers {d1:?} au lieu de +Z"
            );
            // À mi-course, il est bien en diagonale (l'animation passe par là).
            let dm = Quat::from_axis_angle(axe, FRAC_PI_2 * 0.5) * dehors;
            assert!(dm.z > 0.6 && dm.dot(dehors) > 0.6);
        }
    }

    // La section d'équipage est montée **entre l'habitat et le bout de l'épine**.
    // Ce créneau est étroit et il n'est délimité par rien de visible : trop bas,
    // le collier tourne dans l'habitat fixe ; trop haut, il tourne autour de
    // rien. Ces deux bornes ne se voient qu'en tournant la caméra, d'où le test.
    #[test]
    fn la_section_dequipage_se_glisse_entre_lhabitat_et_le_bout_de_lepine() {
        // ⚠️ **Deux unités à ne pas mélanger.** Les `*_CENTRE_Y` sont des
        // positions le long de l'épine et sont multipliées par `ISV_ECHELLE` à la
        // pose ; les longueurs de composants, elles, ne le sont **pas** (la
        // charge utile garde sa taille, cf. `la_charge_utile_suit_le_gabarit_de_
        // lepine`). Comparer un centre brut à une demi-longueur donne un résultat
        // faux — conservateur ici, mais qui dériverait si `ISV_ECHELLE` bougeait.
        let hab_sommet = HAB_CENTRE_Y * ISV_ECHELLE + HAB_LONG * 0.5;
        let centre = EQUIPAGE_CENTRE_Y * ISV_ECHELLE;
        let collier_bas = centre - EQUIPAGE_COLLIER * 0.5;
        let collier_haut = centre + EQUIPAGE_COLLIER * 0.5;
        let epine_sommet = EPINE_SOMMET_Y * ISV_ECHELLE;

        assert!(
            collier_bas > hab_sommet,
            "le collier descend à {collier_bas} alors que l'habitat monte à {hab_sommet} : \
             la section tournerait dans les modules fixes"
        );
        assert!(
            collier_haut < epine_sommet,
            "le collier monte à {collier_haut}, au-delà du sommet d'épine \
             {epine_sommet} : il n'aurait plus de support"
        );
    }

    // Le collier doit **envelopper** l'épine sans laisser de jour : les deux
    // bornes sont serrées et se contredisent si on les relâche, d'où le test.
    //
    // ⚠️ Ce test dit l'**inverse** de son ancêtre, qui exigeait un jeu franc
    // autour de la flèche comme le voudrait un vrai palier. Cette version-là
    // donnait un anneau de vide visible tout autour, et la pièce avait l'air
    // enfilée sur l'épine plutôt que montée dessus (arbitrage rendu à l'écran le
    // 2026-07-30 : le rendu l'emporte sur le mécanisme). Ne pas « réparer » le
    // sens de ces assertions sans avoir regardé la vue.
    #[test]
    fn le_collier_dequipage_enveloppe_lepine_sans_jour() {
        let r_collier = EQUIPAGE_COLLIER_RAYON;

        // Borne basse : l'alésage passe **sous** le hors-tout de la flèche, donc
        // sous sa section inscrite. Les membrures mordent dans la paroi du
        // tambour et il ne reste rien à voir entre les deux.
        assert!(
            EQUIPAGE_ALESAGE < EPINE,
            "alésage {EQUIPAGE_ALESAGE:.3} ≥ épine {EPINE:.3} : le jour redevient visible"
        );
        // Borne haute : la jaquette doit dépasser le hors-tout de la flèche,
        // sinon les longerons ressortent **à travers** le collier.
        assert!(
            r_collier > EPINE,
            "jaquette {r_collier:.3} ≤ épine {EPINE:.3} : les longerons traversent le collier"
        );
        // Et il reste de la matière : un tambour n'est pas une feuille.
        assert!(
            r_collier - EQUIPAGE_ALESAGE > 0.5,
            "paroi de collier trop mince ({:.3})",
            r_collier - EQUIPAGE_ALESAGE
        );
        // Les bras partent de la jaquette : l'envergure doit la dépasser, sinon
        // les modules seraient posés dans le collier lui-même.
        assert!(
            EQUIPAGE_BRAS > r_collier,
            "envergure {EQUIPAGE_BRAS} ≤ rayon de collier {r_collier}"
        );
    }

    // La tête est un **étagement**, pas un blindage feuilleté : c'est le vide
    // entre plaques qui laisse au nuage de plasma la place de s'étaler. Une pile
    // resserrée ne pare plus rien à 0,7 c. Ce test garde donc le rapport
    // écart/plaque, qui est la seule chose que le montage doit *dire*.
    #[test]
    fn la_tete_reste_un_etagement() {
        assert!(
            BOUCLIER_GRANDS >= 3,
            "moins de 3 grandes plaques : l'étagement ne se lit plus"
        );
        // L'espacement doit rester du même ordre que la plaque, pas un joint.
        let rapport = BOUCLIER_ECART / BOUCLIER_GRAND_RAYON;
        assert!(
            rapport > 0.5,
            "écart {BOUCLIER_ECART} pour un rayon {BOUCLIER_GRAND_RAYON} (rapport {rapport:.2}) : \
             les plaques se touchent presque, l'étagement ne se lit plus"
        );
        // Mais la pile doit rester une **tête**, pas un fuselage : plus longue
        // que haute et elle cesse de lire comme un bouclier.
        let prof = (BOUCLIER_GRANDS - 1) as f32 * BOUCLIER_ECART;
        let haut = 2.0 * BOUCLIER_GRAND_RAYON * BOUCLIER_ELANCEMENT;
        assert!(
            prof < haut,
            "pile longue de {prof:.1} pour une plaque haute de {haut:.1} : c'est un fût, plus une tête"
        );
        // Et la petite plaque doit rester la **petite** : c'est elle qui prend
        // le premier choc, et le schéma la veut nettement en retrait.
        assert!(
            BOUCLIER_PETIT_RAYON < BOUCLIER_GRAND_RAYON * 0.85,
            "petite plaque de {BOUCLIER_PETIT_RAYON} contre {BOUCLIER_GRAND_RAYON} : elles ne se distinguent plus"
        );
    }

    // La section a été **divisée par deux** le 2026-07-30 parce qu'elle écrasait
    // la silhouette. Ce test garde la trace du gabarit retenu : c'est un choix
    // d'échelle validé à l'œil, donc exactement le genre de valeur qu'une
    // retouche ultérieure risque de défaire sans s'en apercevoir.
    #[test]
    fn la_section_dequipage_reste_a_lechelle_du_vaisseau() {
        let demi_envergure = EQUIPAGE_BRAS + EQUIPAGE_LONG;
        // Elle doit rester **sous** la moitié de l'ancien gabarit (16), à la
        // constante près qu'impose le collier.
        assert!(
            demi_envergure < 10.0,
            "demi-envergure {demi_envergure} : la section redomine la silhouette"
        );
        // Mais rester lisible : une section qui ne dépasse plus du fret n'a plus
        // l'air de tourner autour de quoi que ce soit.
        assert!(
            demi_envergure > FRET_RAYON,
            "demi-envergure {demi_envergure} sous le rayon du fret {FRET_RAYON:.3}"
        );
        // Bras et module gardent leur proportion d'origine (≈ 1:1) : c'est ce
        // rapport qui fait lire « nacelle au bout d'une traverse ».
        let bras_utile = EQUIPAGE_BRAS - EQUIPAGE_COLLIER_RAYON;
        let rapport = bras_utile / EQUIPAGE_LONG;
        assert!(
            (0.7..1.4).contains(&rapport),
            "bras {bras_utile} / module {EQUIPAGE_LONG} = {rapport:.2}, hors du 1:1 voulu"
        );
    }

    // La charge utile garde sa taille, mais elle est **calée sur le gabarit de
    // l'épine** : si `ISV_ECHELLE` bouge, rayons de couronne et portées de
    // ferrure doivent suivre tout seuls. C'est exactement ce qui a été raté une
    // fois — l'épine élargie de 20 % et les ferrures d'habitat encore calculées
    // pour l'ancienne, donc plantées **dans** la structure.
    #[test]
    fn la_charge_utile_suit_le_gabarit_de_lepine() {
        // **Les deux variantes d'épine sont vérifiées.** L'hexagonale est 3,2 %
        // plus large hors-tout : c'est peu, et c'est justement l'ordre de
        // grandeur qui replante la charge utile dans la structure sans qu'on le
        // remarque (§C.6). Boucler ici est ce qui garantit qu'ajouter une
        // variante d'épine ne peut pas passer sans recaler les couronnes.
        for epine in [Epine::Carree, Epine::Hexagonale] {
            let g = epine.hors_tout();
            let nom = epine.nom();

            // Fret : le creux central de la triforce laisse passer l'épine.
            let creux = fret_rayon(g) - FRET_NACELLE * (0.5 + 0.5 * 0.22);
            assert!(creux >= g, "{nom} : creux du fret {creux:.3} < épine {g:.3}");

            // Habitat : le longeron de ferrure vient **sur** l'épine — ni enfoncé
            // dedans, ni suspendu dans le vide.
            let rail = hab_rayon(g) - HAB_INSCRIT - HAB_ATTACHE * 0.5;
            assert!(
                (rail - g).abs() < 1e-3,
                "{nom} : ferrure d'habitat à {rail:.3}, épine à {g:.3}"
            );
            // ...et le fût lui-même reste à distance de l'épine.
            let face = hab_rayon(g) - HAB_INSCRIT;
            assert!(face > g, "{nom} : l'habitat mord l'épine ({face:.3} vs {g:.3})");

            // Collier d'équipage : sa jaquette doit toujours dépasser la flèche,
            // sinon les longerons ressortent à travers (cf. §C.8). C'est la marge
            // la plus serrée du vaisseau, et l'épine hexagonale la rogne encore.
            assert!(
                EQUIPAGE_COLLIER_RAYON > g,
                "{nom} : jaquette de collier {EQUIPAGE_COLLIER_RAYON} <= épine {g:.3}"
            );
            // ...et son alésage doit rester **sous** la flèche, pour qu'aucun jour
            // ne se rouvre entre le tambour et la structure.
            assert!(
                equipage_alesage(g) < g,
                "{nom} : alésage de collier {:.3} >= épine {g:.3}",
                equipage_alesage(g)
            );

            // **Les trois modules ne se gênent pas entre eux.** Support d'une
            // section onigiri P2 dans la direction de sa voisine (à 60° d'un
            // coin) : `dv·cos 30° + ρ`. C'est ce qui dit que le serrage n'est
            // borné que par l'épine — sans ce calcul, le « plancher » du jeu ne
            // serait qu'une supposition.
            let (dv, rho) = (2.0 * (1.0 - 0.22), 2.0 * 0.22);
            let portee = dv * (PI / 6.0).cos() + rho;
            let entre_axes = 2.0 * hab_rayon(g) * (PI / 3.0).sin();
            assert!(
                entre_axes > 2.0 * portee,
                "{nom} : modules d'habitat qui se croisent ({entre_axes:.3} <= {:.3})",
                2.0 * portee
            );
        }
    }

    // Les deux cuves d'un même côté ne doivent plus se traverser. Cote purement
    // sphérique : deux sphères de rayon `r` se touchent à `2r`, il faut donc
    // davantage.
    #[test]
    fn les_cuves_de_carburant_ne_se_traversent_pas() {
        let res_long = 5.0_f32;
        let res_r = res_long * 0.5 * 1.3;
        let ecart = 2.0 * res_r + RESERVOIR_JEU;
        assert!(
            ecart > 2.0 * res_r,
            "écart {ecart:.3} pour des cuves de rayon {res_r:.3} : elles se traversent"
        );
        // Et l'ancien écart, celui des plaques hexagonales, est bien insuffisant :
        // c'est ce qui justifie d'avoir découplé les deux cotes.
        let sg = Profil::P3.rayon() * 0.5;
        let ap = 2.0 * sg * 3.0_f32.sqrt() * 0.5;
        assert!(
            2.0 * ap < 2.0 * res_r,
            "l'écart des plaques ({:.3}) suffirait aux cuves : le découplage serait inutile",
            2.0 * ap
        );
    }

    // **La propulsion doit maintenant TOUCHER le pied de l'épine**, pas le dégager.
    //
    // ⚠️ Ce test a changé de sens **deux fois**, et chaque fois délibérément — ce
    // n'est pas une dérive, c'est l'intention de conception qui a bougé :
    //   1. « jeu franc > 0,4 » — la propulsion pendait à côté du pied et le
    //      traversait, il fallait l'écarter ;
    //   2. « pas de recouvrement franc > −0,25 » — déport resserré, contact toléré ;
    //   3. **ici** : engagement **exigé**, la propulsion devant se lire comme montée
    //      sur l'épine et non suspendue à côté.
    // Ne pas « corriger » le sens de ces assertions sans avoir regardé la vue.
    //
    // Mesuré sur la géométrie cuite du preset, jamais sur les cotes : le rayon
    // interne de l'aile n'est pas déductible du déport (cf. `PROPULSION_DEPORT`).
    #[test]
    fn la_propulsion_touche_le_pied_de_lepine() {
        use crate::vaisseau::maillage::Batisseur;
        let EtatStation::Prete(s) = preset_isv_hexa() else { panic!("preset vide") };

        // Le modèle est couché : l'axe du vaisseau est X, le rayon se lit en Y-Z.
        let mut pied: Vec<Vec3> = Vec::new(); // sommets du pied, côté moteurs
        let mut aile: Vec<Vec3> = Vec::new(); // sommets des ailes radiateur
        for p in s.pieces() {
            let charpente = matches!(&p.composant, Composant::CharpenteHexa { .. });
            let radiateur = matches!(&p.composant, Composant::RadiateurMega { .. });
            if !charpente && !radiateur {
                continue;
            }
            let mut b = Batisseur::new();
            p.composant.dessiner(&mut b);
            for lot in b.terminer() {
                for v in &lot.vertices {
                    let w = p
                        .transforme
                        .transform_point3(vec3(v.position[0], v.position[1], v.position[2]));
                    // Bande du pied : au-delà de la base du cône, côté tuyères.
                    if charpente && w.x < -10.0 {
                        pied.push(w);
                    }
                    if radiateur {
                        aile.push(w);
                    }
                }
            }
        }

        // ⚠️ **Les deux mesures doivent porter sur la même tranche du vaisseau.**
        // Comparer le rayon minimal de l'aile au rayon maximal du pied *sans* cette
        // précaution rend un « engagement » qui n'existe pas dès que les deux pièces
        // ne sont plus à la même hauteur : c'est exactement ce que produit
        // `PROPULSION_AVANCE`, qui fait glisser l'aile sous le fût. On restreint donc
        // au **recouvrement axial** des deux, et son absence est en soi un échec.
        let borne = |v: &[Vec3], f: fn(f32, f32) -> f32, init: f32| {
            v.iter().fold(init, |acc, w| f(acc, w.x))
        };
        let bas = borne(&pied, f32::max, f32::MIN).min(borne(&aile, f32::max, f32::MIN));
        let haut = borne(&pied, f32::min, f32::MAX).max(borne(&aile, f32::min, f32::MAX));
        // Et ce recouvrement doit rester **substantiel**. Réduit à un liseré, il ne
        // porte plus que quelques sommets de bord et la mesure part en vrille : à
        // `PROPULSION_AVANCE = 1,5` la bande tombe à 0,1 et le « jeu » bascule à
        // +1,34, c'est-à-dire l'inverse de la réalité. Une mesure sur trop peu de
        // matière est pire qu'aucune mesure.
        assert!(
            bas - haut > 0.3,
            "recouvrement axial de {:.2} seulement (bande [{haut:.1} ; {bas:.1}]) :              l'aile a glissé sous le fût, la mesure n'a plus de sens",
            bas - haut
        );

        let pied_max = pied
            .iter()
            .filter(|w| (haut..=bas).contains(&w.x))
            .fold(0.0f32, |m, w| m.max(w.yz().length()));
        let radia_min = aile
            .iter()
            .filter(|w| (haut..=bas).contains(&w.x))
            .fold(f32::MAX, |m, w| m.min(w.yz().length()));
        let jeu = radia_min - pied_max;
        // **Elle mord dans le pied** : c'est cet engagement qui la fait lire comme
        // montée sur l'épine. Un simple affleurement (−0,11, l'état précédent) ne
        // suffit pas — de trois quarts, il laisse encore voir un jour entre les deux.
        assert!(
            jeu < -0.3,
            "engagement de {jeu:.2} — radiateur à {radia_min:.2}, pied à {pied_max:.2} : \
             la propulsion effleure l'épine au lieu de s'y poser"
        );
        // **Mais elle ne l'avale pas.** Au-delà, l'aile traverserait le fût de part
        // en part et ressortirait de l'autre côté.
        assert!(
            jeu > -3.0,
            "engagement de {jeu:.2} : l'aile traverse le pied au lieu de s'y ancrer"
        );
    }

    // Les tests ci-dessus vérifient les **formules**. Celui-ci vérifie le
    // **câblage** : que `isv()` passe bien le gabarit de son épine à la charge
    // utile, au lieu de lire la constante carrée. C'est une erreur invisible aux
    // tests de formules — et c'est littéralement le bug de §C.6, où le calcul
    // était juste mais appliqué à l'ancien gabarit.
    #[test]
    fn le_second_isv_recale_vraiment_sa_charge_utile() {
        let cotes = |e: EtatStation| {
            let EtatStation::Prete(s) = e else { panic!("preset vide") };
            let mut ratelier = None;
            let mut alesage = None;
            let mut hexa = false;
            let mut carree = false;
            for p in s.pieces() {
                match &p.composant {
                    Composant::RatelierCargo { rayon, .. } => ratelier = Some(*rayon),
                    Composant::CollierRotatif { alesage: a, .. } => alesage = Some(*a),
                    Composant::CharpenteHexa { .. } => hexa = true,
                    Composant::Charpente { .. } => carree = true,
                    _ => {}
                }
            }
            (ratelier.expect("pas de fret"), alesage.expect("pas de collier"), hexa, carree)
        };

        let (r_c, a_c, hexa_c, carree_c) = cotes(preset_isv());
        let (r_h, a_h, hexa_h, carree_h) = cotes(preset_isv_hexa());

        // Chacun porte bien la charpente qu'il annonce, et **seulement** elle.
        assert!(carree_c && !hexa_c, "le premier ISV n'est plus en épine carrée");
        assert!(hexa_h && !carree_h, "le second ISV n'est pas en épine hexagonale");

        // La couronne de fret et l'alésage du collier ont suivi le gabarit.
        let g = Epine::Hexagonale.hors_tout();
        assert!(r_h > r_c, "couronne de fret non recalée ({r_h:.4} vs {r_c:.4})");
        assert!((r_h - fret_rayon(g)).abs() < 1e-4, "fret à {r_h:.4}, attendu {:.4}", fret_rayon(g));
        assert!(a_h > a_c, "alésage de collier non recalé ({a_h:.4} vs {a_c:.4})");
        assert!((a_h - equipage_alesage(g)).abs() < 1e-4);

        // Et le premier ISV est resté sur ses cotes historiques.
        assert!((r_c - FRET_RAYON).abs() < 1e-4);
        assert!((a_c - EQUIPAGE_ALESAGE).abs() < 1e-4);
    }

    // Les constantes historiques doivent rester **exactement** celles de l'épine
    // carrée : le second preset ne doit rien changer au premier. Sans ça, on
    // comparerait deux vaisseaux qui diffèrent par autre chose que leur épine.
    #[test]
    fn le_gabarit_carre_est_inchange_par_lajout_de_lhexagonal() {
        assert!((Epine::Carree.hors_tout() - EPINE).abs() < 1e-6);
        assert!((fret_rayon(EPINE) - FRET_RAYON).abs() < 1e-6);
        assert!((hab_rayon(EPINE) - HAB_RAYON).abs() < 1e-6);
        assert!((equipage_alesage(EPINE) - EQUIPAGE_ALESAGE).abs() < 1e-6);
        // Et l'hexagonale est bien **plus large**, sinon tout ce recalage serait
        // du vent.
        let h = Epine::Hexagonale.hors_tout();
        assert!(h > EPINE, "épine hexagonale {h:.4} pas plus large que {EPINE:.4}");
        let ecart = h / EPINE - 1.0;
        assert!(
            (0.02..0.05).contains(&ecart),
            "écart de gabarit {:.1} %, hors du ~3 % attendu",
            ecart * 100.0
        );
    }

    // **Le panache ne doit pas lécher le vaisseau.** C'est la raison d'être du
    // braquage de 5° des tuyères, décidé bien avant qu'il y ait un panache à
    // regarder : l'ISV est un tracteur, il remorque sa charge utile **dans l'axe
    // de ses propres jets**, et sans ce braquage il la baignerait dans un plasma
    // de pions. Un panache trop ouvert annule le braquage et ramène le problème.
    //
    // Rien de tout ça ne se voit sur le composant seul : c'est un rapport entre
    // trois choses — l'angle des tuyères, l'évasement du jet, et le gabarit de
    // ce qui est remorqué derrière.
    #[test]
    fn le_panache_ne_leche_pas_la_charge_utile() {
        let EtatStation::Prete(s) = preset_isv_fixe(Epine::Hexagonale, 1.0) else {
            panic!("l'ISV doit être prête");
        };
        // Axe du jet de chaque tuyère, dans le repère du vaisseau fini.
        let jets: Vec<(Vec3, Vec3)> = s
            .pieces()
            .iter()
            .filter(|p| matches!(p.composant, Composant::Panache { .. }))
            .map(|p| {
                (
                    p.transforme.transform_point3(Vec3::ZERO),
                    p.transforme.transform_vector3(Vec3::Z).normalize(),
                )
            })
            .collect();
        assert_eq!(jets.len(), 2, "un panache par tuyère");

        // Tout ce qui est **remorqué** : la charge utile et la tête, c'est-à-dire
        // ce qui se trouve en aval des moteurs. L'ossature de propulsion est
        // exclue — le jet en sort, il la frôle forcément.
        let remorque: Vec<(Vec3, f32)> = s
            .pieces()
            .iter()
            .filter(|p| {
                matches!(
                    p.composant,
                    Composant::RatelierCargo { .. }
                        | Composant::ModuleHabitat { .. }
                        | Composant::ModuleEquipage { .. }
                        | Composant::BouclierPetit { .. }
                        | Composant::BouclierGrand { .. }
                )
            })
            .map(|p| (p.centre(), p.composant.rayon_local()))
            .collect();
        assert!(remorque.len() >= 8, "{} pièces remorquées : trop peu", remorque.len());

        let mut pire = f32::MAX;
        let mut coupable = Vec3::ZERO;
        for (origine, axe) in &jets {
            for (centre, rayon) in &remorque {
                // Distance de la pièce à l'axe du jet, moins le rayon du jet à
                // cette distance-là : c'est le jeu réel entre les deux volumes.
                let le_long = (*centre - *origine).dot(*axe);
                if le_long <= 0.0 {
                    continue; // en amont de la tuyère : le jet ne va pas par là
                }
                let ecart = (*centre - *origine - *axe * le_long).length();
                let t = le_long / (PANACHE_LONGUEUR);
                let demi_jet = crate::vaisseau::composant::rayon_panache(
                    Profil::P1.rayon() / 0.40 * PANACHE_COL,
                    PANACHE_BOUT,
                    t,
                );
                let jeu = ecart - demi_jet - rayon;
                if jeu < pire {
                    pire = jeu;
                    coupable = *centre;
                }
            }
        }
        assert!(
            pire > 1.0,
            "le panache passe à {pire:.1} d'une pièce remorquée (centre {coupable:?}) : \
             il la baigne au lieu de la contourner"
        );
    }

    // Le bardage se monte **au droit des moteurs**, donc sur la portion où
    // l'épine s'ouvre vers son pied — de 2,81 à 1,28 de rayon sur les treize
    // unités couvertes. Il doit donc l'**épouser**, et il n'y a que deux façons
    // de rater ça, opposées et toutes deux voyantes : trop serré, l'épine
    // ressort au travers ; trop lâche, le bardage flotte autour comme une gaine.
    //
    // ⚠️ La comparaison qui compte est **la surface la plus rentrante du bardage
    // contre la plus saillante de l'épine**, et non leurs deux circonradius. Les
    // deux hexagones ne sont calés sur aucune orientation commune : au pire ils
    // sont décalés d'un demi-pas, et c'est alors le **milieu de facette** du
    // bardage (rayon inscrit = 0,866 × circonradius) qui passe au droit d'un
    // **longeron** de l'épine. Le premier jet de ce test comparait les maxima
    // des deux pièces, trouvait un jeu positif partout — et le pincement se
    // voyait quand même à l'écran.
    #[test]
    fn le_bardage_thermique_epouse_lepine() {
        let EtatStation::Prete(s) = preset_isv_fixe(Epine::Hexagonale, 0.0) else {
            panic!("l'ISV doit être prête");
        };
        // Sommets d'une famille, casés par tranche de 0,5 le long de l'axe. Le
        // maillage cuit n'a de sommets qu'aux bords de ses facettes : une
        // tranche plus fine tomberait dans le vide et ne mesurerait rien.
        let par_tranche = |f: &dyn Fn(&Composant) -> bool| -> Vec<(f32, f32, f32)> {
            let mut v: Vec<(f32, f32, f32)> = Vec::new();
            for p in s.pieces().iter().filter(|p| f(&p.composant)) {
                let mut b = crate::vaisseau::maillage::Batisseur::new();
                b.poser_transforme(p.transforme);
                p.composant.dessiner(&mut b);
                for lot in b.terminer() {
                    for vt in &lot.vertices {
                        let w = vec3(vt.position[0], vt.position[1], vt.position[2]);
                        let case = (w.x / 0.5).floor() * 0.5;
                        let r = w.yz().length();
                        match v.iter_mut().find(|(c, _, _)| (*c - case).abs() < 1e-3) {
                            Some(e) => {
                                e.1 = e.1.min(r);
                                e.2 = e.2.max(r);
                            }
                            None => v.push((case, r, r)),
                        }
                    }
                }
            }
            v
        };
        let epine = par_tranche(&|c| matches!(c, Composant::CharpenteHexa { .. }));

        // Le bardage est pris par sa **section de calcul** et non par ses
        // sommets. C'est la surface qui vient se plaquer sur la poutre ; ses
        // lèvres, relevées par construction, ne touchent rien et fausseraient la
        // mesure — une tranche qui ne contient qu'une lèvre lit un rayon bien
        // trop grand et fait croire à un bardage qui flotte.
        let long = THERMIQUE_FIN_Y - THERMIQUE_DEBUT_Y;
        // Rayon **inscrit** : le bardage ne peut pas s'approcher de l'axe plus
        // près que ça, quel que soit le calage angulaire des deux hexagones.
        let inscrit = 3.0_f32.sqrt() * 0.5;

        let (mut jeu_max, mut jeu_min) = (f32::MIN, f32::MAX);
        let (mut pire_x, mut pire) = (0.0f32, 0.0f32);
        let mut mesures = 0;
        for (x, _, _) in epine.iter() {
            if !(THERMIQUE_DEBUT_Y..=THERMIQUE_FIN_Y).contains(x) {
                continue;
            }
            // Rayon de l'épine pris sur une **fenêtre** d'une baie et non sur la
            // seule tranche : certaines tranches ne coupent que des diagonales et
            // lisent un rayon anormalement bas, ce qui ferait croire à un
            // bardage qui flotte là où il est simplement en face d'un vide.
            let epine_max = epine
                .iter()
                .filter(|(c, _, _)| (c - x).abs() <= 1.5)
                .fold(0.0f32, |m, (_, _, r)| m.max(*r));
            let t = (x - THERMIQUE_DEBUT_Y) / long;
            let bardage = crate::vaisseau::composant::section_bardage(
                THERMIQUE_RAYON_PIED,
                THERMIQUE_RAYON_BOUT,
                THERMIQUE_COURBURE,
                t,
            );
            let jeu = bardage * inscrit - epine_max;
            if jeu > jeu_max {
                (pire_x, pire) = (*x, jeu);
            }
            jeu_max = jeu_max.max(jeu);
            jeu_min = jeu_min.min(jeu);
            mesures += 1;
        }
        assert!(mesures >= 6, "{mesures} tranches mesurées : trop peu pour conclure");
        assert!(
            jeu_min > 0.05,
            "pincement de {:.2} : le bardage mord sur l'épine",
            -jeu_min
        );
        assert!(
            jeu_max < 0.40,
            "le bardage flotte de {pire:.2} au-dessus de l'épine vers X = {pire_x:.0}"
        );
    }

    // **Proportions d'ensemble.** Ce qui fait qu'un ISV lit comme un ISV et non
    // comme une fusée quelconque tient à trois rapports, et à rien d'autre : un
    // très long tronçon d'épine **nu**, une charge utile dominée par son fret, et
    // un vaisseau qui reste franchement plus long que large. Chacun des trois se
    // perd par accumulation de petites retouches, sans qu'aucune ne soit fautive
    // prise isolément — d'où ce test, qui les mesure sur le vaisseau assemblé.
    //
    // Les bornes sont larges à dessein : elles ne prescrivent pas une silhouette,
    // elles interdisent de la perdre.
    #[test]
    fn les_proportions_densemble_de_lisv_tiennent() {
        let EtatStation::Prete(s) = preset_isv_fixe(Epine::Hexagonale, 0.0) else {
            panic!("l'ISV doit être prête");
        };
        // Étendue axiale et rayon d'une famille de pièces, mesurés sur la
        // géométrie cuite : les `rayon_local()` ignorent l'échelle portée par la
        // transformée de l'ossature (cf. §C.1) et sous-estimeraient tout.
        let etendue = |f: &dyn Fn(&Composant) -> bool| -> (f32, f32, f32) {
            let (mut a, mut z, mut r) = (f32::MAX, f32::MIN, 0.0f32);
            for p in s.pieces().iter().filter(|p| f(&p.composant)) {
                let mut b = crate::vaisseau::maillage::Batisseur::new();
                b.poser_transforme(p.transforme);
                p.composant.dessiner(&mut b);
                for lot in b.terminer() {
                    for v in &lot.vertices {
                        let w = vec3(v.position[0], v.position[1], v.position[2]);
                        a = a.min(w.x);
                        z = z.max(w.x);
                        r = r.max(w.yz().length());
                    }
                }
            }
            (a, z, r)
        };

        let (x0, x1, rayon) = etendue(&|_| true);
        let long = x1 - x0;

        // La **propulsion** est un bloc à un bout : moteurs, nacelles, cuves,
        // ailes. On en prend le bout le plus éloigné des tuyères.
        let (_, prop_fin, _) = etendue(&|c| {
            matches!(
                c,
                Composant::MoteurAntimatiere { .. }
                    | Composant::ReacteurAntimatiere { .. }
                    | Composant::RadiateurMega { .. }
                    | Composant::BlocMoteur { .. }
                    | Composant::Reservoir { .. }
            )
        });
        // La **charge utile** est un bloc à l'autre bout, la tête exclue.
        let (util_debut, util_fin, _) = etendue(&|c| {
            matches!(
                c,
                Composant::RatelierCargo { .. }
                    | Composant::ModuleHabitat { .. }
                    | Composant::ModuleEquipage { .. }
            )
        });
        let (fret_a, fret_b, _) = etendue(&|c| matches!(c, Composant::RatelierCargo { .. }));
        let (hab_a, hab_b, _) = etendue(&|c| matches!(c, Composant::ModuleHabitat { .. }));

        // 1. L'**épine nue** entre les deux blocs : la signature du vaisseau.
        //    C'est une poutre en **tension**, et ce qui le dit est sa longueur à
        //    vide — pas le treillis, qui court d'un bout à l'autre.
        let nu = util_debut - prop_fin;
        assert!(
            nu > long * 0.25,
            "épine nue de {nu:.1} sur {long:.1} ({:.0} %) : le vaisseau n'a plus sa longue poutre en tension",
            nu / long * 100.0
        );
        let utile = util_fin - util_debut;
        assert!(
            nu > utile,
            "épine nue de {nu:.1} contre {utile:.1} de charge utile : la poutre ne domine plus"
        );

        // 2. Le **fret** domine la charge utile. Le vrai ISV porte 4 rangées de
        //    4 modules : c'est la plus grosse masse embarquée, et si l'habitat
        //    lui dispute la place, le vaisseau cesse d'être un cargo.
        let (f, h) = (fret_b - fret_a, hab_b - hab_a);
        assert!(
            f > h * 1.8,
            "fret {f:.1} contre habitat {h:.1} : le vaisseau ne lit plus comme un cargo"
        );

        // 3. **Élancement.** Le diamètre hors-tout est pris aux ailes radiateur ;
        //    en deçà de six fois, la silhouette s'épaissit et l'ISV commence à
        //    ressembler à une station.
        let elancement = long / (2.0 * rayon);
        assert!(
            elancement > 6.0,
            "élancement {elancement:.1} ({long:.1} de long pour {:.1} de diamètre)",
            2.0 * rayon
        );
    }

    // **À quel bout va la tête de bouclier** — la question ouverte depuis §C.8,
    // où nos propres notes se contredisaient. Tranchée par le schéma
    // d'assemblage : au bout **opposé aux moteurs**, après toute la charge utile,
    // sur le haut d'épine libre. Ce test fige la réponse, parce qu'elle ne se
    // déduit d'aucune autre et qu'un déplacement de rangée pourrait la brouiller.
    #[test]
    fn la_tete_de_bouclier_coiffe_le_bout_oppose_aux_moteurs() {
        let EtatStation::Prete(s) = preset_isv_fixe(Epine::Hexagonale, 0.0) else {
            panic!("l'ISV doit être prête");
        };
        let x = |f: &dyn Fn(&Composant) -> bool| -> Vec<f32> {
            s.pieces().iter().filter(|p| f(&p.composant)).map(|p| p.centre().x).collect()
        };
        let moteurs = x(&|c| matches!(c, Composant::MoteurAntimatiere { .. }));
        let petite = x(&|c| matches!(c, Composant::BouclierPetit { .. }));
        let grandes = x(&|c| matches!(c, Composant::BouclierGrand { .. }));
        let habitat = x(&|c| matches!(c, Composant::ModuleHabitat { .. }));
        assert_eq!(petite.len(), 1, "une seule petite plaque en tête");
        assert_eq!(grandes.len(), BOUCLIER_GRANDS, "trois grandes plaques");

        let moteur_max = moteurs.iter().fold(f32::MIN, |m, v| m.max(*v));
        let hab_max = habitat.iter().fold(f32::MIN, |m, v| m.max(*v));
        let tete_min = petite[0].min(grandes.iter().fold(f32::MAX, |m, v| m.min(*v)));
        assert!(
            tete_min > hab_max,
            "tête à {tete_min:.1} : elle n'est pas au-delà de l'habitat ({hab_max:.1})"
        );
        assert!(
            tete_min > moteur_max + 80.0,
            "tête à {tete_min:.1} : pas franchement à l'opposé des moteurs ({moteur_max:.1})"
        );
        // La **petite** plaque vient en premier, les trois grandes derrière : le
        // schéma est explicite là-dessus, et l'ordre a un sens — c'est la plaque
        // sacrificielle qui prend le grain, les grandes encaissent le nuage.
        assert!(
            grandes.iter().all(|g| *g > petite[0]),
            "la petite plaque ({:.1}) n'est pas en tête des grandes ({grandes:?})",
            petite[0]
        );
        // Et rien de la tête ne redescend sur l'épine, qui est déjà occupée.
        let sommet = EPINE_SOMMET_Y * ISV_ECHELLE;
        assert!(
            tete_min > sommet,
            "tête à {tete_min:.1}, sommet d'épine à {sommet:.1} : les plaques mordent sur l'épine"
        );
    }

    // Les quatre plaques sont **enfilées sur un mât commun**. C'est donc la
    // **plus petite** qui borne la section du mât, pas les grandes : son alésage
    // est proportionnel à son rayon, et il est le plus étroit de la pile.
    //
    // Le piège est qu'un mât trop gros ne se voit pas — il traverse le moyeu et
    // ressort de l'autre côté sans que rien ne l'arrête, et la pile a l'air
    // enfilée alors qu'elle est empalée. On mesure donc les deux sur la
    // géométrie cuite, plutôt que de refaire le calcul des fractions ici.
    #[test]
    fn le_mat_de_tete_passe_le_plus_petit_alesage() {
        let transversal = |c: &Composant, garder: &dyn Fn(f32) -> bool| -> (f32, f32) {
            let mut b = crate::vaisseau::maillage::Batisseur::new();
            c.dessiner(&mut b);
            let (mut mini, mut maxi) = (f32::MAX, 0.0f32);
            for lot in b.terminer() {
                for v in &lot.vertices {
                    if !garder(v.position[2]) {
                        continue;
                    }
                    let r = vec2(v.position[0], v.position[1]).length();
                    mini = mini.min(r);
                    maxi = maxi.max(r);
                }
            }
            (mini, maxi)
        };

        // Alésage : les sommets du moyeu sont les seuls à vivre loin du plan
        // médian de la plaque.
        let seuil = BOUCLIER_PETIT_RAYON * 0.05;
        let (alesage, _) = transversal(
            &Composant::BouclierPetit { profil: Profil::P1, rayon: BOUCLIER_PETIT_RAYON },
            &|z| z.abs() > seuil,
        );
        let (_, mat) = transversal(
            &Composant::Treillis {
                profil: BOUCLIER_MAT,
                longueur: 20.0,
                style: StyleTreillis::Triangulaire,
            },
            &|_| true,
        );
        assert!(
            mat < alesage - 0.05,
            "mât de {mat:.3} pour un alésage de {alesage:.3} : il n'enfile pas la petite plaque, il la traverse"
        );
        // Et pas ridiculement fin non plus : un mât qui flotte dans son alésage
        // ne porte visiblement rien.
        assert!(
            mat > alesage * 0.5,
            "mât de {mat:.3} pour un alésage de {alesage:.3} : il flotte dans le moyeu"
        );
    }

    // L'ISV est un **tracteur** : les moteurs sont à un bout, la charge utile à
    // l'**autre**, au bout d'une longue épine en tension. C'est la décision de
    // conception la plus facile à casser par inadvertance en déplaçant une
    // rangée — on la verrouille ici. (Le modèle est couché : l'axe du vaisseau
    // est X après le pivot final.)
    #[test]
    fn isv_porte_son_fret_a_loppose_des_moteurs() {
        let EtatStation::Prete(s) = preset_isv() else {
            panic!("l'ISV doit être prête");
        };
        let moyenne_x = |f: &dyn Fn(&Composant) -> bool| -> Option<f32> {
            let v: Vec<f32> = s
                .pieces()
                .iter()
                .filter(|p| f(&p.composant))
                .map(|p| p.centre().x)
                .collect();
            (!v.is_empty()).then(|| v.iter().sum::<f32>() / v.len() as f32)
        };
        let fret = moyenne_x(&|c| matches!(c, Composant::RatelierCargo { .. })).expect("des rangées de fret");
        let moteurs = moyenne_x(&|c| matches!(c, Composant::MoteurAntimatiere { .. })).expect("des moteurs");
        let habitat = moyenne_x(&|c| matches!(c, Composant::ModuleHabitat { .. })).expect("de l'habitat");
        assert!(
            fret > moteurs + 30.0,
            "fret ({fret:.1}) pas nettement à l'opposé des moteurs ({moteurs:.1})"
        );
        // **Ordre des sections** le long de l'épine : moteurs → fret → habitat.
        // L'habitat est ce qu'on éloigne le plus des tuyères ; s'il repassait
        // devant le fret, c'est tout le sens de la disposition qui tomberait.
        assert!(
            habitat > fret,
            "habitat ({habitat:.1}) pas au-delà du fret ({fret:.1})"
        );

        // Les rangées sont **enfilées sur l'épine**, pas déportées sur un côté :
        // leur centre reste sur l'axe du vaisseau.
        for p in s.pieces() {
            if matches!(p.composant, Composant::RatelierCargo { .. }) {
                let c = p.centre();
                assert!(c.y.abs() < 1e-3 && c.z.abs() < 1e-3, "rangée hors axe : {c:?}");
            }
        }
    }

    #[test]
    fn toutes_graines_donnent_une_station_finie() {
        for g in 0..30u64 {
            for style in Style::TOUS {
                let etat = generer(&ParamsStation { graine: g, complexite: 3, style, ossature: None });
                assert!(matches!(etat, EtatStation::Prete(_)), "graine {g} {style:?}");
            }
        }
    }

    /// La silhouette cible (`docs/suivi/stations.md` Partie A) : **une** barre
    /// de puissance en deux tronçons opposés, panneaux dessus, boom nu, grappe
    /// habitée entièrement sous la barre. Vérifiée sur un lot de graines — c'est
    /// le filet contre les échecs silencieux de `poser` (§2 du doc), qui
    /// laissaient une station plausible mais fausse.
    #[test]
    fn silhouette_generee_converge() {
        for oss in [Ossature::Iss, Ossature::Mir] {
            for c in [2u8, 3, 4] {
                for g in 0..12u64 {
                    let p = ParamsStation { graine: g, complexite: c, style: Style::Historique, ossature: Some(oss) };
                    let etat = generer(&p);
                    let st = etat.doit_dessiner().expect("station finie");
                    let ctx = format!("{oss:?} c={c} graine {g}");

                    // Tronçons de poutre P2 : une barre (2) jusqu'à c=3, une
                    // structure en **H** (2 barres = 4 tronçons) à c=4.
                    let poutres: Vec<Vec3> = st
                        .pieces()
                        .iter()
                        .filter(|p| matches!(p.composant, Composant::Treillis { profil: Profil::P2, .. }))
                        .map(|p| p.centre())
                        .collect();
                    if c < 4 {
                        assert_eq!(poutres.len(), 2, "{ctx}: deux demi-poutres");
                        let long = 8.0 + c as f32 * 3.5;
                        assert!(
                            poutres[0].distance(poutres[1]) > long,
                            "{ctx}: demi-poutres opposées, pas bout à bout"
                        );
                    } else {
                        assert_eq!(poutres.len(), 4, "{ctx}: H = quatre demi-poutres");
                        // Deux tronçons de chaque côté de chacun des deux axes :
                        // c'est ce qui fait le H (deux barres parallèles).
                        assert_eq!(poutres.iter().filter(|p| p.x > 0.0).count(), 2, "{ctx}: H, côté +X");
                        assert_eq!(poutres.iter().filter(|p| p.z > 0.0).count(), 2, "{ctx}: H, côté +Z");
                    }
                    let y_poutre = poutres.iter().map(|p| p.y).sum::<f32>() / poutres.len() as f32;

                    // La grappe pressurisée reste **sous** la barre (corridor).
                    for p in st.pieces() {
                        if let Composant::ModuleAxial { profil: Profil::P1, .. } = p.composant {
                            assert!(
                                p.centre().y < y_poutre - 2.0,
                                "{ctx}: module à y={:.1} au ras de la poutre (y={y_poutre:.1})",
                                p.centre().y
                            );
                        }
                    }

                    // Tous les grands panneaux vivent sur la poutre, aux bras.
                    let panneaux: Vec<Vec3> = st
                        .pieces()
                        .iter()
                        .filter(|p| matches!(p.composant, Composant::PanneauSolaire { longueur, .. } if longueur > 4.5))
                        .map(|p| p.centre())
                        .collect();
                    assert!(panneaux.len() >= 4, "{ctx}: au moins deux paires d'arrays ({} trouvés)", panneaux.len());
                    for p in &panneaux {
                        assert!(
                            (p.y - y_poutre).abs() < 4.0,
                            "{ctx}: array à y={:.1} loin de la poutre (y={y_poutre:.1})",
                            p.y
                        );
                    }

                    // **Proportions façon ISS** : l'habitat suit la puissance
                    // (1 à 2,5 modules par aile), le refroidissement et la
                    // propulsion suivent l'habitat.
                    let modules = st
                        .pieces()
                        .iter()
                        .filter(|p| matches!(p.composant, Composant::ModuleAxial { profil: Profil::P1, .. }))
                        .count();
                    let rads = st
                        .pieces()
                        .iter()
                        .filter(|p| matches!(p.composant, Composant::Radiateur { .. }))
                        .count();
                    let props = st
                        .pieces()
                        .iter()
                        .filter(|p| matches!(p.composant, Composant::Propulseur { .. }))
                        .count();
                    let ailes = panneaux.len() as f32;
                    assert!(
                        (modules as f32) >= ailes * 0.9,
                        "{ctx}: habitat sous-dimensionné ({modules} modules pour {ailes} ailes)"
                    );
                    assert!(
                        (modules as f32) <= ailes * 2.5 + 2.0,
                        "{ctx}: habitat surdimensionné ({modules} modules pour {ailes} ailes)"
                    );
                    // Bande de proportion, **étalonnée par mesure** et non
                    // choisie : le vrai ISS tient 14 radiateurs pour 16 modules
                    // (0,88), et le plancher est là pour attraper une station
                    // qui ne refroidit visiblement rien, pas pour imposer le
                    // ratio réel.
                    //
                    // ⚠️ Plancher descendu de 0,40 à 0,35 le 2026-08-01. Le
                    // regroupement des faces miroir par `|x|` (cf. `cle_surface`)
                    // divise par deux le nombre de tirages de `fabrique_appendice`,
                    // ce qui **décale le flux du RNG** : les stations changent
                    // toutes, sans que la grammaire ait empiré. Mesuré sur les 72
                    // combinaisons de ce test : **un seul cas** passe sous 0,40
                    // (Mir c=2 graine 4, à 0,357), les 71 autres sont au-dessus.
                    // Un écart de 0,04 sur un plancher déjà à moins de la moitié
                    // du ratio réel ne se voit pas — abaisser le plancher est plus
                    // honnête que de retoucher la grammaire pour un tirage.
                    assert!(
                        (rads as f32) >= modules as f32 * 0.35 && (rads as f32) <= modules as f32 * 1.2 + 2.0,
                        "{ctx}: refroidissement disproportionné ({rads} radiateurs pour {modules} modules)"
                    );
                    assert!(props >= 1, "{ctx}: aucune propulsion");

                    // **Symétrie miroir de la barre** : tout appendice posé sur
                    // la structure de puissance a son jumeau de l'autre côté de
                    // la jonction (même type, position réfléchie sur x).
                    let sur_barre: Vec<(u8, Vec3)> = st
                        .pieces()
                        .iter()
                        .filter_map(|p| {
                            let cat = match p.composant {
                                Composant::PanneauSolaire { .. } => 0u8,
                                Composant::Radiateur { .. } => 1,
                                Composant::Antenne { .. } => 2,
                                Composant::Caisson { .. } => 3,
                                _ => return None,
                            };
                            let cp = p.centre();
                            ((cp.y - y_poutre).abs() < 4.0 && cp.x.abs() > 2.0)
                                .then_some((cat, cp))
                        })
                        .collect();
                    for (cat, cp) in &sur_barre {
                        let miroir = vec3(-cp.x, cp.y, cp.z);
                        assert!(
                            sur_barre.iter().any(|(c2, p2)| c2 == cat && p2.distance(miroir) < 0.8),
                            "{ctx}: appendice cat {cat} en ({:.1},{:.1},{:.1}) sans jumeau miroir",
                            cp.x, cp.y, cp.z
                        );
                    }

                    // Le boom et la jonction restent nus : aucun appendice dans
                    // le corridor central.
                    for p in st.pieces() {
                        let est_app = matches!(
                            p.composant,
                            Composant::PanneauSolaire { .. }
                                | Composant::Radiateur { .. }
                                | Composant::Antenne { .. }
                                | Composant::Caisson { .. }
                        );
                        let cpos = p.centre();
                        assert!(
                            !(est_app && cpos.x.abs() < 1.2 && cpos.z.abs() < 1.2 && cpos.y > 1.5),
                            "{ctx}: appendice sur le boom en ({:.1},{:.1},{:.1})",
                            cpos.x, cpos.y, cpos.z
                        );
                    }
                }
            }
        }
    }
    // --- Elysium ---------------------------------------------------------

    // Ce qui fait tenir l'ossature : la base d'un bras doit tomber **exactement**
    // sur une face du moyeu. C'est la raison du choix de la `Charpente` carrée
    // contre `CharpenteHexa`, et ça se mesure au lieu de se supposer.
    #[test]
    fn la_base_dun_bras_delysium_tombe_pile_sur_une_face_du_moyeu() {
        let sg = ELYSIUM_PROFIL.rayon() * 0.5;
        let face = 2.0 * sg; // côté de l'hexagone du moyeu
        let base_carree = 2.0 * sg; // largeur de base d'une Charpente
        assert!((base_carree - face).abs() < 1e-5, "carrée : {base_carree} vs face {face}");

        // La variante hexagonale, elle, déborderait — d'où son rejet.
        let rg = ELYSIUM_PROFIL.rayon() * 0.5 * std::f32::consts::SQRT_2;
        let entre_plats = 2.0 * rg * (PI / 6.0).cos();
        assert!(entre_plats > face * 1.2, "hexa devrait déborder : {entre_plats} vs {face}");
    }

    // Trois bras, à 120°, dans le plan du moyeu — et pas quatre ni coplanaires
    // par accident. Sondé sur la géométrie cuite, pas sur les constantes, et
    // sur les **deux** variantes : le moyeu ne doit pas dépendre du bras.
    /// **Les deux miroirs sont inclinés, et regardent le même soleil.**
    ///
    /// À plat au-dessus de l'anneau, un miroir ne renvoie rien : la lumière
    /// arrive perpendiculairement à l'axe de rotation, et un disque parallèle à
    /// l'anneau la réfléchirait d'où elle vient. C'est le défaut que l'œil a vu.
    ///
    /// ⚠️ Et le second miroir a son propre piège : incliné du **même signe** que
    /// le premier, il regarderait dans l'autre direction — les deux normales
    /// partant de côtés opposés de l'anneau. Le test le vérifie par le produit
    /// scalaire avec [`SOLEIL`], qui doit être positif **pour les deux**.
    #[test]
    fn les_deux_miroirs_sont_inclines_vers_le_meme_soleil() {
        let EtatStation::Prete(st) = preset_stanford_moyeu() else { panic!("moyeu vide") };
        let miroirs: Vec<&crate::vaisseau::Piece> = st
            .pieces()
            .iter()
            .filter(|p| matches!(p.composant, Composant::Miroir { .. }))
            .collect();
        assert_eq!(miroirs.len(), 2, "il en faut un de chaque côté de l'anneau");

        let mut cotes = Vec::new();
        for m in &miroirs {
            let n = m.transforme.transform_vector3(Vec3::Z).normalize();
            let y = m.centre().y;
            cotes.push(y);

            // Ni parallèle à l'anneau (le défaut d'origine), ni couché dans son
            // plan (il ne le verrait plus).
            let cos = n.dot(Vec3::Y).abs();
            assert!(cos < 0.95, "miroir quasi parallèle à l'anneau (cos = {cos})");
            assert!(cos > 0.2, "miroir presque dans le plan de l'anneau (cos = {cos})");
            // À l'angle déclaré. Comparer à la constante plutôt qu'à un nombre
            // réécrit ici : l'inclinaison est un réglage à l'œil.
            let attendu = MIROIR_INCLINAISON.cos();
            assert!((cos - attendu).abs() < 0.05, "inclinaison {cos} au lieu de {attendu}");

            // **Face au soleil.**
            assert!(
                n.dot(SOLEIL) > 0.1,
                "miroir en y={y:.1} tourné à l'opposé du soleil (n·soleil = {})",
                n.dot(SOLEIL)
            );
            // **Et face à l'anneau** : sa normale redescend vers le plan y = 0.
            assert!(
                n.y * y < 0.0,
                "miroir en y={y:.1} tourné dos à l'anneau (n.y = {})",
                n.y
            );
        }
        // Un de chaque côté, à la même distance.
        cotes.sort_by(f32::total_cmp);
        assert!(cotes[0] < 0.0 && cotes[1] > 0.0, "les deux miroirs sont du même côté");
        assert!((cotes[0] + cotes[1]).abs() < 1e-3, "paire asymétrique : {cotes:?}");
    }

    /// Les cœurs d'habitat (`CoeurGris`) d'un état, **triés du centre vers les
    /// bouts**.
    ///
    /// Le tore en porte six, et il faut savoir lesquels : deux accouplés au
    /// centre, deux coiffes, deux flèches. Même composant, même variante — seule
    /// la distance à l'origine les distingue, et s'appuyer sur l'ordre de pose
    /// serait s'appuyer sur un détail d'écriture.
    fn coeurs_tries(e: EtatStation) -> Vec<crate::vaisseau::Piece> {
        let EtatStation::Prete(st) = e else { panic!("station vide") };
        let mut v: Vec<crate::vaisseau::Piece> = st
            .pieces()
            .iter()
            .filter(|p| {
                matches!(
                    p.composant,
                    Composant::ModuleAxial { variante: VarianteModule::CoeurGris, .. }
                )
            })
            .cloned()
            .collect();
        v.sort_by(|a, b| a.centre().length().total_cmp(&b.centre().length()));
        v
    }

    /// **Aucune pièce du pilier n'en enfouit une autre.**
    ///
    /// ⚠️ C'est le défaut que ce tore a réellement porté : trois segments de fût
    /// par côté, **entièrement contenus** dans l'habitat axial qui couvrait la
    /// même tranche avec un rayon deux fois supérieur. Six pièces dessinées pour
    /// rien, invisibles, et qu'aucun test ne voyait — il a fallu sonder les
    /// positions pour s'en apercevoir.
    #[test]
    fn le_pilier_n_enfouit_aucune_de_ses_pieces() {
        let EtatStation::Prete(st) = preset_stanford() else { panic!("station vide") };
        let mut plages: Vec<(f32, f32)> = st
            .pieces()
            .iter()
            .filter_map(|p| {
                let Composant::ModuleAxial { longueur, .. } = p.composant else { return None };
                let k = p.transforme.x_axis.truncate().length();
                let axe = p.transforme.transform_vector3(Vec3::Z).normalize();
                // ⚠️ **Seuls les modules alignés sur l'axe.** Les tunnels des
                // bras sont des `ModuleAxial` eux aussi, mais couchés à
                // l'horizontale : c'est l'orientation qui définit la colonne,
                // pas le type de composant.
                if axe.y.abs() < 0.99 {
                    return None;
                }
                let demi = longueur * 0.5 * k;
                let y = p.centre().y;
                Some((y - demi, y + demi))
            })
            .collect();
        assert!(plages.len() >= 6, "moins de six modules axiaux : {}", plages.len());
        plages.sort_by(|a, b| a.0.total_cmp(&b.0));
        for f in plages.windows(2) {
            let (fin_precedent, debut_suivant) = (f[0].1, f[1].0);
            assert!(
                debut_suivant >= fin_precedent - 1e-3,
                "deux modules se recouvrent : le précédent finit à {fin_precedent:.3},                  le suivant commence à {debut_suivant:.3}"
            );
        }
    }

    /// **Le pilier polaire sort des deux côtés, et porte un module à chaque
    /// bout.** C'est la zone de gravité nulle du tore : amarrage, observation.
    #[test]
    fn le_pilier_polaire_est_symetrique_et_coiffe() {
        // La colonne est à cheval sur les deux moitiés depuis que l'ossature
        // tourne : c'est la station entière qu'il faut regarder.
        let EtatStation::Prete(st) = preset_stanford() else { panic!("station vide") };
        let hauts: Vec<f32> = st.pieces().iter().map(|p| p.centre().y).collect();
        let plus_haut = hauts.iter().cloned().fold(f32::MIN, f32::max);
        let plus_bas = hauts.iter().cloned().fold(f32::MAX, f32::min);
        // Six cœurs, du centre vers les bouts : la paire accouplée au moyeu,
        // les deux coiffes, puis les deux flèches. La symétrie se mesure sur les
        // **flèches**, qui sont les extrémités de la colonne.
        let coeurs = coeurs_tries(preset_stanford());
        assert_eq!(coeurs.len(), 6, "deux cœurs au centre, deux coiffes, deux flèches");
        let poles: Vec<f32> = coeurs[4..].iter().map(|p| p.centre().y).collect();
        let haut = poles.iter().cloned().fold(f32::MIN, f32::max);
        let bas = poles.iter().cloned().fold(f32::MAX, f32::min);
        assert!(haut > 0.0 && bas < 0.0, "le pilier ne sort pas des deux côtés : {bas} .. {haut}");
        assert!(
            (haut + bas).abs() < 1e-3,
            "pilier asymétrique : {bas} en bas, {haut} en haut"
        );
        // ⚠️ Il y avait ici deux assertions du genre « quelque chose dépasse le
        // pôle » — vraies tant que le miroir était l'élément le plus haut. La
        // flèche, doublée, culmine désormais au-dessus de lui : elles ne
        // décrivaient pas une propriété mais l'ordre de grandeur du moment. Ce
        // qui reste vrai, et qui compte, c'est que les extrémités de la colonne
        // sont bien les points extrêmes de la station **sur son axe**.
        assert!(
            (plus_haut - haut).abs() < 1e-3 || plus_haut > haut,
            "un élément dépasse la flèche par le haut sans être elle"
        );
        assert!((plus_bas - bas).abs() < 1e-3 || plus_bas < bas);
    }

    /// Le mât du pôle bas : sa plage en Y, mesurée sur la pièce et non
    /// recalculée depuis les constantes — sinon les tests qui s'y adossent ne
    /// diraient plus rien du montage réel.
    fn mat_bas(st: &crate::vaisseau::Station) -> (f32, f32) {
        let fleche = st
            .pieces()
            .iter()
            .filter(|p| matches!(p.composant, Composant::ModuleAxial { .. }))
            .filter(|p| p.transforme.transform_vector3(Vec3::Z).normalize().y.abs() > 0.99)
            .min_by(|a, b| a.centre().y.total_cmp(&b.centre().y))
            .expect("pas de mât polaire");
        let Composant::ModuleAxial { longueur, .. } = fleche.composant else { unreachable!() };
        let demi = longueur * 0.5 * fleche.transforme.x_axis.truncate().length();
        (fleche.centre().y - demi, fleche.centre().y + demi)
    }

    /// **La charge du pôle bas est enfilée sur le mât** : les cuves finissent
    /// exactement à son bout, l'habitat est au-dessus, et rien ne pend dans le
    /// vide au-delà de la structure qui le porte.
    ///
    /// C'est le point d'ancrage du calcul : toute la pile se mesure **depuis le
    /// bout**. Ancrée par le haut, la dernière rangée de cuves passerait dans le
    /// vide dès qu'une cote bouge, et personne ne le verrait.
    #[test]
    fn la_charge_polaire_est_enfilee_sur_le_mat() {
        let EtatStation::Prete(st) = preset_stanford() else { panic!("station vide") };
        let (bas, haut) = mat_bas(&st);

        let cuves = st
            .pieces()
            .iter()
            .find(|p| matches!(p.composant, Composant::GrappeReservoirs { .. }))
            .expect("pas de cuves au pôle bas");
        let Composant::GrappeReservoirs { rayon_cuve, rangees, .. } = cuves.composant else {
            unreachable!()
        };
        let axe = cuves.transforme.transform_vector3(Vec3::Z).normalize();
        let bout = cuves.centre().y + axe.y * crate::vaisseau::grappe_hauteur(rayon_cuve, rangees);
        let (cuves_bas, cuves_haut) = (cuves.centre().y.min(bout), cuves.centre().y.max(bout));
        assert!(
            (cuves_bas - bas).abs() < 1e-3,
            "les cuves ne finissent pas au bout du mât : {cuves_bas:.3} contre {bas:.3}"
        );

        let hab: Vec<&crate::vaisseau::Piece> = st
            .pieces()
            .iter()
            .filter(|p| matches!(p.composant, Composant::ModuleHabitat { .. }))
            .collect();
        assert_eq!(hab.len(), HAB_MODULES, "la couronne polaire n'a pas trois modules");
        let y = hab[0].centre().y;
        for m in &hab {
            assert!((m.centre().y - y).abs() < 1e-3, "les trois modules ne sont pas au même niveau");
        }
        let (hab_bas, hab_haut) = (y - HAB_LONG * 0.5, y + HAB_LONG * 0.5);
        assert!(
            hab_bas >= bas - 1e-3 && hab_haut <= haut + 1e-3,
            "l'habitat polaire déborde du mât : {hab_bas:.3}..{hab_haut:.3} pour un mât {bas:.3}..{haut:.3}"
        );

        // **L'écart demandé existe vraiment.** Accolées, les deux couronnes se
        // liraient comme un seul bloc de six volumes.
        let ecart = hab_bas - cuves_haut;
        assert!(
            ecart > 1e-3,
            "habitat et cuves se touchent (ou se croisent) : écart {ecart:.3}"
        );
        assert!(
            (ecart - POLE_ECART).abs() < 1e-3,
            "l'écart posé ({ecart:.3}) n'est pas celui demandé ({POLE_ECART})"
        );

        // Et la pile **s'affine en descendant** : les cuves sont plus étroites
        // que la couronne d'habitat au-dessus. L'inverse ferait une massue.
        let large_hab = hab_couronne(FLECHE_PROFIL.rayon()) + Profil::P2.rayon();
        let large_cuves = crate::vaisseau::grappe_rayon_hors_tout(rayon_cuve);
        assert!(
            large_cuves < large_hab,
            "les cuves ({large_cuves:.3}) débordent l'habitat ({large_hab:.3})"
        );
    }

    /// **Les trois modules polaires ne se croisent pas.** Le mât est en P0, cinq
    /// fois plus fin que l'épine de l'ISV : au jeu nominal la couronne serait
    /// trop serrée et les trois fûts s'interpénétreraient. C'est
    /// [`hab_rayon_mini`] qui reprend la main — et sans lui rien ne le dirait,
    /// puisque le montage resterait « valide ».
    #[test]
    fn les_modules_polaires_ne_se_croisent_pas() {
        let g = FLECHE_PROFIL.rayon();
        let entre_axes = 2.0 * hab_couronne(g) * (PI / 3.0).sin();
        let f = crate::vaisseau::pieces::ONIGIRI_FILET;
        let portee = Profil::P2.rayon() * ((1.0 - f) * (PI / 6.0).cos() + f);
        assert!(
            entre_axes > 2.0 * portee,
            "modules polaires qui se croisent ({entre_axes:.3} <= {:.3})",
            2.0 * portee
        );
        // Et la ferrure atteint quand même le mât : c'est elle qui s'allonge.
        let rail = hab_couronne(g) - HAB_INSCRIT - hab_attache(g) * 0.5;
        assert!((rail - g).abs() < 1e-3, "ferrure polaire à {rail:.3}, mât à {g:.3}");
    }

    /// **Le montage de l'ISV est inchangé.** Le calcul de couronne est devenu
    /// général pour porter le mât polaire ; il doit redonner *exactement* les
    /// cotes historiques là où c'est le jeu qui commande, sans quoi la
    /// généralisation aurait déplacé l'habitat du vaisseau au passage.
    #[test]
    fn la_couronne_de_lisv_est_inchangee() {
        for epine in [Epine::Carree, Epine::Hexagonale] {
            let g = epine.hors_tout();
            assert!(
                (hab_couronne(g) - hab_rayon(g)).abs() < 1e-5,
                "{} : couronne {:.4} au lieu de {:.4}",
                epine.nom(),
                hab_couronne(g),
                hab_rayon(g)
            );
            assert!(
                (hab_attache(g) - HAB_ATTACHE).abs() < 1e-5,
                "{} : ferrure {:.4} au lieu de {HAB_ATTACHE}",
                epine.nom(),
                hab_attache(g)
            );
        }
    }

    /// **Aucune pièce de la charge polaire ne traverse un miroir.** Les deux
    /// disques font plus de quarante unités de rayon et sont inclinés : ce que
    /// l'on ajoute sur la colonne passe forcément dans leur voisinage. Une
    /// pièce à cheval sur le plan d'un miroir ressortirait de l'autre côté.
    ///
    /// ⚠️ La marge la plus serrée vaut ~1,8 : c'est peu, et c'est mesuré. Toute
    /// retouche à la hauteur des miroirs, à la longueur du mât ou à la taille
    /// des cuves peut la consommer — d'où cette garde.
    #[test]
    fn la_charge_polaire_ne_traverse_aucun_miroir() {
        let EtatStation::Prete(st) = preset_stanford() else { panic!("station vide") };
        let miroirs: Vec<(Vec3, Vec3)> = st
            .pieces()
            .iter()
            .filter(|p| matches!(p.composant, Composant::Miroir { .. }))
            .map(|p| (p.centre(), p.transforme.transform_vector3(Vec3::Z).normalize()))
            .collect();
        assert_eq!(miroirs.len(), 2, "il n'y a plus deux miroirs");
        for p in st.pieces() {
            if !matches!(
                p.composant,
                Composant::ModuleHabitat { .. } | Composant::GrappeReservoirs { .. }
            ) {
                continue;
            }
            for (c, n) in &miroirs {
                let d = (p.centre() - *c).dot(*n);
                assert!(
                    d.abs() > p.composant.rayon_local(),
                    "{} est à cheval sur un miroir : {:.3} pour un rayon de {:.3}",
                    p.composant.nom(),
                    d.abs(),
                    p.composant.rayon_local()
                );
            }
        }
    }

    /// **L'anneau tourne, le moyeu non** — et les deux moitiés sont disjointes :
    /// aucune pièce ne doit se retrouver dans les deux, sinon elle serait
    /// dessinée deux fois et tournerait à moitié.
    #[test]
    fn les_deux_moities_du_tore_sont_disjointes_et_completes() {
        let EtatStation::Prete(moyeu) = preset_stanford_moyeu() else { panic!("moyeu vide") };
        let EtatStation::Prete(anneau) = preset_stanford_anneau() else { panic!("anneau vide") };
        let EtatStation::Prete(tout) = preset_stanford() else { panic!("tout vide") };
        assert_eq!(
            moyeu.nb_pieces() + anneau.nb_pieces(),
            tout.nb_pieces(),
            "la vue d'un seul tenant ne recompose pas exactement les deux moitiés"
        );
        // Le tore lui-même est du côté tournant, le miroir du côté fixe.
        assert!(
            anneau.pieces().iter().any(|p| matches!(p.composant, Composant::Tore { .. })),
            "le tore n'est pas dans la moitié tournante"
        );
        assert!(
            moyeu.pieces().iter().any(|p| matches!(p.composant, Composant::Miroir { .. })),
            "le miroir tournerait avec l'anneau"
        );
        // Et la colonne d'habitat axiale est bien du côté immobile : c'est elle
        // que « despun » désigne.
        assert!(
            moyeu.pieces().iter().any(|p| matches!(
                p.composant,
                Composant::ModuleAxial { variante: VarianteModule::CoeurGris, .. }
            )),
            "les cœurs axiaux tourneraient avec l'anneau"
        );
        // L'ossature centrale, elle, tourne : elle porte l'anneau.
        assert!(
            anneau.pieces().iter().any(|p| matches!(p.composant, Composant::TreillisHexagone { .. })),
            "l'ossature centrale ne tourne pas avec l'anneau qu'elle porte"
        );
        // **Toute** la colonne axiale est immobile : aucun cœur ne doit traîner
        // du côté tournant. C'est la garde qui a manqué quand les coiffes y
        // étaient encore.
        assert!(
            !anneau.pieces().iter().any(|p| matches!(
                p.composant,
                Composant::ModuleAxial { variante: VarianteModule::CoeurGris, .. }
            )),
            "un cœur d'habitat tourne avec l'anneau"
        );
        assert!(
            !anneau.pieces().iter().any(|p| matches!(p.composant, Composant::Miroir { .. })),
            "un miroir traîne dans la moitié tournante"
        );
    }

    // Le moyeu d'Elysium porte une **jante médiane** de plus que celui de
    // Stanford — même hauteur totale, un anneau de plus à Y = 0, là où les six
    // bras traversent le moyeu.
    #[test]
    fn le_moyeu_delysium_a_une_jante_de_plus_que_celui_de_stanford() {
        let compte_hex = |e: EtatStation| -> usize {
            let EtatStation::Prete(st) = e else { panic!("station vide") };
            st.pieces().iter().filter(|p| matches!(p.composant, Composant::TreillisHexagone { .. })).count()
        };
        let (s, e) = (compte_hex(preset_stanford_anneau()), compte_hex(preset_elysium()));
        assert_eq!(s, 2, "Stanford : deux plateaux, pas plus");
        assert_eq!(e, 3, "Elysium : trois plateaux, la jante médiane comprise");
    }

    // Les six tronçons de la jante d'Elysium referment le tour : chacun
    // démarre à son propre bras (`30° + k·60°`), dans le plan XZ **interne**
    // à `jante_elysium` — testée ici en l'isolant de `preset_elysium`, pour
    // ne pas mélanger cette géométrie avec le pivot global vers Z (voir
    // `elysium_fait_face_a_z` plus bas pour celui-là).
    #[test]
    fn la_jante_delysium_ferme_le_tour() {
        let mut asm = Assembleur::new();
        jante_elysium(&mut asm);
        let EtatStation::Prete(st) = asm.terminer() else { panic!("jante vide") };
        let jantes: Vec<&crate::vaisseau::Piece> =
            st.pieces().iter().filter(|p| matches!(p.composant, Composant::Jante { .. })).collect();
        assert_eq!(jantes.len(), ELYSIUM_N_BRAS, "un tronçon par bras");

        let (rayon_majeur, arc, ouverture) = match &jantes[0].composant {
            Composant::Jante { rayon_majeur, arc, ouverture, .. } => (*rayon_majeur, *arc, *ouverture),
            _ => unreachable!(),
        };
        assert!((arc - TAU / ELYSIUM_N_BRAS as f32).abs() < 1e-5, "un sixième de tour");
        assert!(ouverture > 0.0 && ouverture < PI, "ouverture hors bornes");

        // Le point local (rayon_majeur, 0, 0) est le `u = 0` de chaque canal —
        // transformé, chacun doit tomber à `30° + k·60°`, dans le plan XZ.
        let local = Vec3::new(rayon_majeur, 0.0, 0.0);
        let mut angles: Vec<f32> = jantes
            .iter()
            .map(|p| {
                let monde = p.transforme.transform_point3(local);
                assert!(monde.y.abs() < 1e-3, "une jante sort du plan de l'anneau, y = {}", monde.y);
                monde.z.atan2(monde.x).rem_euclid(TAU).to_degrees()
            })
            .collect();
        angles.sort_by(f32::total_cmp);
        for (k, a) in angles.iter().enumerate() {
            let attendu = 30.0 + 60.0 * k as f32;
            assert!((a - attendu).abs() < 0.5, "tronçon à {a:.1}°, attendu {attendu}°");
        }
    }

    // `preset_elysium` pivote tout d'un quart de tour autour de X : l'anneau,
    // bâti en interne dans le plan XZ (voir `la_jante_delysium_ferme_le_tour`
    // ci-dessus), doit se retrouver dans le plan **XY** une fois publié — Z
    // devient l'axe étroit (nul en `y` avant le pivot), Y prend la place que Z
    // tenait (le grand rayon de l'anneau).
    #[test]
    fn elysium_fait_face_a_z() {
        let EtatStation::Prete(st) = preset_elysium() else { panic!("elysium vide") };
        let jantes: Vec<&crate::vaisseau::Piece> =
            st.pieces().iter().filter(|p| matches!(p.composant, Composant::Jante { .. })).collect();
        assert_eq!(jantes.len(), ELYSIUM_N_BRAS, "un tronçon par bras");

        let rayon_majeur = match &jantes[0].composant {
            Composant::Jante { rayon_majeur, .. } => *rayon_majeur,
            _ => unreachable!(),
        };
        let local = Vec3::new(rayon_majeur, 0.0, 0.0);
        let mut angles: Vec<f32> = jantes
            .iter()
            .map(|p| {
                let monde = p.transforme.transform_point3(local);
                assert!(monde.z.abs() < 1e-3, "une jante sort du plan XY après le pivot, z = {}", monde.z);
                monde.y.atan2(monde.x).rem_euclid(TAU).to_degrees()
            })
            .collect();
        angles.sort_by(f32::total_cmp);
        for (k, a) in angles.iter().enumerate() {
            let attendu = 30.0 + 60.0 * k as f32;
            assert!((a - attendu).abs() < 0.5, "tronçon à {a:.1}°, attendu {attendu}°");
        }
    }

    // Stanford garde trois bras (une face de moyeu sur deux) ; Elysium en a
    // désormais **six** — une par face, l'hexagone n'en ayant que six. Chaque
    // station a donc son propre pas angulaire (120° / 60°), et c'est ce que le
    // tableau encode plutôt que de dupliquer le test.
    //
    // Stanford reste bâtie (et publiée) dans le plan XZ (axe étroit Y).
    // Elysium est bâtie dans le plan XZ en interne, mais `preset_elysium` la
    // pivote ensuite d'un quart de tour autour de X pour la publier dans le
    // plan XY (axe étroit Z, voir `elysium_fait_face_a_z`) — d'où l'axe étroit
    // et l'axe de l'angle qui diffèrent entre les deux lignes du tableau.
    #[test]
    fn les_bras_sont_regulierement_repartis_dans_le_plan_du_moyeu() {
        let plat_angle_xz = (|c: Vec3| (c.y, c.z.atan2(c.x))) as fn(Vec3) -> (f32, f32);
        let plat_angle_xy = (|c: Vec3| (c.z, c.y.atan2(c.x))) as fn(Vec3) -> (f32, f32);
        for (nom, etat, attendu_n, pas, plat_angle) in [
            ("stanford", preset_stanford(), STANFORD_N_BRAS, 120.0, plat_angle_xz),
            ("elysium", preset_elysium(), ELYSIUM_N_BRAS, 60.0, plat_angle_xy),
        ] {
            let EtatStation::Prete(st) = etat else { panic!("{nom} : station vide") };
            let charpentes: Vec<&crate::vaisseau::Piece> = st
                .pieces()
                .iter()
                .filter(|p| matches!(p.composant, Composant::Charpente { .. }))
                .collect();
            assert_eq!(charpentes.len(), attendu_n, "{nom} : {attendu_n} bras");

            let mut angles: Vec<f32> = charpentes
                .iter()
                .map(|p| {
                    let c = p.transforme.w_axis.truncate();
                    let (plat, angle) = plat_angle(c);
                    assert!(plat.abs() < 1e-4, "{nom} : bras hors du plan, coordonnée étroite = {plat}");
                    angle.rem_euclid(TAU).to_degrees()
                })
                .collect();
            angles.sort_by(f32::total_cmp);
            for (i, a) in angles.iter().enumerate() {
                let attendu = 30.0 + pas * i as f32;
                assert!((a - attendu).abs() < 0.5, "{nom} : bras {i} à {a:.1}°, attendu {attendu}°");
            }
        }
    }

    // Les deux mégastructures ont désormais des centres **distincts** : c'est
    // la raison d'être de la séparation, et ce test la garde.
    //
    // Mesuré sur la géométrie **cuite** et non sur le champ `longueur` : la
    // réduction de Stanford est une mise à l'échelle, elle vit dans la
    // transformée. Les deux pièces déclarent donc la même longueur tout en
    // n'ayant pas la même taille — lire la cote annoncée ne verrait rien.
    #[test]
    fn les_deux_megastructures_nont_pas_le_meme_centre() {
        // Écart entre les deux tambours accouplés = la taille réellement
        // dessinée. ⚠️ **La paire du moyeu**, c'est-à-dire les deux cœurs les
        // plus proches du centre : depuis que le pilier polaire coiffe ses bouts
        // du même composant, le tore en compte quatre.
        let taille = |e: EtatStation| -> f32 {
            let c = coeurs_tries(e);
            assert!(c.len() >= 2, "moins de deux habitats");
            c[0].centre().distance(c[1].centre())
        };
        let (s, e) = (taille(preset_stanford_moyeu()), taille(preset_elysium()));
        assert!(s < e, "Stanford {s:.2} devrait être plus petit qu'Elysium {e:.2}");
        assert!((s / e - STANFORD_ECHELLE).abs() < 1e-4, "rapport {:.4}", s / e);
    }

    // La réduction porte sur **toutes** les dimensions, diamètre compris — et
    // pas seulement sur la longueur, ce qui aurait allongé la silhouette.
    #[test]
    fn lhabitat_de_stanford_est_reduit_en_diametre_aussi() {
        // Même remarque que ci-dessus : le cœur du **moyeu**, donc le plus
        // proche du centre.
        let rayon = |e: EtatStation| -> f32 {
            let c = coeurs_tries(e);
            let p = c.first().expect("un habitat");
            // Une transformée rigide garde les longueurs ; ici l'échelle les
            // multiplie, et c'est exactement ce qu'on veut lire.
            p.transforme.x_axis.truncate().length() * p.composant.rayon_local()
        };
        let (s, e) = (rayon(preset_stanford_moyeu()), rayon(preset_elysium()));
        assert!(
            (s / e - STANFORD_ECHELLE).abs() < 1e-4,
            "diamètre non réduit : {s:.3} contre {e:.3} (rapport {:.4})",
            s / e
        );
    }

    // Le tunnel **remplit** le bras : deux fois plus long, deux fois plus de
    // modules — et non des modules deux fois plus longs.
    #[test]
    fn le_tunnel_suit_la_longueur_du_bras() {
        let modules = |bras: f32| -> usize {
            let mut asm = Assembleur::new();
            bras_et_tunnels(&mut asm, bras, 1); // un seul bras : compte isolé du nombre d'arms
            let EtatStation::Prete(st) = asm.terminer() else { panic!() };
            st.pieces()
                .iter()
                .filter(|p| {
                    matches!(
                        p.composant,
                        Composant::ModuleAxial { variante: VarianteModule::Standard, .. }
                    )
                })
                .count()
        };
        let (court, long) = (modules(STANFORD_BRAS), modules(ELYSIUM_BRAS));
        assert!(court > 0, "aucun module de tunnel");
        assert_eq!(long, 2 * court, "{court} → {long} modules pour un bras doublé");
    }

    // L'habitat du moyeu doit rester **ceinturé** par l'hexagone, pas l'avaler.
    //
    // Mesuré sur les fils des deux pièces réelles, en lisant `HABITAT_PROFIL`
    // et `ELYSIUM_PROFIL` : une version qui recopiait « P2 » et « P3 » dans le
    // test ne rougissait pas quand on poussait le profil au cran du dessus —
    // elle récitait le choix au lieu de le garder.
    #[test]
    fn lhabitat_du_moyeu_reste_ceinture_par_lhexagone() {
        // Rayon hors-tout de l'habitat, autour de son axe (local Z).
        let hab = Composant::ModuleAxial {
            profil: HABITAT_PROFIL,
            variante: VarianteModule::CoeurGris,
            longueur: ELYSIUM_HABITAT,
        };
        // ⚠️ Les bouts d'un `Fil` sont sur son **axe** : sans ajouter son rayon,
        // on mesurerait la ligne moyenne d'un cylindre et pas sa peau — c'est ce
        // qui faisait passer le sabotage à travers une première version.
        let r_hab = crate::vaisseau::fils(&hab)
            .iter()
            .flat_map(|f| [(f.a, f.rayon_a), (f.b, f.rayon_b)])
            .fold(0.0_f32, |m, (p, r)| m.max(p.truncate().length() + r));

        // Ceinture du moyeu : le bord extérieur d'une barre **au milieu d'une
        // face**, la direction la plus courte. Prendre l'étendue maximale (les
        // sommets, 15 % plus loin) déclarerait contenue une pièce qui ressort
        // déjà au travers de chaque face — c'est ce qui laissait passer le
        // sabotage.
        let r_hex = crate::vaisseau::hexagone_ceinture(ELYSIUM_PROFIL);

        assert!(
            r_hab < r_hex,
            "habitat {r_hab:.3} déborde le moyeu {r_hex:.3} — il l'avalerait au lieu d'y être ceinturé"
        );
    }

    // Les deux habitats du moyeu sont **parallèles à Y** — l'axe de rotation —
    // donc leurs écoutilles regardent hors du plan de l'anneau.
    #[test]
    fn les_habitats_du_moyeu_sont_paralleles_a_laxe() {
        // ⚠️ **La paire du moyeu** : les deux cœurs les plus proches du centre.
        // Les deux autres coiffent les pôles du pilier, et n'ont pas à
        // s'accoupler par le tambour.
        let coeurs = coeurs_tries(preset_stanford_moyeu());
        assert!(coeurs.len() >= 2, "moins de deux habitats");
        let mut tambours = Vec::new();
        for h in &coeurs[..2] {
            // L'axe local du module est +Z ; sa transformée doit l'amener sur Y.
            let axe = h.transforme.transform_vector3(Vec3::Z).normalize();
            assert!(axe.dot(Vec3::Y).abs() > 0.999, "habitat non parallèle à Y : {axe:?}");
            // Le **tambour** est le bout −Z : c'est par lui qu'ils s'accouplent.
            let centre = h.transforme.w_axis.truncate();
            // L'échelle vit dans la transformée : la lire là plutôt que de
            // supposer que la pièce est à sa taille nominale.
            let k = h.transforme.x_axis.truncate().length();
            tambours.push(centre - axe * (ELYSIUM_HABITAT * 0.5 * k));
        }
        // Les deux tambours se rejoignent au centre : accouplés par leur grande
        // face, cols vers l'extérieur — et non empilés bout à bout, ce qui
        // mettrait un col contre un tambour.
        assert!(
            tambours[0].distance(tambours[1]) < 1e-3,
            "tambours disjoints : {:?} et {:?}",
            tambours[0],
            tambours[1]
        );
        assert!(tambours[0].length() < 1e-3, "l'accouplement n'est pas au centre");
    }

    // Le tore doit **boucler au bout des bras**, et sa section valoir la hauteur
    // du moyeu. Les deux cotes se déduisent l'une de l'autre dans le preset ;
    // ce test les mesure sur la géométrie publiée, pas sur les constantes.
    #[test]
    fn le_tore_de_stanford_ceinture_le_bout_des_bras() {
        let EtatStation::Prete(st) = preset_stanford() else { panic!("station vide") };
        let tore = st
            .pieces()
            .iter()
            .find_map(|p| match p.composant {
                Composant::Tore { rayon_majeur, rayon_mineur, .. } => Some((rayon_majeur, rayon_mineur)),
                _ => None,
            })
            .expect("le tore de Stanford");

        // Bout d'un bras = centre du bras + demi-longueur, mesuré sur la pièce.
        let bout = st
            .pieces()
            .iter()
            .filter_map(|p| match p.composant {
                Composant::Charpente { longueur, .. } => {
                    Some(p.transforme.w_axis.truncate().length() + longueur * 0.5)
                }
                _ => None,
            })
            .fold(0.0_f32, f32::max);
        assert!((tore.0 - bout).abs() < 1e-3, "rayon majeur {} vs bout de bras {bout}", tore.0);

        // Diamètre de la section = hauteur du moyeu (= côté de son hexagone).
        assert!(
            (2.0 * tore.1 - hauteur_moyeu()).abs() < 1e-4,
            "section {} pour une hauteur de moyeu {}",
            2.0 * tore.1,
            hauteur_moyeu()
        );
    }

    // Les jonctions du vitrage doivent tomber **sur les bras**. Un décalage
    // mettrait du verre là où passe l'effort, et ne se verrait qu'à l'œil, de
    // trois quarts, sur un anneau de 150 unités de tour.
    #[test]
    fn les_jonctions_du_tore_tombent_sur_les_bras() {
        let EtatStation::Prete(st) = preset_stanford() else { panic!() };
        let (jonctions, phase) = st
            .pieces()
            .iter()
            .find_map(|p| match p.composant {
                Composant::Tore { jonctions, phase, .. } => Some((jonctions, phase)),
                _ => None,
            })
            .expect("le tore");

        let mut bras: Vec<f32> = st
            .pieces()
            .iter()
            .filter(|p| matches!(p.composant, Composant::Charpente { .. }))
            .map(|p| {
                let c = p.transforme.w_axis.truncate();
                c.z.atan2(c.x).rem_euclid(TAU)
            })
            .collect();
        bras.sort_by(f32::total_cmp);
        assert_eq!(jonctions, bras.len(), "une jonction par bras");

        let mut vues: Vec<f32> =
            (0..jonctions).map(|k| (phase + TAU * k as f32 / jonctions as f32).rem_euclid(TAU)).collect();
        vues.sort_by(f32::total_cmp);
        for (j, b) in vues.iter().zip(&bras) {
            let d = (j - b + PI).rem_euclid(TAU) - PI;
            assert!(d.abs() < 1e-3, "jonction à {:.1}° pour un bras à {:.1}°", j.to_degrees(), b.to_degrees());
        }
    }

    // Elysium porte un **second anneau** : une copie du `Composant::Tore` de
    // Stanford — sans son moyeu ni ses bras, Elysium ayant déjà les siens —
    // au tiers du **diamètre** total (donc deux tiers du rayon), lui-même
    // divisé par deux sur demande, strictement entre le moyeu et la jante
    // extérieure, jonctions calées sur les six bras.
    #[test]
    fn elysium_a_un_second_anneau_a_la_moitie_du_tiers_du_diametre() {
        let EtatStation::Prete(st) = preset_elysium() else { panic!() };
        let tores: Vec<&crate::vaisseau::Piece> =
            st.pieces().iter().filter(|p| matches!(p.composant, Composant::Tore { .. })).collect();
        assert_eq!(tores.len(), 1, "un seul second anneau");

        let (rayon_majeur, jonctions, phase) = match &tores[0].composant {
            Composant::Tore { rayon_majeur, jonctions, phase, .. } => (*rayon_majeur, *jonctions, *phase),
            _ => unreachable!(),
        };
        let attendu = (2.0 * elysium_rayon_jante() / 3.0) * 0.5;
        assert!(
            (rayon_majeur - attendu).abs() < 1e-4,
            "rayon majeur {rayon_majeur}, attendu {attendu} (moitié du tiers du diamètre)"
        );
        assert!(
            rayon_majeur > 0.0 && rayon_majeur < elysium_rayon_jante(),
            "hors de l'intervalle moyeu/jante extérieure : {rayon_majeur}"
        );
        assert_eq!(jonctions, ELYSIUM_N_BRAS, "une jonction par bras");
        assert!((phase - PI / 6.0).abs() < 1e-5, "phase non calée sur les bras");
    }

    // La longueur des bras **dérive** de l'épine de l'ISV. Si quelqu'un rallonge
    // l'épine sans y penser, ce test dit que les bras ont suivi — c'est le
    // contraire d'un verrou de valeur.
    #[test]
    fn un_bras_delysium_vaut_la_moitie_de_lepine_de_lisv() {
        assert!((ELYSIUM_BRAS * 2.0 - EPINE_LONGUEUR).abs() < 1e-5);
        assert!((STANFORD_BRAS * 4.0 - EPINE_LONGUEUR).abs() < 1e-5);
    }
