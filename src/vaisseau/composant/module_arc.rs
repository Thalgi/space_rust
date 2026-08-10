//! **Module en coin d'arc** — la brique de l'Endurance (*Interstellar*),
//! `docs/conception/stations.md` §D.3, étape 4 de l'ordre de travail §D.5.
//!
//! Un anneau fermé de douze modules, sans moyeu ni rayons : **l'anneau *est* le
//! vaisseau**, et il tourne sur lui-même pour la gravité.
//!
//! ⚠️ **Ce n'est pas un tore.** `Composant::Tore` est une surface de révolution ;
//! la section de l'Endurance est un **caisson à faces planes**. Réutiliser le
//! tore la rendrait ronde là où elle est facettée — c'est-à-dire lui ôterait sa
//! silhouette. D'où une brique à part, qui balaie une section rectangulaire le
//! long d'un arc en un petit nombre de **facettes plates**.
//!
//! ## Comment douze modules ferment un cercle
//!
//! Chaque module porte deux ports de bout, sur ses deux faces radiales. Le port
//! de début (indice 0) regarde en arrière, celui de fin (indice 1) en avant, le
//! long de la tangente. Les chaîner par `accoupler` fait tourner chaque suivant
//! de `arc` : après `2π / arc` modules, on est revenu au départ.
//!
//! ⚠️ **Le graphe de ports est un arbre : l'anneau ne se referme pas en cycle.**
//! Le douzième module *touche* le premier géométriquement — ses ports libres
//! coïncident — mais rien ne les soude, et le modèle continue de voir une
//! chaîne. C'est sans conséquence pour le rendu comme pour la sauvegarde ; ça
//! en aurait une le jour où l'on voudrait qu'un retrait au milieu casse
//! l'anneau en deux, ce que personne ne demande.

use crate::vaisseau::peintre::Peintre;
use crate::vaisseau::{GenrePort, Port, Profil, Repere};
use macroquad::prelude::*;
use std::f32::consts::PI;

/// Nombre de modules d'une Endurance — douze coins de 30°.
pub const ENDURANCE_MODULES: usize = 12;

/// Arc d'un module d'Endurance, en radians.
pub fn arc_endurance() -> f32 {
    2.0 * PI / ENDURANCE_MODULES as f32
}

/// Angle maximal d'une facette : au-delà, la corde s'écarte trop de l'arc et le
/// module se lit comme un polygone grossier plutôt que comme un caisson courbe.
const FACETTE_MAX: f32 = PI / 24.0; // 7,5°

/// Les quatre familles qui se répètent autour de l'anneau (§D.3).
#[derive(Clone, Copy, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
pub enum VarianteArc {
    /// Habitat permanent : hublots et main courante.
    Habitat,
    /// Soute : panneaux nus, écoutille de fret.
    Soute,
    /// Poste d'amarrage d'un **Ranger** (le petit vaisseau à aile).
    PosteRanger,
    /// Poste d'amarrage d'un **Lander** (le gros atterrisseur trapu).
    PosteLander,
}

impl VarianteArc {
    /// ⚠️ `TOUS` et `nom` ne sont consommes que par les tests pour l'instant.
    /// **La palette de l'assembleur ne liste qu'un echantillon par variante de
    /// `Composant`, pas par sous-variante** (meme limite que `VarianteCaisson`
    /// et toutes les autres familles) : on ne peut donc pas encore choisir
    /// « ARC SOUTE » a l'ecran, seulement la famille par defaut. C'est une
    /// vraie lacune pour l'Endurance, dont l'interet est justement l'alternance
    /// des quatre familles autour de l'anneau.
    #[allow(dead_code)]
    pub const TOUS: [VarianteArc; 4] = [
        VarianteArc::Habitat,
        VarianteArc::Soute,
        VarianteArc::PosteRanger,
        VarianteArc::PosteLander,
    ];

    #[allow(dead_code)] // voir la note sur TOUS
    pub fn nom(self) -> &'static str {
        match self {
            VarianteArc::Habitat => "ARC HABITAT",
            VarianteArc::Soute => "ARC SOUTE",
            VarianteArc::PosteRanger => "ARC POSTE RANGER",
            VarianteArc::PosteLander => "ARC POSTE LANDER",
        }
    }

    pub(super) fn cout(self) -> f32 {
        match self {
            VarianteArc::Habitat => 26.0,
            VarianteArc::Soute => 18.0,
            VarianteArc::PosteRanger => 22.0,
            VarianteArc::PosteLander => 24.0,
        }
    }
}

/// Centre de courbure, en repère local : l'origine est le **milieu de la face
/// de début**, et le centre de l'anneau se trouve à `rayon` sur −X.
fn centre(rayon: f32) -> Vec3 {
    vec3(-rayon, 0.0, 0.0)
}

/// Point de la ligne moyenne à l'angle `t` (0 = face de début).
fn point(rayon: f32, t: f32) -> Vec3 {
    centre(rayon) + vec3(rayon * t.cos(), 0.0, rayon * t.sin())
}

/// Orientation à l'angle `t` : Z suit la tangente (sens de parcours), X pointe
/// vers l'**extérieur** de l'anneau, Y reste le haut.
fn repere_a(t: f32) -> Quat {
    Quat::from_rotation_y(-t)
}

/// Nombre de facettes plates pour couvrir `arc`. Au moins deux : une seule
/// donnerait une corde droite, donc un module qui ne courbe pas du tout.
fn facettes(arc: f32) -> usize {
    ((arc.abs() / FACETTE_MAX).ceil() as usize).max(2)
}

/// Bout de la ligne moyenne, en repère local — l'axe de la capsule englobante.
pub(super) fn bout(rayon: f32, arc: f32) -> Vec3 {
    point(rayon, arc)
}

/// Les deux ports de bout. Voir l'en-tête du module pour le chaînage.
pub(super) fn ports(profil: Profil, rayon: f32, arc: f32) -> Vec<Port> {
    vec![
        // 0 — face de début, à l'origine, regardant en arrière.
        Port::new(
            Repere::new(Vec3::ZERO, Quat::from_rotation_y(PI)),
            GenrePort::ModuleAxial,
            profil,
        ),
        // 1 — face de fin, au bout de l'arc, regardant en avant le long de la
        // tangente. C'est ce port qu'on vise pour poser le module suivant.
        Port::new(
            Repere::new(point(rayon, arc), repere_a(arc)),
            GenrePort::ModuleAxial,
            profil,
        ),
    ]
}

pub(super) fn dessiner<P: Peintre>(
    p: &mut P,
    variante: VarianteArc,
    rayon: f32,
    arc: f32,
    section: f32,
) {
    let coque = Color::new(0.74, 0.76, 0.80, 1.0);
    let sombre = Color::new(0.20, 0.22, 0.26, 1.0);
    let metal = Color::new(0.58, 0.60, 0.65, 1.0);
    let n = facettes(arc);

    for k in 0..n {
        let t0 = arc * k as f32 / n as f32;
        let t1 = arc * (k + 1) as f32 / n as f32;
        let (a, b) = (point(rayon, t0), point(rayon, t1));
        // La facette est **plate** : sa boîte suit la corde, pas l'arc. C'est
        // exactement ce qui donne les arêtes visibles de l'Endurance.
        let milieu = (a + b) * 0.5;
        let corde = (b - a).length();
        let rot = repere_a((t0 + t1) * 0.5);
        p.empiler_transforme(Mat4::from_rotation_translation(rot, milieu));
        p.cube(Vec3::ZERO, vec3(section, section, corde), coque);
        // Arêtes : sans elles, douze facettes claires se lisent comme un tube.
        p.cube_fil(Vec3::ZERO, vec3(section, section, corde), sombre);
        detail(p, variante, Facette { section, corde, k, n }, metal, sombre);
        p.depiler_transforme();
    }
}

/// Où l'on en est dans le balayage : la facette `k` sur `n`, ses cotes.
/// Groupé plutôt que passé en cinq scalaires — deux `f32` et deux `usize` à la
/// suite s'intervertissent sans que le compilateur bronche.
struct Facette {
    section: f32,
    corde: f32,
    k: usize,
    n: usize,
}

/// Ce qui distingue les quatre familles, dessiné **dans le repère de la
/// facette** : X vers l'extérieur de l'anneau, Y le haut, Z la tangente.
fn detail<P: Peintre>(p: &mut P, variante: VarianteArc, f: Facette, metal: Color, sombre: Color) {
    let Facette { section, corde, k, n } = f;
    let dehors = section * 0.5;
    match variante {
        VarianteArc::Habitat => {
            // Hublots sur la face extérieure, et main courante le long du toit.
            let verre = Color::new(0.12, 0.30, 0.42, 1.0);
            for s in [-1.0_f32, 1.0] {
                p.cube(
                    vec3(dehors, section * 0.12, corde * 0.22 * s),
                    vec3(section * 0.04, section * 0.22, corde * 0.26),
                    verre,
                );
            }
            let y = section * 0.5;
            p.cylindre(
                vec3(dehors * 0.55, y, -corde * 0.45),
                vec3(dehors * 0.55, y, corde * 0.45),
                section * 0.025,
                metal,
            );
        }
        VarianteArc::Soute => {
            // Panneaux nus cerclés, et une écoutille de fret par module.
            for t in [-0.28_f32, 0.28] {
                p.cube(
                    vec3(0.0, 0.0, corde * t),
                    vec3(section * 1.02, section * 1.02, corde * 0.05),
                    sombre,
                );
            }
            if k == n / 2 {
                p.cylindre(
                    vec3(dehors, 0.0, 0.0),
                    vec3(dehors + section * 0.06, 0.0, 0.0),
                    section * 0.30,
                    sombre,
                );
            }
        }
        VarianteArc::PosteRanger => {
            // Berceau plat sur la face **intérieure** : le Ranger se range dans
            // le creux de l'anneau, pas à l'extérieur.
            if k == n / 2 {
                p.cube(
                    vec3(-dehors - section * 0.10, 0.0, 0.0),
                    vec3(section * 0.20, section * 0.62, corde * 0.72),
                    metal,
                );
                for s in [-1.0_f32, 1.0] {
                    p.cylindre(
                        vec3(-dehors, section * 0.26 * s, 0.0),
                        vec3(-dehors - section * 0.20, section * 0.26 * s, 0.0),
                        section * 0.04,
                        sombre,
                    );
                }
            }
        }
        VarianteArc::PosteLander => {
            // Même idée, plus massive : le Lander est trapu, son berceau déborde.
            if k == n / 2 {
                p.cube(
                    vec3(-dehors - section * 0.16, 0.0, 0.0),
                    vec3(section * 0.32, section * 0.82, corde * 0.92),
                    metal,
                );
                p.cube(
                    vec3(-dehors - section * 0.34, 0.0, 0.0),
                    vec3(section * 0.10, section * 0.50, corde * 0.50),
                    sombre,
                );
            }
        }
    }
}

/// Rayon englobant local : le module s'étend de l'origine jusqu'au bout de
/// l'arc, et déborde de sa demi-section dans toutes les directions.
///
/// Mesuré sur la **corde** (origine → point le plus lointain de la ligne
/// moyenne), pas sur la longueur d'arc : c'est la distance euclidienne qui
/// compte pour une sphère englobante, et la prendre curviligne la surestimerait.
pub(super) fn rayon_local(rayon: f32, arc: f32, section: f32) -> f32 {
    let mut loin = 0.0_f32;
    let n = facettes(arc);
    for k in 0..=n {
        let t = arc * k as f32 / n as f32;
        loin = loin.max(point(rayon, t).length());
    }
    loin + section * 0.5 * 3.0_f32.sqrt()
}

pub(super) fn cout(variante: VarianteArc) -> f32 {
    variante.cout()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vaisseau::accoupler;

    /// **Douze modules chaînés ferment le cercle.** C'est la propriété qui fait
    /// exister la brique : chaîner par `accoupler` doit ramener au point de
    /// départ, sans que le preset ait à recaler quoi que ce soit à la main.
    ///
    /// Le test suit la **pose réelle** — celle que `Chantier::poser` produirait —
    /// plutôt que de recalculer un cercle à côté : c'est l'accord entre les deux
    /// qui est en jeu, pas la trigonométrie.
    ///
    /// ⚠️ **Ce test ne prouve que la ROTATION**, et c'est le red-check qui l'a
    /// appris. Déplacer le port de fin à `0,9 × arc` le laisse **vert** : la
    /// position du corps s'obtient en sommant douze fois le même vecteur, tourné
    /// à chaque pas d'un douzième de tour — et la somme d'un vecteur *quelconque*
    /// sur les douze multiples de 30° est nulle par symétrie. **N'importe quelle
    /// position de port ferme donc l'anneau**, du moment que la rotation vaut
    /// bien 2π/12.
    ///
    /// Ce qui lie enfin la position à la rotation est
    /// `les_ports_sont_tangents_a_l_anneau`, juste en dessous. Et une inversion
    /// *globale* du signe reste invisible aux deux (la chaîne tourne à l'envers
    /// mais reste cohérente) : c'est
    /// `les_rayons_declares_contiennent_la_piece`, qui confronte la géométrie
    /// **dessinée** au rayon déclaré, qui attrape ce cas-là.
    #[test]
    fn douze_modules_chaines_referment_l_anneau() {
        let (r, a) = (14.2_f32, arc_endurance());
        let ports = ports(Profil::P2, r, a);
        // Premier module : posé à l'identité.
        let mut corps = Repere::IDENTITE;
        let depart = corps.compose(ports[0].repere);
        for _ in 0..ENDURANCE_MODULES - 1 {
            // Le suivant s'accouple par son port 0 sur le port 1 du courant.
            let hote = corps.compose(ports[1].repere);
            corps = accoupler(hote, ports[0].repere);
        }
        // Le port de FIN du douzième doit retomber sur le port de DÉBUT du
        // premier : l'anneau est bouclé.
        let fin = corps.compose(ports[1].repere);
        assert!(
            fin.pos.distance(depart.pos) < 1e-3,
            "l'anneau ne se referme pas : {:?} vs {:?}",
            fin.pos,
            depart.pos
        );
    }

    /// **Les deux ports sont tangents à l'anneau.** C'est ce qui lie leur
    /// *position* à leur *orientation* — et donc ce qui manquait au test de
    /// fermeture ci-dessus, qui ne voyait que la rotation.
    ///
    /// Un port d'anneau regarde le long de la tangente : sa direction avant doit
    /// être perpendiculaire au rayon qui le porte. Décaler sa position sans
    /// tourner son repère (ou l'inverse) casse cette perpendicularité tout de
    /// suite.
    #[test]
    fn les_ports_sont_tangents_a_l_anneau() {
        let (r, a) = (14.2_f32, arc_endurance());
        let c = centre(r);
        for arc in [a, PI / 8.0, PI / 3.0] {
            for p in ports(Profil::P2, r, arc) {
                let rayon_vers_port = (p.repere.pos - c).normalize();
                let cos = p.repere.avant().dot(rayon_vers_port).abs();
                assert!(
                    cos < 1e-3,
                    "arc {arc} : port non tangent (cos = {cos}) — position et orientation                      ne parlent pas du même angle"
                );
            }
        }
    }

    /// **Le module reste dans le plan de l'anneau.** Une erreur de signe dans le
    /// repère ferait vriller la chaîne en hélice, ce que la fermeture seule ne
    /// détecterait pas si l'écart restait petit.
    #[test]
    fn la_chaine_ne_vrille_pas_hors_du_plan() {
        let (r, a) = (14.2_f32, arc_endurance());
        let ports = ports(Profil::P2, r, a);
        let mut corps = Repere::IDENTITE;
        for k in 0..ENDURANCE_MODULES {
            let bout = corps.compose(ports[1].repere);
            assert!(
                bout.pos.y.abs() < 1e-3,
                "module {k} : la chaîne a quitté le plan (y = {})",
                bout.pos.y
            );
            let hote = corps.compose(ports[1].repere);
            corps = accoupler(hote, ports[0].repere);
        }
    }

    /// **Le rayon de l'anneau est bien celui qu'on demande.** Les douze centres
    /// de facettes doivent tomber à `rayon` du centre de courbure — sans quoi
    /// une Endurance de 28,4 U de diamètre n'en ferait pas 28,4.
    #[test]
    fn la_ligne_moyenne_est_a_la_bonne_distance_du_centre() {
        let (r, a) = (14.2_f32, arc_endurance());
        let c = centre(r);
        for k in 0..=8 {
            let t = a * k as f32 / 8.0;
            let d = point(r, t).distance(c);
            assert!((d - r).abs() < 1e-4, "t={t} : rayon {d} au lieu de {r}");
        }
    }

    /// **Un arc plus large demande plus de facettes.** Le module doit rester
    /// courbe quel que soit l'angle : deux facettes sur 90° donneraient un coude.
    #[test]
    fn le_nombre_de_facettes_suit_l_angle() {
        assert!(facettes(arc_endurance()) >= 4, "30° mérite au moins 4 facettes");
        assert!(facettes(PI / 2.0) > facettes(arc_endurance()), "90° devrait en demander plus");
        // Et jamais moins de deux, même pour un arc minuscule.
        assert!(facettes(0.001) >= 2);
    }

    /// **Les quatre familles ont chacune leur nom et leur coût.** Elles se
    /// répètent autour de l'anneau (§D.3) : deux qui se ressembleraient à
    /// l'écran ne feraient plus une alternance, mais un anneau uniforme.
    #[test]
    fn les_quatre_familles_sont_nommees_et_facturees() {
        let mut noms: Vec<&str> = VarianteArc::TOUS.iter().map(|v| v.nom()).collect();
        noms.sort_unstable();
        let avant = noms.len();
        noms.dedup();
        assert_eq!(noms.len(), avant, "deux familles d'arc portent le même nom");
        for v in VarianteArc::TOUS {
            assert!(v.cout() > 0.0, "{} est gratuite", v.nom());
        }
        assert_eq!(VarianteArc::TOUS.len(), 4, "§D.3 en annonce quatre");
    }

    /// L'enveloppe contient le module : le point le plus lointain de la ligne
    /// moyenne, sa demi-section comprise, tient dans le rayon annoncé.
    #[test]
    fn le_rayon_local_contient_le_module() {
        let (r, a, s) = (14.2_f32, arc_endurance(), 2.0);
        let borne = rayon_local(r, a, s);
        for k in 0..=16 {
            let t = a * k as f32 / 16.0;
            let d = point(r, t).length() + s * 0.5 * 3.0_f32.sqrt();
            assert!(d <= borne + 1e-4, "t={t} : {d} dépasse le rayon local {borne}");
        }
    }
}
