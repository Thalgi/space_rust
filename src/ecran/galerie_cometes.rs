//! Galerie des comètes : **une case par famille**.
//!
//! ⚠️ **Elle montrait des orbites, elle montre maintenant des comètes.** Le
//! premier jet alignait des ellipses — l'argument était qu'un noyau fait
//! quelques pixels et se ressemble d'un exemplaire à l'autre. C'était vrai du
//! noyau, et faux de la comète : ce qu'on vient voir, c'est **la queue**, sa
//! couleur et sa forme. Signalé à l'écran : *« la vue comète avec des
//! trajectoires n'est pas vraiment ce que je recherchais à la base »*.
//!
//! Chaque case pose donc la même comète au **même point de son plongeon**, et ne
//! change qu'une chose : sa **famille** (`comete::CATALOGUE`). Toute différence
//! visible entre deux cases vient donc de la matière et des queues, de rien
//! d'autre — c'est ce qui en fait une comparaison et non une vitrine.

use crate::astre::{Astre, CameraInfo};
use crate::comete::{self, TypeQueue};
use crate::genese::apparence_comete;
use crate::orbite::Orbite;
use crate::planete::Planete;
use crate::soleil::Soleil;
use macroquad::prelude::*;

/// **L'orbite de la vitrine.** Pas celle du jeu : la planche a son échelle,
/// comme celle des étoiles. Une orbite de 30 UA dans une case de 300 px ne se
/// lirait pas, et ce qu'on juge ici n'est pas une trajectoire.
const DEMI_GRAND_AXE: f32 = 3.5;
const EXCENTRICITE: f32 = 0.9;

/// Rayon de l'étoile de la vitrine. Petite à dessein : elle donne le foyer,
/// l'éclairage et surtout la **direction anti-solaire** — sans elle, on ne
/// verrait pas que la queue la fuit.
const RAYON_ETOILE: f32 = 0.07;

/// Rayon du noyau, celui des comètes tirées par la genèse.
const RAYON_NOYAU: f32 = 0.09;

/// Proportions d'une case et champ de sa caméra. ⚠️ **Source unique** : le
/// découpage de l'écran et le cadrage calculé plus bas s'en servent tous deux.
/// Deux valeurs auraient fini par diverger, et la queue serait sortie du cadre
/// d'un côté seulement.
const RATIO_CASE: f32 = 0.72;
const FOVY: f32 = 45.0;

/// **La comète est posée à son périhélie**, là où sa queue est la plus franche.
/// L'étoile est à l'origine, donc la queue part vers les `x` croissants.
fn position_comete() -> Vec3 {
    vec3(DEMI_GRAND_AXE * (1.0 - EXCENTRICITE), 0.0, 0.0)
}

fn orbite() -> Orbite {
    Orbite::new(
        DEMI_GRAND_AXE,
        EXCENTRICITE,
        Vec3::X,
        Vec3::Z,
        crate::systeme::G * crate::genese::MASSE_ETOILE,
        0.0,
    )
}

/// **La vitesse au périhélie**, perpendiculaire au rayon.
///
/// ⚠️ Elle n'est pas décorative : c'est elle qui donne sa **courbure** à la queue
/// de poussière (`comete::queue_poussiere`). Posée le long de `+Z`, la traînée
/// part vers `-Z`, donc vers le bas de la case : la courbure se voit.
fn vitesse_comete() -> Vec3 {
    vec3(0.0, 0.0, orbite().etat(0.0).1.length())
}

/// **Jusqu'où s'étend la plus longue queue**, mesurée sur les rubans eux-mêmes.
///
/// ⚠️ **Mesurée, pas estimée.** La queue d'ions fait 1,4 fois celle de poussière
/// et toutes les familles n'ont pas les mêmes ; un cadrage réglé à la main sur
/// « à peu près deux unités » couperait la plus longue dès qu'on touche à une
/// constante de `comete`. C'est le cadrage qui suit la queue, jamais l'inverse.
fn etendue_queues() -> f32 {
    let (p0, v) = (position_comete(), vitesse_comete());
    let seuil = comete::seuil_activite(DEMI_GRAND_AXE, EXCENTRICITE);
    let mut max: f32 = 0.0;
    for i in 0..comete::CATALOGUE.len() {
        for ruban in comete::rubans(comete::preset(i), p0, Vec3::ZERO, v, RAYON_NOYAU, seuil) {
            for (p, demi, _) in ruban {
                max = max.max(p.x - p0.x + demi);
            }
        }
    }
    max
}

/// **Demi-largeur du monde visible** dans une case, pour une caméra posée à
/// `hauteur` au-dessus du plan et regardant vers le bas.
///
/// Source unique de la projection : [`cadrage`] s'en sert pour choisir la
/// hauteur, et sa garde pour vérifier ce qui tient dans le cadre.
fn demi_champ(hauteur: f32) -> f32 {
    hauteur * (FOVY * 0.5).to_radians().tan() / RATIO_CASE
}

/// **Le cadrage d'une case** : `(abscisse visée, hauteur de caméra, demi-largeur
/// visible)`.
///
/// Il doit tenir **l'étoile et le bout de la plus longue queue**, avec une marge.
/// L'étoile donne l'anti-solaire : la rogner ferait perdre le sens de l'image.
///
/// La demi-largeur est rendue plutôt que recalculée par l'appelant : elle sort de
/// [`demi_champ`], et deux chemins vers la même grandeur finissent par diverger.
fn cadrage() -> (f32, f32, f32) {
    const MARGE: f32 = 1.12;
    // ⚠️ De **l'étoile entière** au bout de la queue. Le cadre partait du
    // centre de l'étoile : tant que la queue était longue, la marge couvrait
    // son rayon ; divisée par quatre (2026-10-20), l'étoile sortait du cadre.
    let (x0, x1) = (-RAYON_ETOILE, position_comete().x + etendue_queues());
    let demi = (x1 - x0) * 0.5 * MARGE;
    // Inverse de `demi_champ` : la hauteur qui donne exactement la largeur voulue.
    let hauteur = demi * RATIO_CASE / (FOVY * 0.5).to_radians().tan();
    ((x0 + x1) * 0.5, hauteur, demi_champ(hauteur))
}

/// Une case : sa famille, et la comète qui la montre.
struct Cellule {
    libelle: String,
    comete: Planete,
}

/// Le libellé d'une case : le nom de la famille, et ce qu'elle porte.
fn libelle(p: &comete::Preset) -> String {
    let q = match p.queues {
        TypeQueue::Poussiere => "poussiere",
        TypeQueue::Ions => "ions",
        TypeQueue::Double => "poussiere + ions",
    };
    format!("{}   -   {q}", p.nom)
}

/// Galerie des comètes (comparaison des familles).
pub struct GalerieCometes {
    cellules: Vec<Cellule>,
    etoile: Soleil,
    /// Défilement doux, molette normalisée (`super::defilement`).
    scroll: super::defilement::Defilement,
}

impl GalerieCometes {
    pub fn new() -> Self {
        let (pos, vit) = (position_comete(), vitesse_comete());
        let cellules = (0..comete::CATALOGUE.len())
            .map(|i| Cellule {
                libelle: libelle(comete::preset(i)),
                // ⚠️ **Avec son orbite** : le seuil d'activité de la queue s'en
                // dérive (`comete::seuil_activite`). Sans elle, la planche
                // montrerait des noyaux nus — et c'est la queue qu'elle sert à
                // juger.
                //
                // ⚠️ Et **la même graine pour toutes** : le patatoïde doit être le
                // même d'une case à l'autre, sans quoi on ne saurait pas si une
                // différence vient de la famille ou du tirage.
                comete: Planete::new(pos, vit, RAYON_NOYAU, 1e-6, apparence_comete(), Vec::new())
                    .avec_orbite(orbite())
                    .en_comete_preset(0.37, i),
            })
            .collect();
        Self {
            etoile: Soleil::new(Vec3::ZERO, RAYON_ETOILE, vec3(1.0, 0.95, 0.85), 1.0),
            cellules,
            scroll: Default::default(),
        }
    }

    /// Une frame. Renvoie `true` pour revenir à l'accueil (Échap).
    pub fn frame(&mut self) -> bool {
        if is_key_pressed(KeyCode::Escape) {
            return true;
        }
        if is_key_pressed(KeyCode::R) {
            crate::soleil::vider_cache_materials();
            *self = Self::new();
        }
        let dt = get_frame_time().min(0.05);
        clear_background(Color::new(0.01, 0.01, 0.03, 1.0));

        let n = self.cellules.len().max(1);
        let top = 48.0;
        let label_h = 22.0;
        let cols = 2.min(n);
        let cw = screen_width() / cols as f32;
        let ch = cw * RATIO_CASE + label_h;
        let render_h = ch - label_h;
        let rows = n.div_ceil(cols);
        let h_vue = screen_height() - top;
        let max_scroll = (rows as f32 * ch - h_vue).max(0.0);
        let scroll = self.scroll.maj(max_scroll, get_frame_time());

        let (cible_x, hauteur, _) = cadrage();
        let cible = vec3(cible_x, 0.0, 0.0);

        self.etoile.update(dt);
        let mut labels: Vec<(String, f32, f32)> = Vec::new();
        for (i, c) in self.cellules.iter_mut().enumerate() {
            c.comete.update(dt);
            let cell_x = (i % cols) as f32 * cw;
            let cell_y = top + (i / cols) as f32 * ch - scroll;
            if cell_y + render_h < top || cell_y > screen_height() {
                continue;
            }
            // **Vue de dessus.** La queue s'étale alors en travers de la case, de
            // l'étoile vers la droite, et sa courbure vers le bas — les deux
            // choses qu'on vient comparer sont dans le plan de l'image.
            let pos = cible + vec3(0.0, hauteur, 0.0);
            let cam3d = Camera3D {
                position: pos,
                target: cible,
                // Vue plongeante : « le haut » est l'axe des Z, sans quoi
                // l'orientation serait indéfinie.
                up: Vec3::Z,
                fovy: FOVY.to_radians(),
                aspect: Some(cw / render_h),
                viewport: Some((
                    cell_x as i32,
                    (screen_height() - (cell_y + render_h)) as i32,
                    cw as i32,
                    render_h as i32,
                )),
                ..Default::default()
            };
            set_camera(&cam3d);

            let (right, up, forward) = crate::camera::base_camera(cible - pos);
            let cam = CameraInfo {
                pos,
                origine: Vec3::ZERO,
                right,
                up,
                forward,
                light_pos: Vec3::ZERO, // l'étoile est au foyer
                light_color: Vec3::ONE,
                lights_pos: [Vec3::ZERO; 4],
                lights_color: [Vec3::ONE, Vec3::ZERO, Vec3::ZERO, Vec3::ZERO],
            };

            self.etoile.draw(&cam);
            c.comete.draw(&cam);
            labels.push((c.libelle.clone(), cell_x, cell_y + render_h + 16.0));
        }

        set_default_camera();
        let col = Color::new(0.9, 0.85, 0.6, 1.0);
        for (nom, cell_x, y) in &labels {
            let tw = crate::police::mesure(nom, 18);
            crate::police::texte(nom, cell_x + (cw - tw) * 0.5, *y, 18.0, col);
        }
        draw_rectangle(0.0, 0.0, screen_width(), top, Color::new(0.01, 0.01, 0.03, 1.0));
        crate::police::texte(
            "GALERIE DES COMETES   -   une case par famille, toutes au meme perihelie   -   \
             l'etoile est a gauche   -   molette: defiler   Echap: menu",
            12.0,
            30.0,
            18.0,
            Color::new(0.7, 0.85, 0.85, 1.0),
        );
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seuil() -> f32 {
        comete::seuil_activite(DEMI_GRAND_AXE, EXCENTRICITE)
    }

    /// **Tout le catalogue est exposé, une fois chacun.**
    ///
    /// ⚠️ C'est la raison d'être de la planche : ajouter une famille à
    /// `comete::CATALOGUE` sans l'y montrer la rendrait invisible, et personne ne
    /// s'en apercevrait — le défaut le plus tenace de ce chantier.
    #[test]
    fn la_planche_expose_tout_le_catalogue() {
        let noms: Vec<&str> = comete::CATALOGUE.iter().map(|p| p.nom).collect();
        assert!(noms.len() >= 4, "un catalogue de {} familles ne se compare pas", noms.len());
        let mut tries = noms.clone();
        tries.sort_unstable();
        tries.dedup();
        assert_eq!(tries.len(), noms.len(), "deux familles portent le même nom : {noms:?}");
        for p in comete::CATALOGUE.iter() {
            // Le libellé dit la famille ET ce qu'elle porte : sans le second, la
            // planche ne montrerait pas qu'il existe plusieurs types de queue.
            assert!(libelle(p).contains(p.nom), "le libellé perd le nom : {}", libelle(p));
        }
    }

    /// **Les trois types de queue sont représentés.**
    ///
    /// Une planche qui ne montrerait que des comètes à double queue ne dirait pas
    /// qu'il y a un choix — et c'est exactement ce qu'on vient y chercher.
    #[test]
    fn le_catalogue_couvre_les_trois_types_de_queue() {
        for t in [TypeQueue::Poussiere, TypeQueue::Ions, TypeQueue::Double] {
            assert!(comete::CATALOGUE.iter().any(|p| p.queues == t), "aucune famille ne porte {t:?}");
        }
    }

    /// **Deux familles se distinguent à l'œil**, du noyau comme de la queue.
    ///
    /// ⚠️ Sans cette garde, un catalogue de cinq nuances de gris passerait tous
    /// les autres tests : il aurait le bon nombre d'entrées, les bons types de
    /// queue, et ne montrerait rien. La planche promet des **colorations
    /// différentes** ; c'est ça qu'on mesure.
    ///
    /// ⚠️ **Et sur les deux à la fois, sans échappatoire.** Le premier jet
    /// acceptait deux familles identiques dès lors que leurs types de queue
    /// différaient — un sabotage qui recopiait les couleurs de « Glacée » dans
    /// « Poussiéreuse » le laissait **vert**. Or ce que la planche promet est la
    /// **coloration selon la matière** : deux cases de la même couleur ne
    /// l'illustrent pas, quoi qu'elles portent.
    #[test]
    fn les_familles_se_distinguent_a_l_oeil() {
        let ecart = |a: Vec3, b: Vec3| (a - b).length();
        let teinte = |c: Color| vec3(c.r, c.g, c.b);
        for i in 0..comete::CATALOGUE.len() {
            for j in (i + 1)..comete::CATALOGUE.len() {
                let (a, b) = (comete::preset(i), comete::preset(j));
                // La queue visible de chacune : celle qu'elle porte vraiment.
                let qa = if a.a_poussiere() { a.poussiere } else { a.ions };
                let qb = if b.a_poussiere() { b.poussiere } else { b.ions };
                let d_noyau = ecart(a.glace, b.glace);
                let d_queue = ecart(teinte(qa), teinte(qb));
                assert!(
                    d_noyau > 0.08 && d_queue > 0.08,
                    "« {} » et « {} » se ressemblent trop : noyau {d_noyau:.3}, queue {d_queue:.3}",
                    a.nom,
                    b.nom
                );
            }
        }
    }

    /// **Chaque case montre une queue franche**, et pas un filament.
    ///
    /// ⚠️ La planche est posée au **périhélie** exprès : c'est le seul endroit où
    /// la queue est à son maximum. Une case tiède ne permettrait de juger ni la
    /// couleur ni la forme.
    #[test]
    fn chaque_case_montre_une_queue_franche() {
        let (p0, v) = (position_comete(), vitesse_comete());
        let a = comete::activite(p0.length(), seuil());
        assert!(a > 0.3, "la planche est posée là où la comète ne fume pas ({a})");

        for i in 0..comete::CATALOGUE.len() {
            let p = comete::preset(i);
            let rubans = comete::rubans(p, p0, Vec3::ZERO, v, RAYON_NOYAU, seuil());
            let attendu = if p.queues == TypeQueue::Double { 2 } else { 1 };
            assert_eq!(rubans.len(), attendu, "« {} » : {} queue(s)", p.nom, rubans.len());
            // Et elle se voit : l'opacité au ras du noyau doit être franche.
            for r in &rubans {
                assert!(r[0].2.a > 0.5, "« {} » : queue à {} d'opacité", p.nom, r[0].2.a);
            }
        }
    }

    /// **Le cadre tient l'étoile ET le bout de la plus longue queue.**
    ///
    /// ⚠️ C'est tout l'objet du cadrage calculé. Une queue rognée au bord de la
    /// case se lirait comme une queue courte, et l'étoile hors champ ferait
    /// perdre la direction anti-solaire — sans elle, rien ne dit pourquoi la
    /// queue part de ce côté.
    ///
    /// Le test énonce **l'exigence** ; [`cadrage`] en est la solution et
    /// [`demi_champ`] la projection partagée : aucune arithmétique n'est recopiée.
    #[test]
    fn le_cadre_tient_l_etoile_et_le_bout_de_la_queue() {
        let (cible, _, demi) = cadrage();
        let (gauche, droite) = (cible - demi, cible + demi);
        let bout = position_comete().x + etendue_queues();

        assert!(gauche <= -RAYON_ETOILE, "l'étoile sort du cadre à gauche ({gauche})");
        assert!(droite >= bout, "la queue est rognée : cadre jusqu'à {droite}, queue jusqu'à {bout}");
        // Mais pas un cadre vide : la comète doit remplir sa case.
        assert!(droite < bout * 1.6, "le cadre est trop large, la comète y est perdue");
    }

    /// **La planche a son échelle à elle**, pas celle du jeu.
    ///
    /// ⚠️ Même piège qu'à la galerie des étoiles, où poser le rayon du tirage
    /// aurait fait de la naine brune le plus gros objet de la vitrine. Ici
    /// l'orbite du jeu fait des dizaines d'UA : à l'échelle, la case ne montrerait
    /// qu'un trait.
    #[test]
    fn la_planche_a_son_echelle_a_elle() {
        const _: () =
            assert!(DEMI_GRAND_AXE < 10.0, "une orbite de jeu ne tiendrait pas dans la case");
        // Et la comète reste hors de l'étoile dessinée.
        assert!(
            position_comete().x > RAYON_ETOILE * 1.3,
            "la comète traverse l'étoile de la vitrine : {}",
            position_comete().x
        );
    }
}
