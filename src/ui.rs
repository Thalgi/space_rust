use macroquad::prelude::*;

/// Panneau Minitel (fond bleu nuit, cadre cyan, barre de titre).
pub fn minitel_panel(r: Rect, titre: &str) {
    draw_rectangle(r.x, r.y, r.w, r.h, Color::new(0.02, 0.03, 0.12, 0.97));
    draw_rectangle_lines(r.x, r.y, r.w, r.h, 2.0, Color::new(0.0, 0.85, 0.85, 1.0));
    draw_rectangle(r.x, r.y, r.w, 26.0, Color::new(0.0, 0.6, 0.6, 1.0));
    crate::police::texte(titre, r.x + 10.0, r.y + 18.0, 20.0, BLACK);
}

/// Projette une direction du monde dans le plan écran de la boussole.
///
/// Sortie du dessin pour être **testable** : le `-` devant `up` est l'inversion
/// d'axe entre le monde (Y monte) et l'écran (Y descend), et c'est l'erreur
/// classique de ce genre de widget. Sans test, elle ne se voit qu'à l'œil — et un
/// repère faux est pire que pas de repère du tout.
fn projeter_axe(d: Vec3, right: Vec3, up: Vec3, rayon: f32) -> Vec2 {
    vec2(d.dot(right), -d.dot(up)) * rayon
}

/// **Boussole d'axes** : le repère XYZ du monde, projeté dans un coin de l'écran,
/// pour savoir d'un coup d'œil où l'on regarde.
///
/// `right`/`up`/`forward` sont la base de la caméra (`crate::camera::base_orbite`,
/// via `CameraInfo`) — les mêmes vecteurs qui servent à l'éclairage. La boussole ne
/// peut donc pas se désynchroniser de la vue.
///
/// **Projection orthographique et non perspective**, volontairement : un gizmo
/// d'orientation montre des *directions*, pas des positions. Une perspective y
/// ajouterait une déformation qui n'apprend rien et fausserait la lecture des
/// angles entre axes.
pub fn boussole_axes(centre: Vec2, rayon: f32, right: Vec3, up: Vec3, forward: Vec3) {
    // Fond : sans lui les axes se perdent dans le fond étoilé.
    let bord = rayon + 14.0;
    draw_circle(centre.x, centre.y, bord, Color::new(0.02, 0.03, 0.12, 0.72));
    draw_circle_lines(centre.x, centre.y, bord, 1.0, Color::new(0.0, 0.55, 0.55, 0.9));

    // Rouge/vert/bleu pour X/Y/Z : la convention de tous les logiciels 3D. En
    // inventer une autre ne ferait perdre du temps à la lecture.
    let axes = [
        (Vec3::X, "X", Color::new(1.0, 0.35, 0.35, 1.0)),
        (Vec3::Y, "Y", Color::new(0.40, 1.0, 0.48, 1.0)),
        (Vec3::Z, "Z", Color::new(0.45, 0.65, 1.0, 1.0)),
    ];

    // Tri par profondeur : l'axe le plus proche de la caméra se dessine **en
    // dernier**, donc par-dessus. Sans ça, deux axes qui se croisent à l'écran se
    // recouvrent dans un ordre arbitraire et la lecture devient ambiguë.
    let mut ordre = [0usize, 1, 2];
    ordre.sort_by(|a, b| {
        axes[*b].0.dot(forward).total_cmp(&axes[*a].0.dot(forward))
    });

    let projeter = |d: Vec3| centre + projeter_axe(d, right, up, rayon);

    for i in ordre {
        let (axe, nom, teinte) = axes[i];
        // Un axe qui **fuit** la caméra est atténué : c'est ce qui lève
        // l'ambiguïté entre +X et −X, dont les projections se superposent.
        let fuit = axe.dot(forward) > 0.0;
        let c = if fuit {
            Color::new(teinte.r * 0.42, teinte.g * 0.42, teinte.b * 0.42, 1.0)
        } else {
            teinte
        };

        // Moignon du côté négatif : donne l'orientation même quand l'axe est vu
        // presque de profil.
        let arriere = projeter(-axe * 0.34);
        draw_line(centre.x, centre.y, arriere.x, arriere.y, 1.5, Color::new(c.r, c.g, c.b, 0.4));

        let bout = projeter(axe);
        draw_line(centre.x, centre.y, bout.x, bout.y, 2.5, c);
        draw_circle(bout.x, bout.y, 3.5, c);

        // Étiquette poussée vers l'extérieur. Quand l'axe pointe droit sur la
        // caméra sa projection est quasi nulle : on décale alors vers le haut,
        // faute de direction utilisable. Un seul axe peut être dans ce cas à la
        // fois, les étiquettes ne peuvent donc pas s'empiler.
        let v = bout - centre;
        let dir = if v.length() > rayon * 0.2 { v.normalize() } else { vec2(0.0, -1.0) };
        let lab = bout + dir * 13.0;
        crate::police::texte(nom, lab.x - 5.0, lab.y + 6.0, 20.0, c);
    }
}

/// Ligne/bouton cliquable façon télétexte (surbrillance inversée au survol).
pub fn minitel_ligne(r: Rect, label: &str, souris: Vec2) {
    let survol = r.contains(souris);
    let (bg, fg) = if survol {
        (Color::new(0.0, 0.85, 0.85, 1.0), BLACK)
    } else {
        (Color::new(0.04, 0.05, 0.18, 1.0), Color::new(0.55, 1.0, 0.75, 1.0))
    };
    draw_rectangle(r.x, r.y, r.w, r.h, bg);
    draw_rectangle_lines(r.x, r.y, r.w, r.h, 1.0, Color::new(0.0, 0.7, 0.7, 1.0));
    crate::police::texte(label, r.x + 10.0, r.y + r.h * 0.5 + 6.0, 20.0, fg);
}

/// **Bascule à deux étages** : un mot au-dessus d'un filet, un mot dessous.
///
/// « ORB PLAN: ON » disait l'état dans le libellé — il fallait donc LIRE pour
/// savoir. Ici l'état est la **couleur du fond**, et le libellé ne dit plus que
/// le sujet.
///
/// Voir [`COULEUR_ACTIF`] pour le choix des deux couleurs, qui n'est pas libre.
pub fn bascule_double(r: Rect, haut: &str, bas: &str, actif: bool, souris: Vec2) {
    let survol = r.contains(souris);
    let fond = if actif { COULEUR_ACTIF } else { COULEUR_INACTIF };
    // Le survol éclaircit sans changer de camp : on doit toujours lire l'état.
    let fond = if survol {
        Color::new(
            (fond.r + 0.14).min(1.0),
            (fond.g + 0.14).min(1.0),
            (fond.b + 0.14).min(1.0),
            1.0,
        )
    } else {
        fond
    };
    let texte = if actif { TEXTE_ACTIF } else { TEXTE_INACTIF };

    draw_rectangle(r.x, r.y, r.w, r.h, fond);
    draw_rectangle_lines(r.x, r.y, r.w, r.h, 1.0, texte);

    // Deux cases égales, séparées par le filet : c'est le « double carré ».
    let mi = r.y + r.h * 0.5;
    draw_line(r.x + 5.0, mi, r.x + r.w - 5.0, mi, 1.0, texte);

    let taille = taille_tenant_dans(&[haut, bas], r.w - 8.0);
    for (mot, cy) in [(haut, r.y + r.h * 0.25), (bas, r.y + r.h * 0.75)] {
        let l = crate::police::mesure(mot, taille as u16);
        crate::police::texte(mot, r.x + (r.w - l) * 0.5, cy + taille * 0.36, taille, texte);
    }
}

/// La plus grande taille de police, parmi les tailles préchargées, où **tous**
/// les mots tiennent dans `largeur`.
///
/// Mesurée avec la vraie police et non estimée : « HABITABLE » et « PLANET »
/// n'ont ni la même longueur ni la même chasse, et c'est le plus large qui
/// commande — sinon un seul libellé déborde et lui seul.
fn taille_tenant_dans(mots: &[&str], largeur: f32) -> f32 {
    taille_tenant_dans_avec(mots, largeur, crate::police::mesure)
}

/// Le calcul seul, sa règle de mesure passée en argument.
///
/// Séparé pour être **testable sans fenêtre** : `police::mesure` lit la police
/// chargée, donc exige un contexte graphique. La logique — descendre tant que le
/// plus large déborde, sans passer sous le plancher lisible — n'en a pas besoin.
fn taille_tenant_dans_avec(mots: &[&str], largeur: f32, mesure: impl Fn(&str, u16) -> f32) -> f32 {
    let mut t = 16.0_f32;
    while t > crate::police::TAILLE_MIN && mots.iter().any(|m| mesure(m, t as u16) > largeur) {
        t -= 1.0;
    }
    t
}

/// **Vert néon « daltonien »** : bascule active.
///
/// Rouge contre vert est le pire couple pour une vision deutéranope ou
/// protanope — les deux teintes se confondent. Ce qui les sépare quand la
/// TEINTE ne le fait pas, c'est la **luminance** : un rouge sombre et un vert
/// néon tiennent à ~8 fois d'écart, lisible même sans percevoir aucune des deux
/// couleurs. Le vert est en outre tiré vers le jaune, où un protanope le voit
/// encore clair.
///
/// L'écart est **tenu par un test** (`les_deux_etats_se_distinguent_sans_la_teinte`) :
/// une retouche esthétique qui rapprocherait les luminances casserait
/// l'accessibilité sans que rien ne le dise à l'œil d'un voyant trichromate.
pub const COULEUR_ACTIF: Color = Color::new(0.44, 1.0, 0.28, 1.0);
/// Rouge sombre : bascule inactive. Voir [`COULEUR_ACTIF`].
pub const COULEUR_INACTIF: Color = Color::new(0.30, 0.045, 0.07, 1.0);
const TEXTE_ACTIF: Color = Color::new(0.03, 0.10, 0.02, 1.0);
const TEXTE_INACTIF: Color = Color::new(0.92, 0.66, 0.62, 1.0);

#[cfg(test)]
mod tests {
    use super::*;

    fn lum(c: Color) -> f32 {
        crate::palette::luminance(vec3(c.r, c.g, c.b))
    }

    /// **Les deux états doivent se distinguer SANS la teinte.**
    ///
    /// Rouge/vert est le couple que ne séparent ni un deutéranope ni un
    /// protanope — soit ~8 % des hommes. Le seul canal qui leur reste est la
    /// luminance. On exige donc un rapport franc, du même ordre que ce que le
    /// WCAG demande entre un texte et son fond.
    #[test]
    fn les_deux_etats_se_distinguent_sans_la_teinte() {
        let (a, i) = (lum(COULEUR_ACTIF), lum(COULEUR_INACTIF));
        let rapport = (a + 0.05) / (i + 0.05);
        assert!(
            rapport >= 4.5,
            "actif {a:.3} contre inactif {i:.3} : rapport de contraste {rapport:.2},              insuffisant pour qui ne distingue pas le rouge du vert"
        );
    }

    /// Et le texte doit rester lisible **sur les deux fonds**.
    #[test]
    fn le_texte_contraste_avec_son_fond() {
        for (nom, fond, texte) in [
            ("actif", COULEUR_ACTIF, TEXTE_ACTIF),
            ("inactif", COULEUR_INACTIF, TEXTE_INACTIF),
        ] {
            let (f, t) = (lum(fond), lum(texte));
            let (haut, bas) = if f > t { (f, t) } else { (t, f) };
            let rapport = (haut + 0.05) / (bas + 0.05);
            assert!(rapport >= 4.5, "{nom} : contraste texte/fond de {rapport:.2}");
        }
    }

    /// La taille de police descend jusqu'à ce que le PLUS LARGE des mots tienne.
    ///
    /// Règle de mesure factice — une chasse constante par caractère : le test
    /// porte sur la **descente**, pas sur les métriques de la police.
    #[test]
    fn la_taille_sadapte_au_mot_le_plus_large() {
        let chasse = |m: &str, t: u16| m.chars().count() as f32 * t as f32 * 0.6;
        let taille = |mots: &[&str], l: f32| taille_tenant_dans_avec(mots, l, chasse);

        // Large : rien à réduire.
        assert_eq!(taille(&["ZONE", "HABITABLE"], 400.0), 16.0);
        // Étroit : la taille tombe, et reste au-dessus du plancher lisible.
        let t = taille(&["ZONE", "HABITABLE"], 46.0);
        assert!(t < 16.0, "la taille n'a pas été réduite ({t})");
        assert!(t >= crate::police::TAILLE_MIN, "sous le plancher lisible ({t})");
        // C'est bien le mot le plus large qui commande : ajouter un mot court
        // ne change rien.
        assert_eq!(taille(&["ZONE", "HABITABLE"], 60.0), taille(&["HABITABLE"], 60.0));
        // Et un cas impossible s'arrête au plancher au lieu de boucler.
        assert_eq!(taille(&["HABITABLE"], 1.0), crate::police::TAILLE_MIN);
    }

    // Caméra au repos (yaw 0, pitch 0) : on regarde vers −Z. La boussole doit
    // alors montrer **X à droite** et **Y vers le haut**.
    //
    // On part de `camera::base_orbite` — celle-là même que `construire` utilise —
    // et non de vecteurs choisis à la main : c'est ce qui fait que le test vérifie
    // aussi l'accord entre la caméra et la boussole, et pas seulement la
    // projection prise isolément.
    #[test]
    fn la_boussole_saccorde_a_lorientation_de_la_camera() {
        let (right, up, forward) = crate::camera::base_orbite(0.0, 0.0);
        let r = 10.0;

        let px = projeter_axe(Vec3::X, right, up, r);
        assert!(px.x > 9.9 && px.y.abs() < 1e-4, "X devrait partir à droite, projeté en {px:?}");

        // Y monte à l'écran, donc **y écran négatif** : c'est l'inversion à ne pas
        // perdre.
        let py = projeter_axe(Vec3::Y, right, up, r);
        assert!(py.y < -9.9 && py.x.abs() < 1e-4, "Y devrait monter, projeté en {py:?}");

        // +Z pointe vers la caméra : sa projection s'annule, et il ne doit pas
        // être atténué (c'est l'atténuation qui distingue +Z de −Z).
        let pz = projeter_axe(Vec3::Z, right, up, r);
        assert!(pz.length() < 1e-4, "Z devrait se projeter au centre, {pz:?}");
        assert!(Vec3::Z.dot(forward) < 0.0, "+Z devrait venir vers la caméra");
    }

    // Un quart de tour de la caméra doit faire tourner la boussole d'autant :
    // vue de +X, l'axe X vient vers l'œil et Z part sur le côté.
    #[test]
    fn la_boussole_suit_un_quart_de_tour() {
        let (right, up, forward) = crate::camera::base_orbite(std::f32::consts::FRAC_PI_2, 0.0);

        let px = projeter_axe(Vec3::X, right, up, 10.0);
        assert!(px.length() < 1e-3, "X devrait être de face, {px:?}");
        assert!(Vec3::X.dot(forward) < 0.0, "+X devrait venir vers la caméra");

        let pz = projeter_axe(Vec3::Z, right, up, 10.0);
        assert!(pz.x.abs() > 9.9, "Z devrait passer sur le côté, {pz:?}");
        // Y reste vertical quel que soit le lacet.
        let py = projeter_axe(Vec3::Y, right, up, 10.0);
        assert!(py.y < -9.9, "Y devrait rester vers le haut, {py:?}");
    }
}
