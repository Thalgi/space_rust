mod dessin;

use crate::genese::PresetSauve;
use macroquad::prelude::*;

/// Action que `main` doit appliquer (charger un système, etc.).
#[derive(Clone, Copy)]
pub enum ActionMenu {
    Solaire,
    TauCeti,
    Avatar,
    AlphaCentauri,
    Proxima,
    Binaire,
    Trinaire,
    Quadruple,
    Charger(usize),
    Aleatoire,
    /// Un système aléatoire autour de ce type d'étoile (menu V2, « choisir astre »).
    Astre(&'static str),
    Quitter,
    Retour,
}

/// **Ce que fait une ligne du menu.** Les lignes sont des données, pas des
/// index : la V1, la V2 et la liste des astres n'ont pas les mêmes, et une
/// chaîne de `if i == 3` par variante aurait fini par se désaccorder de ce
/// qu'affiche `dessiner`.
#[derive(Clone, Copy)]
enum Choix {
    Action(ActionMenu),
    /// Saisie du nom d'un système à sauvegarder (V1 seulement).
    Saisie,
    /// Ouvre la liste des types d'étoiles (V2).
    ListeAstres,
    /// Revient de la liste au menu.
    FermerListe,
}

/// Menu Minitel + boutons d'affichage. Gère son propre état (ouvert, saisie de nom,
/// toggles), et renvoie une action quand `main` doit charger/quitter.
pub struct Menu {
    pub ouvert: bool,
    pub saisie: bool,
    pub nom: String,
    pub orbites: bool,         // orbites des planètes
    pub orbites_etoiles: bool, // orbites des étoiles (systèmes multiples)
    pub orbites_lunes: bool,   // orbites des lunes, autour de leur planète
    pub zone: bool,
    pub orbites_cometes: bool, // trajectoires des comètes (2026-10-20)
    /// **Menu de la Skymap V2** : sans sauvegarde (mise de côté le
    /// 2026-10-19), quatre presets, et la liste des astres pour la mise au
    /// point. La V1 garde son menu d'origine.
    v2: bool,
    /// La liste des astres est ouverte à la place du menu.
    liste_astres: bool,
}

impl Menu {
    pub fn new() -> Self {
        Self {
            ouvert: false,
            saisie: false,
            nom: String::new(),
            orbites: true,
            orbites_etoiles: true,
            orbites_lunes: false, // nombreuses : encombrantes par défaut
            zone: true,
            orbites_cometes: true,
            v2: false,
            liste_astres: false,
        }
    }

    /// Le menu de la Skymap V2 (voir le champ `v2`).
    pub fn v2() -> Self {
        Self { v2: true, ..Self::new() }
    }

    /// **Les lignes affichées**, dans l'ordre : libellé et effet.
    fn entrees(&self, presets: &[PresetSauve]) -> Vec<(String, Choix)> {
        use ActionMenu::*;
        let a = |s: &str, x: ActionMenu| (s.to_string(), Choix::Action(x));
        if self.liste_astres {
            // Les types du tirage, **balayés** dans ses tables : un type ajouté
            // demain apparaît ici sans qu'on y touche.
            let mut v: Vec<(String, Choix)> = crate::etoile::types_du_tirage()
                .into_iter()
                .map(|n| (n.to_uppercase(), Choix::Action(Astre(n))))
                .collect();
            v.push(("< RETOUR".into(), Choix::FermerListe));
            return v;
        }
        if self.v2 {
            // Sauvegarde des systèmes mise de côté le 2026-10-19 : le code de
            // persistance reste (`genese::persistance`, la V1 s'en sert).
            return vec![
                a("SYSTEME SOLAIRE", Solaire),
                a("TAU CETI", TauCeti),
                a("ALPHA CENTAURI A+B", AlphaCentauri),
                a("PROXIMA CENTAURI", Proxima),
                ("CHOISIR ASTRE >".into(), Choix::ListeAstres),
                a("+ ALEATOIRE", Aleatoire),
                // Quitte sans rien écrire : la sauvegarde est mise de côté.
                a("QUITTER", Quitter),
            ];
        }
        let mut v = vec![
            a("SYSTEME SOLAIRE", Solaire),
            a("TAU CETI", TauCeti),
            a("AVATAR (PANDORA)", Avatar),
            a("ALPHA CENTAURI A+B", AlphaCentauri),
            a("PROXIMA CENTAURI", Proxima),
            a("BINAIRE A+B", Binaire),
            a("TRINAIRE (A-B)+C", Trinaire),
            a("QUADRUPLE 2+2", Quadruple),
        ];
        v.extend(presets.iter().enumerate().map(|(i, p)| (p.nom.clone(), Choix::Action(Charger(i)))));
        v.push(a("+ ALEATOIRE", Aleatoire));
        v.push(("SAUVER CE SYSTEME".into(), Choix::Saisie));
        v.push(a("QUITTER (SAUVE)", Quitter));
        v
    }

    /// Colonnes du panneau : deux pour la liste des astres, qui en compte
    /// vingt-quatre — sur une seule, elle sortirait d'un écran de 700 px.
    fn colonnes(&self) -> usize {
        if self.liste_astres { 2 } else { 1 }
    }

    // ===== Bascules d'outillage : **la bande du bas** =====
    //
    // Elles etaient a y = 34, c'est-a-dire au milieu de la barre de ressources
    // depuis que la vue systeme porte l'interface de jeu. Le partage est
    // desormais net : le haut au jeu, le bas a l'atelier.
    //
    // `outils` est la bande basse, fournie par l'ecran (`bandeau::strip_outils`)
    // plutot que recalculee ici : deux geometries a tenir d'accord finiraient
    // par se recouvrir, ce qui est exactement le defaut qu'on corrige.

    /// Largeur des bascules, dans l'ordre ou elles se suivent.
    ///
    /// Les cinq sont les calques de lecture (RING/PLANET, RING/STAR,
    /// RING/MOONS, ZONE/HABITABLE, RING/COMET) : des « doubles carres », un
    /// mot par etage. RING/COMET est arrive le 2026-10-20, en dernier pour ne
    /// pas deplacer les quatre autres sous le doigt.
    const LARGEURS: [f32; 5] = [76.0, 76.0, 76.0, 88.0, 76.0];
    /// Hauteur d'une bascule a une ligne (MENU, RETOUR).
    const HAUT_BTN: f32 = 26.0;
    /// Hauteur d'un double carre : deux etages plus le filet.
    ///
    /// La bande d'outils fait `bandeau::BAS_OUTILS` = 56 px ; 44 plus 4 de marge
    /// haute laissent 8 px sous le bouton, de quoi ne pas coller au bord.
    const HAUT_DOUBLE: f32 = 44.0;

    /// Rectangle de la bascule `i` dans la bande d'outils.
    ///
    /// Les quatre se **resserrent** si la bande est trop courte : a 640 px de
    /// large, la colonne d'astres prise, il reste 560 px pour 587 px de
    /// boutons. Sans ce facteur, la derniere sortait de l'ecran.
    fn btn(outils: Rect, i: usize) -> Rect {
        const JEU: f32 = 6.0;
        let bande = Self::bande_bascules(outils);
        let total: f32 = Self::LARGEURS.iter().sum::<f32>() + JEU * (Self::LARGEURS.len() - 1) as f32;
        let k = if total > bande.w && bande.w > 0.0 { bande.w / total } else { 1.0 };
        let x = bande.x + Self::LARGEURS.iter().take(i).map(|l| (l + JEU) * k).sum::<f32>();
        Rect::new(x, bande.y + 4.0, Self::LARGEURS[i] * k, Self::HAUT_DOUBLE)
    }

    /// Les bascules, pour les tests de mise en page.
    #[cfg(test)]
    fn bascules(outils: Rect) -> [Rect; 5] {
        std::array::from_fn(|i| Self::btn(outils, i))
    }

    /// Largeur du couple MENU / RETOUR, reserve a **droite** de la bande.
    const LARGEUR_MENU: f32 = 112.0;

    /// Bouton MENU : bout droit de la bande d'outils.
    ///
    /// Il etait en haut a droite, ou il entrait en collision avec la barre de
    /// ressources des que l'ecran retrecissait (a 640 px la barre va jusqu'a
    /// x = 632, le bouton commencait a 518). Tout l'outillage est desormais
    /// dans la meme bande.
    fn bouton(outils: Rect) -> Rect {
        Rect::new(outils.x + outils.w - Self::LARGEUR_MENU, outils.y + 4.0, Self::LARGEUR_MENU, Self::HAUT_BTN)
    }
    /// RETOUR, a gauche de MENU. N'apparait que camera focalisee.
    fn retour(outils: Rect) -> Rect {
        let b = Self::bouton(outils);
        Rect::new(b.x - Self::LARGEUR_MENU - 6.0, b.y, Self::LARGEUR_MENU, Self::HAUT_BTN)
    }

    /// Place laissee aux bascules : la bande **moins** le couple de droite.
    fn bande_bascules(outils: Rect) -> Rect {
        let pris = Self::LARGEUR_MENU * 2.0 + 12.0;
        Rect::new(outils.x, outils.y, (outils.w - pris).max(40.0), outils.h)
    }
    /// Panneau modal, centre, **sous** le bandeau de ressources. `n` : nombre
    /// de lignes.
    fn menu_rect(&self, n: usize, haut: f32) -> Rect {
        let cols = self.colonnes();
        let rangs = n.div_ceil(cols);
        let l = 300.0 * cols as f32 - 40.0 * (cols - 1) as f32;
        let h = 40.0 + rangs as f32 * 30.0 + if self.saisie { 40.0 } else { 0.0 };
        Rect::new(screen_width() * 0.5 - l * 0.5, haut, l, h)
    }
    /// Ligne `i` sur `n`, rangées **en colonnes** (on lit de haut en bas).
    fn entry_rect(&self, i: usize, n: usize, haut: f32) -> Rect {
        let mr = self.menu_rect(n, haut);
        let cols = self.colonnes();
        let rangs = n.div_ceil(cols).max(1);
        let lc = (mr.w - 24.0) / cols as f32;
        let (col, rang) = (i / rangs, i % rangs);
        let jeu = if cols > 1 { 6.0 } else { 0.0 };
        Rect::new(mr.x + 12.0 + col as f32 * lc, mr.y + 34.0 + rang as f32 * 30.0, lc - jeu, 26.0)
    }

    /// Gestion des clics. Renvoie (souris_sur_ui, action éventuelle).
    ///
    /// `outils` est la bande basse reservee a l'outillage, `haut` le premier `y`
    /// libre sous la barre de ressources. Les deux viennent de l'ecran : le menu
    /// ne connait pas la mise en page du jeu, il s'y range.
    pub fn input(
        &mut self,
        m: Vec2,
        clic: bool,
        presets: &[PresetSauve],
        focus: bool,
        outils: Rect,
        haut: f32,
    ) -> (bool, Option<ActionMenu>) {
        let entrees = self.entrees(presets);
        let n = entrees.len();
        let (bo, boe, bl, bz, bc, bn, re) = (
            Self::btn(outils, 0),
            Self::btn(outils, 1),
            Self::btn(outils, 2),
            Self::btn(outils, 3),
            Self::btn(outils, 4),
            Self::bouton(outils),
            Self::retour(outils),
        );
        let mr = self.menu_rect(n, haut);
        let mut action = None;
        if clic {
            if bo.contains(m) {
                self.orbites = !self.orbites;
            } else if boe.contains(m) {
                self.orbites_etoiles = !self.orbites_etoiles;
            } else if bl.contains(m) {
                self.orbites_lunes = !self.orbites_lunes;
            } else if bz.contains(m) {
                self.zone = !self.zone;
            } else if bc.contains(m) {
                self.orbites_cometes = !self.orbites_cometes;
            } else if bn.contains(m) {
                self.ouvert = !self.ouvert;
                // Rouvert, le menu repart de sa page principale.
                self.liste_astres = false;
            } else if focus && re.contains(m) {
                action = Some(ActionMenu::Retour);
            } else if self.ouvert && !self.saisie {
                if let Some(i) = (0..n).find(|i| self.entry_rect(*i, n, haut).contains(m)) {
                    action = self.activer(entrees[i].1);
                }
            }
        }
        // ⚠️ `bl` (RING/MOONS) manquait à cette liste : un clic dessus
        // passait aussi à la caméra. Toutes les bascules y sont désormais.
        let sur_ui = bo.contains(m)
            || boe.contains(m)
            || bl.contains(m)
            || bz.contains(m)
            || bc.contains(m)
            || bn.contains(m)
            || (focus && re.contains(m))
            || (self.ouvert && mr.contains(m));
        (sur_ui, action)
    }

    fn activer(&mut self, c: Choix) -> Option<ActionMenu> {
        match c {
            Choix::Action(a) => {
                self.ouvert = false;
                self.liste_astres = false;
                Some(a)
            }
            Choix::Saisie => {
                self.saisie = true;
                self.nom.clear();
                None
            }
            Choix::ListeAstres => {
                self.liste_astres = true;
                None
            }
            Choix::FermerListe => {
                self.liste_astres = false;
                None
            }
        }
    }
}

#[cfg(test)]
mod tests_entrees {
    use super::*;

    fn libelles(m: &Menu) -> Vec<String> {
        m.entrees(&[]).into_iter().map(|(l, _)| l).collect()
    }

    /// **Le menu V2 n'a plus rien de la sauvegarde**, garde ses quatre presets
    /// et ouvre la liste des astres. Demandé le 2026-10-19.
    #[test]
    fn le_menu_v2_sans_sauvegarde_avec_la_liste_des_astres() {
        let m = Menu::v2();
        let l = libelles(&m);
        for absent in ["SAUVER CE SYSTEME", "QUITTER (SAUVE)", "AVATAR (PANDORA)", "BINAIRE A+B"] {
            assert!(!l.iter().any(|x| x == absent), "« {absent} » encore au menu V2 : {l:?}");
        }
        for present in ["SYSTEME SOLAIRE", "TAU CETI", "ALPHA CENTAURI A+B", "PROXIMA CENTAURI", "CHOISIR ASTRE >", "QUITTER"] {
            assert!(l.iter().any(|x| x == present), "« {present} » manque au menu V2 : {l:?}");
        }
        // Et un système sauvegardé n'y apparaît pas, même s'il en existe.
        let p = [PresetSauve { nom: "MON SYSTEME".into(), graine: 7 }];
        assert!(!m.entrees(&p).iter().any(|(x, _)| x == "MON SYSTEME"));
        // La V1, elle, n'a pas bougé.
        assert!(libelles(&Menu::new()).iter().any(|x| x == "SAUVER CE SYSTEME"));
    }

    /// **La liste des astres propose chaque type du tirage**, chacun menant à
    /// un système autour de lui, plus un retour.
    #[test]
    fn la_liste_des_astres_couvre_chaque_type_du_tirage() {
        let mut m = Menu::v2();
        let i = m.entrees(&[]).iter().position(|(l, _)| l == "CHOISIR ASTRE >").unwrap();
        let choix = m.entrees(&[])[i].1;
        assert!(m.activer(choix).is_none(), "ouvrir la liste ne doit rien charger");
        let e = m.entrees(&[]);
        let types = crate::etoile::types_du_tirage();
        for t in &types {
            let trouve = e.iter().any(|(_, c)| matches!(c, Choix::Action(ActionMenu::Astre(n)) if n == t));
            assert!(trouve, "« {t} » absent de la liste des astres");
        }
        assert_eq!(e.len(), types.len() + 1, "un type de trop ou un retour manquant");
        // Le retour ramène au menu, sans rien charger.
        let retour = e.last().unwrap().1;
        assert!(m.activer(retour).is_none());
        assert!(libelles(&m).iter().any(|x| x == "CHOISIR ASTRE >"));
    }
}

#[cfg(test)]
mod tests_mise_en_page {
    use super::*;

    fn strip(l: f32, h: f32) -> Rect {
        crate::ecran::bandeau::strip_outils(vec2(l, h))
    }

    // **Les quatre bascules tiennent dans la bande**, a toutes les largeurs.
    // C'etait le defaut signale : posees a une position fixe, elles tombaient
    // au milieu de la barre de ressources, et debordaient sur un ecran etroit.
    #[test]
    fn les_bascules_tiennent_dans_la_bande_doutils() {
        for (l, h) in [(640.0_f32, 480.0_f32), (1000.0, 700.0), (1440.0, 900.0), (1920.0, 1080.0)] {
            let o = strip(l, h);
            let b = Menu::bascules(o);
            for (i, r) in b.iter().enumerate() {
                assert!(r.x >= o.x - 1e-3, "{l}x{h} : bascule {i} a gauche de la bande");
                assert!(r.x + r.w <= o.x + o.w + 1e-3, "{l}x{h} : bascule {i} deborde ({})", r.x + r.w);
                assert!(r.y >= o.y - 1e-3 && r.y + r.h <= o.y + o.h + 1e-3, "{l}x{h} : bascule {i} hors bande");
                assert!(r.w > 20.0, "{l}x{h} : bascule {i} ecrasee a {} px", r.w);
            }
            // Et elles ne se chevauchent pas entre elles.
            for i in 0..b.len() {
                for j in i + 1..b.len() {
                    assert!(
                        b[i].x + b[i].w <= b[j].x + 1e-3 || b[j].x + b[j].w <= b[i].x + 1e-3,
                        "{l}x{h} : bascules {i} et {j} se chevauchent"
                    );
                }
            }
        }
    }

    /// **Les calques sont des « doubles carres »** : deux etages, donc a peu pres
    /// aussi hauts que larges. Une bascule etiree en bandeau ne montrerait plus
    /// deux cases mais une ligne coupee.
    #[test]
    fn les_calques_sont_a_peu_pres_carres() {
        for (l, h) in [(640.0_f32, 480.0_f32), (1000.0, 700.0), (1920.0, 1080.0)] {
            let b = Menu::bascules(strip(l, h));
            for (i, r) in b.iter().enumerate() {
                let rapport = r.w / r.h;
                assert!(
                    (0.7..=2.2).contains(&rapport),
                    "{l}x{h} : calque {i} de rapport {rapport:.2} ({}x{})",
                    r.w,
                    r.h
                );
            }
            // Tous alignes sur le meme bord superieur.
            for (i, r) in b.iter().enumerate() {
                assert!((r.y - b[0].y).abs() < 1e-3, "bascule {i} desalignee");
            }
        }
    }

    // MENU et RETOUR sont **dans la bande** eux aussi, et ne chevauchent ni les
    // bascules ni la barre de ressources. Ils etaient en haut a droite, ou la
    // barre les rattrapait des que l'ecran retrecissait.
    #[test]
    fn menu_et_retour_tiennent_dans_la_bande_sans_rien_recouvrir() {
        for (l, h) in [(640.0_f32, 480.0_f32), (1000.0, 700.0), (1920.0, 1080.0)] {
            let e = vec2(l, h);
            let o = strip(l, h);
            let barre = crate::ecran::bandeau::rectangle(e);
            for (nom, r) in [("MENU", Menu::bouton(o)), ("RETOUR", Menu::retour(o))] {
                assert!(r.x >= o.x - 1e-3 && r.x + r.w <= o.x + o.w + 1e-3, "{l}x{h} : {nom} hors bande");
                let sur_barre = r.x < barre.x + barre.w && barre.x < r.x + r.w
                    && r.y < barre.y + barre.h && barre.y < r.y + r.h;
                assert!(!sur_barre, "{l}x{h} : {nom} recouvre la barre de ressources");
                for (i, b) in Menu::bascules(o).iter().enumerate() {
                    let sur_bascule = r.x < b.x + b.w && b.x < r.x + r.w
                        && r.y < b.y + b.h && b.y < r.y + r.h;
                    assert!(!sur_bascule, "{l}x{h} : {nom} recouvre la bascule {i}");
                }
            }
            // RETOUR est bien a gauche de MENU, sans les superposer.
            assert!(Menu::retour(o).x + Menu::retour(o).w <= Menu::bouton(o).x + 1e-3);
        }
    }

    // Les bascules ne remontent **jamais** dans la barre de ressources : c'est
    // exactement le defaut corrige.
    #[test]
    fn les_bascules_ne_touchent_pas_la_barre_de_ressources() {
        for (l, h) in [(640.0_f32, 480.0_f32), (1000.0, 700.0), (1920.0, 1080.0)] {
            let e = vec2(l, h);
            let barre = crate::ecran::bandeau::rectangle(e);
            for (i, r) in Menu::bascules(strip(l, h)).iter().enumerate() {
                let chevauche = r.x < barre.x + barre.w && barre.x < r.x + r.w
                    && r.y < barre.y + barre.h && barre.y < r.y + r.h;
                assert!(!chevauche, "{l}x{h} : la bascule {i} recouvre la barre de ressources");
            }
        }
    }
}
