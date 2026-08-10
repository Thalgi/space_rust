//! Palette de l'assembleur — `docs/conception/assembleur.md` §6.5, §10.1, §10.2.
//!
//! **Le contenu et le grisage, pas le dessin.** L'écran (`ecran::assembleur`)
//! ne fait que poser les rectangles ; ce qui se *décide* — quelles entrées, dans
//! quelle catégorie, et lesquelles sont posables sur le chantier courant — vit
//! ici, et se teste (§10.9).

use crate::vaisseau::{
    categorie, echantillons, posables, CategoriePalette, Chantier, Composant, GenrePort, Profil,
};

/// Une entrée de palette : un composant représentatif et sa catégorie.
#[derive(Clone)]
pub struct Entree {
    pub comp: Composant,
    pub cat: CategoriePalette,
}

/// Contenu de la palette : les échantillons **montables**, rangés par catégorie.
///
/// Construit **une fois** — la liste ne dépend pas du chantier, seul le grisage
/// en dépend. La reconstruire par image rebâtirait 31 composants pour rien,
/// le défaut que `VueStation` a déjà corrigé sur sa colonne d'items.
///
/// ## Le critère est le **port**, pas la catégorie
///
/// Une entrée sans port de montage (`ports()` rend `[]`) ne peut jamais être
/// proposée par [`posables`], quel que soit le port visé : elle resterait grisée
/// à jamais — « une entrée morte qui se clique et ne fait rien » (§6.5).
///
/// ⚠️ Première rédaction : exclure les **catégories** `PoseeAMain` et `Effet`,
/// dont la documentation dit qu'aucune de leurs variantes n'a de port. C'est
/// vrai, mais **la réciproque est fausse** — et le test l'a trouvée tout de
/// suite : `Volet` est rangé en `Structure` et n'a aucun port. Le tore et le
/// bouclier thermique ne sont pas les seuls à se poser à la main ; la catégorie
/// est un rangement de menu, pas une propriété de montage.
///
/// Filtrer sur le port **subsume** l'exclusion par catégorie (les deux
/// catégories tombent d'elles-mêmes) et ne laisse plus aucune liste à tenir à
/// jour à la main.
pub fn contenu() -> Vec<Entree> {
    let mut v = Vec::new();
    for cat in CategoriePalette::TOUTES {
        for comp in echantillons() {
            if categorie(&comp) == cat && !comp.ports().is_empty() {
                v.push(Entree { comp, cat });
            }
        }
    }
    v
}

/// Catégories à afficher, dans l'ordre de la colonne : celles qui portent au
/// moins une entrée. **Déduite du contenu**, jamais écrite à part — une
/// catégorie vide serait un en-tête qui ne déplie rien, et une liste tenue en
/// parallèle finirait par diverger.
pub fn categories(entrees: &[Entree]) -> Vec<CategoriePalette> {
    let mut v: Vec<CategoriePalette> = Vec::new();
    for e in entrees {
        if !v.contains(&e.cat) {
            v.push(e.cat);
        }
    }
    v
}

/// Pour chaque entrée, peut-elle se poser **quelque part** sur ce chantier ?
///
/// C'est le grisage de §10.2. Une entrée qui ne se pose nulle part doit se voir
/// éteinte, sinon l'utilisateur la choisit et découvre un vaisseau sans aucun
/// port allumé — un cul-de-sac muet.
///
/// **La route compte.** La façon naïve balaie, pour chacun des 31 composants,
/// tous les ports libres : le coût croît avec la station. On relève d'abord les
/// couples `(genre, profil)` **distincts** parmi les ports libres — une poignée,
/// quatre genres et quelques profils — puis un `posables` par couple, et
/// l'union. Le coût cesse de dépendre du nombre de ports.
///
/// **Grisage = incompatible partout, pas encombré partout.** `posables` ne
/// juge que le genre et le profil ; l'anti-collision n'entre pas ici. Une
/// entrée allumée peut donc n'avoir que des ports **rouges** — compatibles mais
/// encombrés. C'est exactement la distinction de §8.4, et elle se joue port par
/// port (L4.4), pas au niveau de la palette : un `peut_poser` par entrée et par
/// port coûterait le balayage que §10.2 cherche justement à éviter.
///
/// ⚠️ **Chantier vide : tout est posable** (§10.1). Sans pièce il n'y a aucun
/// port libre, donc l'union est vide et *toute* la palette serait grisée — un
/// écran où rien ne peut jamais commencer. La première pièce ne se pose pas
/// comme les autres : `racine` la place à l'origine, sans port hôte.
pub fn grisage(chantier: &Chantier, entrees: &[Entree]) -> Vec<bool> {
    if chantier.nb_pieces() == 0 {
        return vec![true; entrees.len()];
    }
    let mut couples: Vec<(GenrePort, Profil)> = Vec::new();
    for p in chantier.libres() {
        if !couples.contains(&(p.genre, p.profil)) {
            couples.push((p.genre, p.profil));
        }
    }
    let mut union: Vec<Composant> = Vec::new();
    for (g, pr) in couples {
        for (comp, _) in posables(g, pr) {
            if !union.contains(&comp) {
                union.push(comp);
            }
        }
    }
    entrees.iter().map(|e| union.contains(&e.comp)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vaisseau::{Profil, VarianteModule};

    fn module() -> Composant {
        Composant::ModuleAxial { profil: Profil::P1, variante: VarianteModule::Standard, longueur: 2.0 }
    }

    /// La palette est peuplée et couvre plusieurs familles — sans quoi les
    /// gardes qui suivent porteraient sur presque rien.
    #[test]
    fn la_palette_est_peuplee_et_variee() {
        let c = contenu();
        assert!(c.len() > 20, "palette maigre : {} entrées", c.len());
        let cats = categories(&c);
        assert!(cats.len() >= 5, "seulement {} catégories affichées", cats.len());
        // Les deux catégories sans port tombent d'elles-mêmes (voir `contenu`).
        for interdite in [CategoriePalette::PoseeAMain, CategoriePalette::Effet] {
            assert!(
                !cats.contains(&interdite),
                "{} n'a aucun port de montage et ne devrait pas s'afficher",
                interdite.nom()
            );
        }
    }

    /// **Le cas qui a fait tomber la première rédaction.** `Volet` est rangé en
    /// `Structure` — une catégorie bien affichée — mais n'expose aucun port :
    /// filtrer par catégorie l'aurait laissé passer, grisé à jamais. Le test
    /// nomme le contre-exemple pour qu'un retour au filtrage par catégorie
    /// rougisse au lieu de repasser inaperçu.
    #[test]
    fn une_variante_sans_port_dans_une_categorie_affichee_est_ecartee() {
        let volet = echantillons()
            .into_iter()
            .find(|c| c.nom() == "VOLET")
            .expect("échantillon VOLET absent");
        assert!(volet.ports().is_empty(), "VOLET a gagné des ports : le contre-exemple a changé");
        let c = contenu();
        assert!(
            categories(&c).contains(&categorie(&volet)),
            "la catégorie de VOLET ne s'affiche plus : le test ne prouve plus rien"
        );
        assert!(!c.iter().any(|e| e.comp == volet), "VOLET, sans port, est dans la palette");
    }

    /// **Les entrées sont groupées par catégorie**, pas éparpillées : la colonne
    /// se dessine en parcourant la liste une fois, et un groupe rompu couperait
    /// une catégorie en deux en-têtes.
    #[test]
    fn les_entrees_sont_groupees_par_categorie() {
        let c = contenu();
        let mut vues: Vec<CategoriePalette> = Vec::new();
        let mut courante: Option<CategoriePalette> = None;
        for e in &c {
            if courante != Some(e.cat) {
                assert!(!vues.contains(&e.cat), "catégorie {} reprise plus loin", e.cat.nom());
                vues.push(e.cat);
                courante = Some(e.cat);
            }
        }
    }

    /// **Toute variante qui a un port de montage atteint la palette.**
    ///
    /// C'est le premier des deux défauts de §6.5 : *une variante absente de la
    /// palette est invisible pour le joueur* — elle existe dans le code, coûte
    /// de la maintenance, et n'est jamais posée. Le test relie les deux critères
    /// qui devraient coïncider : avoir un port, et être affichable.
    ///
    /// Il garde aussi la **soustraction** de `categories()` : le jour où une
    /// catégorie s'ajoute à l'enum sans être exclue, ses variantes arrivent
    /// toutes seules ; si quelqu'un l'exclut à tort, ce test le dit.
    #[test]
    fn toute_variante_montable_est_dans_la_palette() {
        let c = contenu();
        let mut montables = 0;
        for comp in echantillons() {
            if comp.ports().is_empty() {
                continue; // posée à la main : `posables` ne la proposera jamais
            }
            montables += 1;
            assert!(
                c.iter().any(|e| e.comp == comp),
                "{} a des ports mais n'apparaît nulle part dans la palette (catégorie {})",
                comp.nom(),
                categorie(&comp).nom()
            );
        }
        assert!(montables > 20, "seulement {montables} variantes montables : garde peu exercée");
    }

    /// Et l'inverse : **rien d'inposable ne s'affiche.** Une entrée présente
    /// mais impossible à poser est le second défaut de §6.5 — elle se clique et
    /// ne fait rien. C'est ce qui justifie d'exclure `PoseeAMain` et `Effet`.
    #[test]
    fn aucune_entree_sans_port_de_montage() {
        for e in contenu() {
            assert!(
                !e.comp.ports().is_empty(),
                "{} n'a aucun port : entrée morte dans la palette",
                e.comp.nom()
            );
        }
    }

    /// **Sur un chantier vide, tout est posable** (§10.1). C'est la règle qui
    /// rend le premier clic possible : sans elle, aucun port libre n'existe,
    /// l'union est vide, et la palette entière serait éteinte.
    #[test]
    fn le_chantier_vide_n_eteint_rien() {
        let entrees = contenu();
        let gris = grisage(&Chantier::new(), &entrees);
        assert_eq!(gris.len(), entrees.len());
        assert!(gris.iter().all(|&p| p), "une palette éteinte sur un chantier vide : rien ne peut commencer");
    }

    /// **Dès qu'une pièce est là, le grisage discrimine.** Un grisage qui
    /// laisserait tout allumé (ou tout éteint) passerait le test précédent sans
    /// rien garantir : celui-ci exige que les deux réponses existent.
    #[test]
    fn un_chantier_pose_eteint_certaines_entrees_et_pas_d_autres() {
        let mut ch = Chantier::new();
        assert!(ch.racine(module()));
        let entrees = contenu();
        let gris = grisage(&ch, &entrees);
        let allumees = gris.iter().filter(|&&p| p).count();
        assert!(allumees > 0, "un module axial n'accepte rien : le grisage éteint tout");
        assert!(
            allumees < entrees.len(),
            "toutes les entrées posables sur un simple module : le grisage ne filtre rien"
        );
    }

    /// **Le grisage dit la vérité** : une entrée allumée doit avoir au moins un
    /// port libre qui l'accepte pour de bon — vérifié auprès du moteur
    /// (`Chantier::compatibles`), pas auprès du calcul qu'on teste.
    ///
    /// C'est la garde de §6.5 (« palette et moteur ne doivent pas diverger »)
    /// portée au grisage : les deux partent du même catalogue mais par des
    /// chemins opposés — `posables` va du port vers les composants,
    /// `compatibles` du composant vers les ports.
    #[test]
    fn une_entree_allumee_a_vraiment_un_port_qui_l_accepte() {
        let mut ch = Chantier::new();
        ch.racine(module());
        let id = ch.libres()[0].id;
        ch.poser(id, module(), 1); // deux pièces : plus de ports, plus de genres

        let entrees = contenu();
        let gris = grisage(&ch, &entrees);
        let mut verifiees = 0;
        for (e, &allumee) in entrees.iter().zip(&gris) {
            // Indice de montage : le premier port du composant, comme `posables`.
            let Some(idx) = (0..e.comp.ports().len())
                .find(|&i| !ch.compatibles(&e.comp, i).is_empty())
            else {
                assert!(!allumee, "{} : allumée, mais aucun montage n'a de port hôte", e.comp.nom());
                continue;
            };
            assert!(allumee, "{} : éteinte, alors que le montage {idx} trouve un hôte", e.comp.nom());
            verifiees += 1;
        }
        assert!(verifiees > 0, "aucune entrée allumée : la garde ne s'exerce pas");
    }
}

