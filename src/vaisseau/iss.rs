use super::pieces::{module, paire_ailes, pale_solaire, radiateur, treillis};
use super::{cylindre, parabole};
use macroquad::prelude::*;

/// Station Spatiale Internationale, low-poly d'après photo.
///
/// Repère : la poutre intégrée (ITS) court le long de **X**, les modules
/// pressurisés s'enfilent le long de **Z** (axe de vol), les 4 paires d'ailes
/// solaires se déploient en ±Z aux extrémités de la poutre (avec un vrai espace
/// entre paire intérieure et extérieure), les radiateurs blancs rayonnent près
/// du centre, et le **segment russe** est suspendu sous le centre (−Y) avec ses
/// panneaux bleus et un vaisseau amarré.
///
/// La géométrie répétée (poutre, modules, ailes, radiateurs) vient des briques
/// factorisées de [`super::pieces`], socle de la génération procédurale à venir.
pub fn dessiner_iss() {
    // Le segment russe descend loin sous la poutre : on remonte l'ensemble pour
    // le centrer dans le cadre (la caméra vise l'origine).
    unsafe {
        get_internal_gl()
            .quad_gl
            .push_model_matrix(Mat4::from_translation(vec3(0.0, 0.45, 0.0)));
    }
    dessiner_iss_interne();
    unsafe {
        get_internal_gl().quad_gl.pop_model_matrix();
    }
}

fn dessiner_iss_interne() {
    let blanc = Color::new(0.88, 0.88, 0.86, 1.0);
    let metal = Color::new(0.66, 0.68, 0.72, 1.0);
    let sombre = Color::new(0.28, 0.28, 0.31, 1.0);
    let or = Color::new(0.72, 0.60, 0.25, 1.0); // segment russe (feuille)
    let ambre = Color::new(0.60, 0.42, 0.15, 1.0); // cellules US
    let bleu = Color::new(0.13, 0.22, 0.45, 1.0); // panneaux russes
    let radia = Color::new(0.85, 0.86, 0.90, 1.0); // radiateurs blancs

    // ------------------------------------------------------------------
    // Poutre intégrée (ITS) le long de X.
    // ------------------------------------------------------------------
    treillis(vec3(-2.75, 0.0, 0.0), vec3(2.75, 0.0, 0.0), 0.13, metal, sombre);

    // ------------------------------------------------------------------
    // 4 paires d'ailes solaires : 2 par côté (intérieure + extérieure), avec un
    // espace net entre les paires et le joint rotatif (SARJ) à la jonction.
    // ------------------------------------------------------------------
    for &xm in &[-2.5_f32, -1.35, 1.35, 2.5] {
        paire_ailes(
            vec3(xm, 0.0, 0.0),
            Vec3::Z,       // déploiement avant/arrière
            Vec3::X,       // largeur le long de la poutre
            0.22,          // espace central entre les deux pales
            1.55,          // longueur des pales
            0.58,          // largeur des pales
            9,             // nervures de cellules
            ambre,
            metal,
            sombre,
        );
        // Petit radiateur photovoltaïque (PVR) blanc au pied de chaque paire.
        radiateur(
            vec3(xm, 0.22, 0.0),
            Vec3::Y,
            Vec3::Z,
            0.5,
            0.7,
            3,
            radia,
            sombre,
        );
    }
    // Joints rotatifs alpha (SARJ) entre paire intérieure et extérieure.
    for &xj in &[-1.95_f32, 1.95] {
        cylindre(vec3(xj - 0.12, 0.0, 0.0), vec3(xj + 0.12, 0.0, 0.0), 0.16, metal);
    }

    // ------------------------------------------------------------------
    // Radiateurs thermiques centraux (ATCS) : trois grands panneaux blancs.
    // ------------------------------------------------------------------
    for (i, &xr) in [-0.55_f32, 0.0, 0.55].iter().enumerate() {
        let _ = i;
        radiateur(vec3(xr, 0.16, 0.15), Vec3::Y, Vec3::Z, 0.95, 0.5, 5, radia, sombre);
    }

    // ------------------------------------------------------------------
    // Segment pressurisé US : enfilade de modules le long de Z (axe de vol).
    // ------------------------------------------------------------------
    module(vec3(0.0, 0.0, -0.35), Vec3::Z, 0.5, 0.26, blanc, sombre); // Unity
    module(vec3(0.0, 0.0, 0.25), Vec3::Z, 0.7, 0.30, blanc, sombre); // Destiny
    draw_sphere(vec3(0.0, 0.0, 0.68), 0.24, None, metal); // Harmony (nœud)
    module(vec3(0.0, 0.0, 1.0), Vec3::Z, 0.32, 0.20, blanc, sombre); // PMA
    draw_sphere(vec3(0.0, 0.0, 1.28), 0.16, None, metal); // vaisseau amarré (Dragon)

    // Modules radiaux sur Harmony : Columbus (+X), Kibo/JEM (−X, avec module
    // logistique et palette exposée).
    module(vec3(0.5, 0.0, 0.68), Vec3::X, 0.7, 0.21, blanc, sombre); // Columbus
    module(vec3(-0.55, 0.0, 0.68), Vec3::X, 0.8, 0.24, blanc, sombre); // Kibo
    module(vec3(-1.0, 0.42, 0.68), Vec3::Y, 0.32, 0.15, blanc, sombre); // ELM (dessus)
    draw_cube(vec3(-1.0, 0.0, 1.0), vec3(0.42, 0.22, 0.32), None, metal); // palette
    draw_cube_wires(vec3(-1.0, 0.0, 1.0), vec3(0.42, 0.22, 0.32), sombre);

    // Tranquility + Cupola vers le nadir (petit décrochement).
    module(vec3(0.0, -0.35, -0.1), Vec3::Y, 0.4, 0.2, blanc, sombre);
    draw_sphere(vec3(0.0, -0.6, -0.1), 0.14, None, sombre); // Cupola

    // ------------------------------------------------------------------
    // Segment russe suspendu sous le centre (−Y) : Zarya/Zvezda + panneaux
    // bleus, prolongé par un vaisseau (Soyouz/Progress) amarré en bout.
    // ------------------------------------------------------------------
    draw_sphere(vec3(0.0, -0.72, -0.15), 0.18, None, or); // nœud FGB
    module(vec3(0.0, -1.05, -0.15), Vec3::Y, 0.55, 0.20, or, sombre); // Zvezda
    // Deux panneaux solaires bleus déployés le long de X.
    for signe in [-1.0_f32, 1.0] {
        pale_solaire(
            vec3(signe * 0.22, -1.05, -0.15),
            signe * Vec3::X,
            Vec3::Z,
            0.75,
            0.5,
            5,
            bleu,
        );
    }
    module(vec3(0.0, -1.5, -0.15), Vec3::Y, 0.4, 0.16, or, sombre); // module de service
    // Vaisseau amarré (Soyouz) en bout, avec petits panneaux.
    let soyouz = vec3(0.0, -1.85, -0.15);
    draw_sphere(soyouz, 0.15, None, metal);
    cylindre(soyouz, soyouz + vec3(0.0, -0.28, 0.0), 0.12, Color::new(0.30, 0.45, 0.32, 1.0));
    for signe in [-1.0_f32, 1.0] {
        pale_solaire(
            vec3(signe * 0.14, -2.05, -0.15),
            signe * Vec3::X,
            Vec3::Z,
            0.4,
            0.28,
            3,
            bleu,
        );
    }

    // ------------------------------------------------------------------
    // Détails : antenne parabolique et bras Canadarm2 articulé.
    // ------------------------------------------------------------------
    parabole(vec3(0.55, 0.28, 0.4), vec3(0.2, 0.9, 0.4), 0.18, metal);
    let a0 = vec3(-0.2, 0.2, 0.2);
    let a1 = vec3(-0.85, 0.7, 0.4);
    let a2 = vec3(-1.5, 0.35, 0.1);
    cylindre(a0, a1, 0.05, blanc);
    cylindre(a1, a2, 0.05, blanc);
    draw_sphere(a1, 0.09, None, metal);
    draw_sphere(a0, 0.07, None, metal);
}
