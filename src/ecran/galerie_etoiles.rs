use crate::astre::{Astre, CameraInfo};
use crate::etoile::{couleur_corps_noir, couleur_etoile};
use crate::soleil::Soleil;
use crate::disque::Accretion;
use crate::stellaire::Variante;
use macroquad::prelude::*;

/// **Étoiles par ligne.**
///
/// C'était une division de la largeur d'écran par 210 px : en 1440p, douze
/// colonnes, donc douze étoiles minuscules — trop petites pour qu'on y
/// juge la granulation, une tache ou une boucle, alors que c'est justement
/// cette vue qui sert à les juger (`astres_v2.md` §0 : rien ne se lit sous
/// ~10 px de rayon). Un nombre fixe fait grandir les cellules avec l'écran
/// au lieu de les multiplier.
const COLONNES: usize = 5;

/// Catalogue des étoiles : (nom, rayon, température K, luminosité, couronne 0=halo 1=jets 2=vent).
fn catalogue() -> Vec<(&'static str, f32, f32, f32, f32)> {
    vec![
        // Séquence principale O -> M.
        ("O (bleue)", 1.05, 38000.0, 4.0, 0.0),
        ("B (bleu-blanc)", 0.98, 20000.0, 3.0, 0.0),
        ("A (blanche)", 0.9, 9000.0, 2.0, 0.0),
        ("F (blanc-jaune)", 0.85, 6800.0, 1.4, 0.0),
        ("G (jaune - Soleil)", 0.8, 5700.0, 1.0, 0.0),
        ("K (orange)", 0.75, 4500.0, 0.7, 0.0),
        ("M (naine rouge)", 0.58, 3200.0, 0.35, 0.0),
        // Naines / sous-naines.
        ("Naine blanche", 0.45, 16000.0, 0.3, 0.0),
        // ⚠️ Posée **juste après** sa jumelle : même taille, même éclat, même
        // teinte. Seule la pulsation les sépare, et c'est côte à côte qu'on la
        // juge (`etoile::pulsation`).
        ("Naine blanche pulsante (ZZ Ceti)", 0.45, 11800.0, 0.3, 0.0),
        ("Sous-naine (sdB)", 0.55, 25000.0, 0.6, 0.0),
        // Géantes / supergéantes.
        ("Geante rouge", 1.15, 3400.0, 4.5, 0.0),
        // Idem : Mira recouvre la géante rouge, sa pulsation est sa signature.
        ("Mira", 1.18, 2900.0, 4.0, 0.0),
        ("Supergeante rouge", 1.2, 3500.0, 6.0, 0.0),
        ("Cepheide", 1.1, 5800.0, 3.0, 0.0),
        ("Supergeante bleue", 1.15, 21000.0, 6.0, 2.0), // vent stellaire
        // Particulières.
        ("Etoile carbonee (C)", 1.0, 2800.0, 2.5, 0.0),
        ("T Tauri (jeune)", 0.85, 4200.0, 1.2, 6.0), // jets bipolaires + disque protosolaire
        ("Herbig Ae/Be (jeune)", 2.6, 9000.0, 2.0, 6.0), // sa grande sœur : rideaux
        ("Etoile Be", 1.8, 18000.0, 4.0, 7.0),           // aplatie, disque de décrétion
        ("Hypergeante (LBV)", 3.0, 20000.0, 8.0, 8.0),   // Eta Carinae : lobes en bulbe
        ("Naine blanche polluee", 0.65, 16000.0, 0.27, 9.0), // verte, rose, cyan ou rouge
        ("Wolf-Rayet", 1.05, 40000.0, 5.0, 2.0),     // vent violent
        ("Etoile a flares (M)", 0.58, 3000.0, 0.4, 0.0),
        ("Etoile a neutrons", 0.42, 40000.0, 0.2, 1.0), // jets fixes
        ("Pulsar", 0.42, 40000.0, 0.25, 3.0),           // jets qui tournent (phare)
        ("Magnetar", 0.45, 40000.0, 0.3, 4.0), // arcs magnétiques
        // Lot 11. Rayon de **vitrine** : plus grosse que la supergéante rouge
        // (1,2) ; sa vraie taille, une UA, ne tiendrait pas dans une case.
        // Teinte pourpre, ébullition, couronne serrée et cœur : par son nom.
        ("Quasi-etoile", 1.35, 3600.0, 6.0, 0.0),
    ]
}

/// **Tout le catalogue du trou noir**, exposé côte à côte : ses trois
/// régimes, puis la cible visuelle concurrente.
///
/// En jeu le régime se **dérive de la graine** : on tombe sur l'un des trois,
/// et il faudrait régénérer jusqu'à voir les autres. Une planche-contact doit
/// les montrer ensemble — c'est sa raison d'être. Et la cible Interstellar,
/// qu'aucune graine n'atteint, ne se juge **que** dans une vue comme
/// celle-ci.
///
/// ⚠️ Le gabarit (rayon, température, luminosité) vient de
/// `etoile::particuliere`, et la liste des presets d'`Accretion::catalogue` :
/// **pas d'une copie**, ni d'un côté ni de l'autre. La galerie montre ce que
/// le jeu produit, ou elle ne sert à rien.
///
/// Elle construit des `Soleil`, ce qui exige le contexte graphique : elle ne
/// se teste donc pas. La **décision** — quels presets, sous quels noms —
/// est gardée chez `Accretion`, qui elle se teste.
fn trous_noirs() -> Vec<(String, Box<dyn Astre>)> {
    let (tmin, tmax, rayon, lumi, nom_tn, mode) = crate::etoile::particuliere(0.0);
    let couleur = couleur_corps_noir((tmin + tmax) * 0.5);
    Accretion::catalogue()
        .into_iter()
        .map(|regime| {
            let s =
                Variante::de_mode(mode).appliquer(Soleil::new(Vec3::ZERO, rayon, couleur, lumi), nom_tn);
            let s = s.avec_regime(regime);
            (format!("Trou noir - {}", regime.libelle()), Box::new(s) as Box<dyn Astre>)
        })
        .collect()
}

/// **Rayon de présentation d'une naine brune**, dans l'échelle de cette planche.
///
/// ⚠️ Ce n'est **pas** celui du tirage (≈ 2 unités). La planche a son échelle à
/// elle — une O bleue y vaut 1,05 contre 4,0 en jeu — et y poser le rayon du jeu
/// ferait de la naine brune le plus gros objet de la vitrine, alors qu'elle est
/// l'un des plus petits. Elle est de la taille de Jupiter, donc en deçà d'une
/// naine M (0,58 ici).
const RAYON_BRUNE: f32 = 0.40;

/// **Les températures des quatre classes de naine brune**, dans l'ordre du plus
/// chaud au plus froid.
///
/// ⚠️ Le **nom** de chaque classe n'est pas écrit ici : il vient de
/// `genese::preset_naine_brune`, la même source que le jeu. Une température et
/// un nom posés côte à côte auraient pu finir par ne plus se correspondre, et la
/// vitrine aurait montré une classe sous le nom d'une autre.
fn temperatures_naines_brunes() -> [f32; 4] {
    [2200.0, 1700.0, 1100.0, 600.0]
}

/// **Les quatre classes de naine brune**, exposées parmi les étoiles.
///
/// Sa place est ici depuis qu'elle tient le **centre** d'un système (lot 6.2) :
/// on veut la voir à côté d'une naine M, qui est ce dont elle se rapproche le
/// plus. Elle reste aussi au catalogue des géantes gazeuses, et c'est cohérent —
/// c'est bien par ce rendu qu'elle est peinte.
///
/// Elle construit des `Planete`, ce qui exige le contexte graphique : elle ne se
/// teste donc pas. La **décision** — quelles classes, sous quels noms — se garde
/// par [`temperatures_naines_brunes`] et `preset_naine_brune`.
fn naines_brunes() -> Vec<(String, Box<dyn Astre>)> {
    temperatures_naines_brunes()
        .into_iter()
        .map(|temp| {
            let nom = crate::genese::preset_naine_brune(temp);
            let app = crate::genese::preset_gazeuse(nom);
            let p = crate::planete::Planete::new(
                Vec3::ZERO,
                Vec3::ZERO,
                RAYON_BRUNE,
                1.0,
                app,
                Vec::new(),
            );
            (format!("{nom}  {} K", temp as i32), Box::new(p) as Box<dyn Astre>)
        })
        .collect()
}

/// **Les aspects d'un type à exposer** : `None` pour un type à aspect unique,
/// sinon ses deux formes, stable puis instable.
///
/// ⚠️ En jeu, l'aspect se dérive de la graine : on tombe sur l'un ou l'autre.
/// La planche les force tous les deux — c'est la seule vue où on les compare.
fn aspects(nom: &str) -> Vec<Option<bool>> {
    use crate::etoile::ebullition;
    if ebullition(nom, false) != ebullition(nom, true) {
        vec![Some(false), Some(true)]
    } else {
        vec![None]
    }
}

/// Galerie « planche-contact » des types d'étoiles (visualisation/calibration).
///
/// ⚠️ Les cellules sont des **`Astre`**, et non des `Soleil`. Depuis le lot 6.2
/// la naine brune tient le centre d'un système sans être une photosphère : elle
/// se rend en géante gazeuse, donc en `Planete`. La boucle de dessin n'appelait
/// déjà que `update` et `draw`, tous deux au trait — le type était plus étroit
/// que l'usage.
pub struct GalerieEtoiles {
    cellules: Vec<(String, Box<dyn Astre>)>,
    /// Défilement doux, molette normalisée (`super::defilement`).
    scroll: super::defilement::Defilement,
    /// Cible basse résolution du filtre pixel (recréée si la fenêtre change).
    cible: Option<RenderTarget>,
    rt_dims: (u32, u32),
}

impl GalerieEtoiles {
    pub fn new() -> Self {
        let cellules = catalogue()
            .into_iter()
            .flat_map(|(nom, rayon, temp, lumi, mode)| {
                aspects(nom).into_iter().map(move |instable| {
                    // ⚠️ `couleur_etoile`, pas `couleur_corps_noir` : une étoile
                    // carbonée doit sa teinte à un filtre moléculaire, et la
                    // vitrine la montrerait orange là où le jeu la peint rouge.
                    let s = Soleil::new(Vec3::ZERO, rayon, couleur_etoile(nom, temp), lumi);
                    // ⚠️ **La même source que la genèse.** Cette chaîne était
                    // recopiée ici, et les deux avaient déjà divergé : le
                    // `mode > 4.5` local attrapait aussi le mode 6, et le T Tauri
                    // du catalogue restait au mode 1, donc sans disque.
                    let s = Variante::de_mode(mode).appliquer(s, nom);
                    let (s, libelle) = match instable {
                        None => (s, nom.to_string()),
                        Some(i) => {
                            let (e, style) = crate::etoile::ebullition(nom, i);
                            let etat = if i { "instable" } else { "stable" };
                            (s.avec_ebullition(e, style), format!("{nom} - {etat}"))
                        }
                    };
                    (libelle, Box::new(s) as Box<dyn Astre>)
                })
            })
            .chain(trous_noirs())
            .chain(naines_brunes())
            .collect();
        Self { cellules, scroll: Default::default(), cible: None, rt_dims: (0, 0) }
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
        if is_key_pressed(KeyCode::P) {
            // Cycle NET -> PIXEL -> PIXEL+PALETTE, sur le réglage global.
            crate::reglages::regler_rendu(crate::reglages::mode_rendu().suivant());
        }
        let dt = get_frame_time().min(0.05);

        let fond = Color::new(0.01, 0.01, 0.03, 1.0);
        clear_background(fond);

        // Filtre pixel : même mécanique que la galerie tellurique — une cible
        // basse résolution, chaque cellule y garde son viewport divisé par
        // `PIX`, et `pixel::blit` remonte le tout (et quantifie si demandé).
        let pixelise = crate::reglages::mode_rendu().pixelise();
        if pixelise {
            let dims = super::pixel::dimensions();
            if self.rt_dims != dims || self.cible.is_none() {
                self.cible = Some(super::pixel::nouvelle_cible(dims.0, dims.1));
                self.rt_dims = dims;
            }
            set_camera(&Camera2D { render_target: self.cible.clone(), ..Default::default() });
            clear_background(fond);
            set_default_camera();
        }

        let n = self.cellules.len().max(1);
        let top = 48.0;
        let label_h = 22.0;
        let cols = COLONNES.min(n);
        let cw = screen_width() / cols as f32;
        // ⚠️ La hauteur de cellule était **fixe** (180 px) alors que la
        // largeur suit l'écran : en 1440p, chaque étoile se retrouvait dans
        // une case large et plate, trop petite pour qu'on y voie la
        // granulation ou une tache. Elle suit maintenant la largeur.
        let ch = cw * 0.72 + label_h;
        let render_h = ch - label_h;
        let rows = n.div_ceil(cols);
        let h_vue = screen_height() - top;
        let max_scroll = (rows as f32 * ch - h_vue).max(0.0);
        let scroll = self.scroll.maj(max_scroll, get_frame_time());

        // --- Phase 3D : dessiner chaque étoile dans son viewport. ---
        let mut labels: Vec<(String, f32, f32)> = Vec::new();
        for (i, (nom, corps)) in self.cellules.iter_mut().enumerate() {
            corps.update(dt); // animation (granulation, taches, éruptions, bandes)
            let cell_x = (i % cols) as f32 * cw;
            let cell_y = top + (i / cols) as f32 * ch - scroll;
            if cell_y + render_h < top || cell_y > screen_height() {
                continue;
            }
            let pos = vec3(0.0, 0.0, 7.5); // recul pour cadrer couronne / jets / vent
            let mut cam3d = Camera3D {
                position: pos,
                target: Vec3::ZERO,
                up: Vec3::Y,
                fovy: 45.0_f32.to_radians(),
                aspect: Some(cw / render_h),
                viewport: Some((
                    cell_x as i32,
                    (screen_height() - (cell_y + render_h)) as i32,
                    cw as i32,
                    render_h as i32,
                )),
                ..Default::default()
            };
            if pixelise {
                // Même repère bas-gauche que le rendu direct, divisé par
                // l'échelle : le blit (`flip_y`) préserve les hauteurs GL.
                let p = super::pixel::PIX as f32;
                cam3d.render_target = self.cible.clone();
                cam3d.viewport = Some((
                    (cell_x / p) as i32,
                    ((screen_height() - (cell_y + render_h)) / p) as i32,
                    (cw / p) as i32,
                    (render_h / p) as i32,
                ));
            }
            set_camera(&cam3d);

            let (right, up, forward) = crate::camera::base_camera(Vec3::ZERO - pos);
            let cam = CameraInfo {
                pos,
                // Le sujet est posé à l'origine du monde et la caméra à
                // quelques unités : aucun grand nombre, donc rien à
                // translater — comportement inchangé à l'identique.
                origine: Vec3::ZERO,
                right,
                up,
                forward,
                light_pos: vec3(0.0, 0.0, 10.0),
                light_color: Vec3::ONE,
                lights_pos: [vec3(0.0, 0.0, 10.0), Vec3::ZERO, Vec3::ZERO, Vec3::ZERO],
                lights_color: [Vec3::ONE, Vec3::ZERO, Vec3::ZERO, Vec3::ZERO],
            };
            corps.draw(&cam);
            labels.push((nom.clone(), cell_x, cell_y + render_h + 16.0));
        }

        // --- Phase 2D : texte après une seule remise en caméra écran. ---
        set_default_camera();
        if pixelise {
            if let Some(rt) = &self.cible {
                super::pixel::blit(&rt.texture);
            }
        }
        let col = Color::new(0.9, 0.85, 0.6, 1.0);
        for (nom, cell_x, y) in &labels {
            let tw = crate::police::mesure(nom, 18);
            crate::police::texte(nom, cell_x + (cw - tw) * 0.5, *y, 18.0, col);
        }

        draw_rectangle(0.0, 0.0, screen_width(), top, Color::new(0.01, 0.01, 0.03, 1.0));
        crate::police::texte(
            "GALERIE DES ETOILES   -   molette: defiler   R: shaders   P: filtre pixel   Echap: menu",
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

    /// **Aucune entrée du catalogue ne porte un mode que personne ne sait
    /// traduire.**
    ///
    /// Le catalogue est écrit à la main, et son mode est un `f32` que rien ne
    /// contraint : une valeur inconnue retombe sur `Normale` **en silence**,
    /// et la vitrine montre une étoile ordinaire là où elle promet un pulsar
    /// ou un disque. C'est la vue qui sert à juger les astres — elle ne peut
    /// pas mentir sur ce qu'elle montre.
    #[test]
    fn chaque_entree_du_catalogue_porte_un_mode_connu() {
        for (nom, _, _, _, mode) in catalogue() {
            let inconnu =
                matches!(Variante::de_mode(mode), Variante::Normale) && mode as i32 != 0;
            assert!(!inconnu, "{nom} porte le mode {mode}, que personne ne sait traduire");
        }
    }

    /// **Et les types qui portent un disque sont bien exposés** : c'est la
    /// seule vue où l'on puisse les juger côte à côte.
    #[test]
    fn les_porteurs_de_disque_sont_exposes() {
        let modes: Vec<f32> = catalogue().into_iter().map(|e| e.4).collect();
        assert!(modes.contains(&6.0), "aucune jeune étoile à disque en galerie");
        // ⚠️ Le trou noir a quitté le catalogue pour `trous_noirs()`, qui en
        // montre les **trois régimes**. En jeu celui-ci se dérive de la
        // graine : on tombe sur l'un des trois, et il faudrait régénérer
        // pour voir les autres — une planche-contact doit les montrer
        // ensemble.
        assert!(!modes.contains(&5.0), "le trou noir est resté au catalogue : il y sera en double");
    }

    /// **Les quatre classes de naine brune sont exposées, et toutes
    /// différentes.**
    ///
    /// ⚠️ La planche ne porte que des **températures** ; les noms viennent de
    /// `preset_naine_brune`. Deux températures tombant dans la même classe
    /// donneraient deux cellules identiques sous le même nom, et une classe
    /// resterait invisible — sans que rien ne le dise, puisque la vitrine
    /// afficherait bien quatre cases.
    #[test]
    fn les_quatre_classes_de_naine_brune_sont_exposees() {
        let noms: Vec<&str> =
            temperatures_naines_brunes().iter().map(|t| crate::genese::preset_naine_brune(*t)).collect();
        for (i, n) in noms.iter().enumerate() {
            assert!(!noms[..i].contains(n), "deux cellules montrent « {n} » : {noms:?}");
        }
        // Et ce sont bien les quatre du catalogue gazeux, pas trois plus une
        // répétition.
        assert_eq!(noms.len(), 4);
        // Du plus chaud au plus froid : la planche se lit comme une séquence.
        let t = temperatures_naines_brunes();
        for i in 1..t.len() {
            assert!(t[i] < t[i - 1], "les classes ne descendent pas en température : {t:?}");
        }
        // ⚠️ Et le rayon reste celui de la **planche**, pas celui du tirage
        // (≈ 2 unités) : sinon la naine brune y serait le plus gros objet alors
        // qu'elle est l'un des plus petits. En deçà d'une naine M.
        let naine_m = catalogue().into_iter().find(|e| e.0 == "M (naine rouge)").expect("naine M").1;
        assert!(RAYON_BRUNE < naine_m, "{RAYON_BRUNE} : plus grosse qu'une naine M ({naine_m})");
        assert!(RAYON_BRUNE > naine_m * 0.4, "invisible dans sa case");
    }

    /// **Un type reconnu par son nom porte le même nom des deux côtés.**
    ///
    /// ⚠️ Deux traits de rendu se reconnaissent **au nom du type** : le filtre
    /// moléculaire de la carbonée, et la **pulsation** des trois variables. Que
    /// la vitrine écrive « Etoile carbonee » là où le tirage écrit « Etoile
    /// carbonee (C) » suffirait à ce que le jeu la peigne rouge et la galerie
    /// orange — ou à ce que Mira batte en jeu et reste figée en vitrine — sans
    /// qu'aucun test ne bronche : deux fonctions appelées avec deux chaînes
    /// différentes.
    ///
    /// Et les noms *diffèrent* déjà ailleurs à dessein (« G (jaune - Soleil) »
    /// contre « G (jaune) ») : on ne peut donc pas exiger l'égalité partout,
    /// seulement là où elle décide d'un rendu.
    #[test]
    fn les_types_reconnus_par_leur_nom_le_portent_des_deux_cotes() {
        let en_vitrine: Vec<&str> = catalogue().into_iter().map(|e| e.0).collect();
        let mut vus = 0;
        for k in 0..3000 {
            let nom = crate::etoile::ordinaire(k as f32 / 3000.0, 0.5).4;
            let filtre = crate::etoile::filtre_teinte(nom) != Vec3::ONE;
            let pulse = crate::etoile::pulsation(nom) != (0.0, 0.0);
            if !filtre && !pulse {
                continue;
            }
            vus += 1;
            assert!(
                en_vitrine.contains(&nom),
                "« {nom} » tient un trait de son nom (filtre {filtre}, pulsation {pulse}) mais ne \
                 figure pas sous ce nom en galerie : {en_vitrine:?}"
            );
        }
        // Un compte, et non « au moins un » : les quatre types concernés sont la
        // carbonée et les trois variables. Si ce nombre baisse, un type a perdu
        // son trait ; s'il monte, un nouveau est arrivé sans passer ici.
        let distincts: std::collections::BTreeSet<&str> = (0..3000)
            .map(|k| crate::etoile::ordinaire(k as f32 / 3000.0, 0.5).4)
            .filter(|n| {
                crate::etoile::filtre_teinte(n) != Vec3::ONE
                    || crate::etoile::pulsation(n) != (0.0, 0.0)
            })
            .collect();
        assert_eq!(distincts.len(), 4, "types reconnus par leur nom : {distincts:?}");
        assert!(vus > 0);
    }

    /// **La planche-contact montre le produit complet** : chaque appétit sous
    /// chaque cible déclinée, et dans le même ordre.
    ///
    /// ⚠️ C'est sa raison d'être. Une cible à qui il manquerait un appétit se
    /// comparerait mal — on croirait juger deux aspects là où l'un des deux
    /// mange simplement autrement.
    #[test]
    fn la_planche_contact_montre_chaque_appetit_sous_chaque_cible() {
        let cat = Accretion::catalogue();
        for cible in ["chaud", "Interstellar"] {
            let noms: Vec<&str> =
                cat.iter().filter(|a| a.cible == cible).map(|a| a.nom).collect();
            assert_eq!(noms, ["endormi", "actif", "vorace"], "cible « {cible} »");
        }
        // La braise est une variante de **teinte**, demandée une seule fois :
        // un quatrième aspect à regarder, pas un quatrième appétit.
        assert_eq!(cat.iter().filter(|a| a.cible == "braise").count(), 1);
        // Et la ceinture suit l'appétit, jamais la teinte : ce sont des
        // cailloux, et `config_ceinture` ignore la cible.
        for a in &cat {
            assert_eq!(a.ceinture.is_some(), a.nom == "vorace", "{}", a.libelle());
        }
    }
}
