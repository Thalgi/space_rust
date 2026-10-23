use super::eruptions::{encoder_flare, encoder_tache, rayon_visible, EtatBoucle};
use super::{Soleil, MAX_TACHES};
use crate::astre::{Astre, CameraInfo};
use crate::impostor::push_quad;
use macroquad::models::{draw_mesh, Mesh, Vertex};
use macroquad::prelude::*;

pub(super) const TAILLE_HALO: usize = 64;
const QUADS_PAR_LOT: usize = 200;

impl Soleil {
    pub(super) fn dessiner(&mut self, cam: &CameraInfo) {
        let c = self.base.position;
        // Rayon DESSINÉ : plancher en pixels (voir `soleil::PX_MIN`). Tout ce
        // qui suit en dérive — quad, boucles, éjections, occultation. Reprendre
        // `base.rayon` pour l'un d'eux détacherait les protubérances de la
        // surface et les ferait masquer par une sphère de la mauvaise taille.
        let r = self.rayon_dessine(cam.pos);
        // Éclat de surface : flare et pulsation, compensés de la taille (voir
        // `eclat_surfacique`). Sans lui, ni l'un ni l'autre n'atteignait
        // l'image — ils n'éclairaient que les planètes.
        let eclat = self.eclat_dessin(cam.pos);
        let rot = self.temps * 0.12;
        let axe_rot = self.axe_rot;

        // **Le disque, moitié arrière d'abord** : l'horizon la masquera.
        // Même encadrement qu'un anneau planétaire (`planete/mod.rs`), et
        // pour la même raison — sans lui, un disque translucide dessiné d'un
        // seul tenant passe devant l'astre qu'il contourne.
        for d in &mut self.disques {
            d.corps_mut().position = c;
            d.draw_moitie(cam, -1.0);
        }

        // ---- 1) Corps : quad + shader ----
        self.verts.clear();
        self.inds.clear();
        push_quad(&mut self.verts, &mut self.inds, cam, c, r * self.couronne, WHITE);

        // Taches ET flares -> même uniform array (xyz = direction surface, w =
        // rayon effectif SIGNÉ : positif tache assombrissante, négatif flare
        // éclaircissant — voir `eruptions::encoder_flare`). Un seul tableau
        // d'uniforms, faute de place pour un second pipeline.
        let mut spots = [Vec4::ZERO; MAX_TACHES];
        let mut n = 0;
        for t in self.taches.iter() {
            if n >= MAX_TACHES {
                break;
            }
            spots[n] = vec4(t.dir.x, t.dir.y, t.dir.z, encoder_tache(rayon_visible(t)));
            n += 1;
        }
        for f in self.flares.iter() {
            if n >= MAX_TACHES {
                break;
            }
            spots[n] = vec4(f.dir.x, f.dir.y, f.dir.z, encoder_flare(f.taille * f.intensite));
            n += 1;
        }

        // Les pôles, tels quels : le shader y lira le même champ que le CPU.
        // Les deux vivent dans le repère « surface », celui qui tourne avec
        // l'étoile — comme `spots` juste au-dessus.
        let champ_uniforme: Vec<Vec4> = self
            .poles
            .iter()
            .map(|p| vec4(p.dir.x, p.dir.y, p.dir.z, p.force))
            .collect();

        // DEUX PASSES sur le même quad et le même shader (voir soleil.frag.glsl) :
        // le disque de l'étoile écrit la profondeur, la couronne non. La couronne
        // vient APRÈS : translucide, elle se dépose sur le disque déjà peint.
        let passes = super::materiau::PASSES;
        for (i, ((zone, _), mat)) in passes.iter().zip(&self.mats).enumerate() {
            let zone = *zone;
            // Uniforms communs (temps, repère caméra, échelle du disque = couronne).
            crate::impostor::uniforms_standard(mat, cam, self.temps, self.couronne);
            mat.set_uniform("teinte", (self.couleur.x, self.couleur.y, self.couleur.z));
            mat.set_uniform("couronne", self.couronne);
            mat.set_uniform("couronne_irreg", self.couronne_irreg);
            mat.set_uniform("couronne_type", self.couronne_type);
            mat.set_uniform("zone", zone);
            mat.set_uniform("graine", self.graine);
            mat.set_uniform("ebullition", self.ebullition);
            mat.set_uniform("ebul_style", self.ebul_style);
            mat.set_uniform("eclat", eclat);
            mat.set_uniform("surface", (self.surface.x, self.surface.y, self.surface.z));
            mat.set_uniform("ebul_reglage", (self.ebul_reglage.x, self.ebul_reglage.y));
            mat.set_uniform("coeur", self.coeur);
            let q = self.axe_rot;
            mat.set_uniform("axe_rot", (q.x, q.y, q.z, q.w));
            mat.set_uniform("accretion_pol", self.accretion_pol);
            // Les taches suivent le dipôle de son disque, s'il est incliné.
            let mag = self.disques.iter().find_map(|d| d.axe_magnetique()).unwrap_or(Vec3::ZERO);
            mat.set_uniform("axe_magnetique", (mag.x, mag.y, mag.z));
            mat.set_uniform_array("spots", &spots[..]);
            mat.set_uniform_array("poles", &champ_uniforme[..]);

            gl_use_material(mat);
            let quad = Mesh {
                vertices: std::mem::take(&mut self.verts),
                indices: std::mem::take(&mut self.inds),
                texture: None,
            };
            draw_mesh(&quad);
            // Les materials sont partagés entre toutes les étoiles : `set_uniform`
            // écrit dans le pipeline, pas dans le clone. Sans ce vidage, la
            // dernière étoile dessinée imposerait sa teinte et sa couronne aux
            // autres — et ici, sa `zone` à la passe d'à côté.
            unsafe {
                get_internal_gl().flush();
            }
            self.verts = quad.vertices;
            self.inds = quad.indices;
            // Le quad est identique d'une passe à l'autre : on le reconstruit
            // tant qu'il en reste une.
            if i + 1 < passes.len() {
                self.verts.clear();
                self.inds.clear();
                push_quad(&mut self.verts, &mut self.inds, cam, c, r * self.couronne, WHITE);
            }
        }
        gl_use_default_material();

        // ---- 2) Boucles + particules : billboards en additif ----
        gl_use_material(&self.mat_plasma);
        self.verts.clear();
        self.inds.clear();
        let mut quads = 0;

        // **Le pinceau, pas l'effet.** Une éjection s'étend sur 5 à 6 rayons
        // stellaires — 24 px au cadrage par défaut, largement visible. Mais
        // on la dessinait avec des billboards de `r × 0,05`, soit **0,20 px**
        // : le rasterizer les avalait, et l'effet entier disparaissait alors
        // que sa géométrie était bonne. Même remède que les débris
        // (`disque_particules.vert.glsl`) : plancher en pixels et fondu.
        let dist = (cam.pos - c).length();
        let vh = crate::impostor::viewport_h();
        for b in &self.boucles {
            let a_w = vers_monde(b.sa, rot, axe_rot);
            let b_w = vers_monde(b.sb, rot, axe_rot);
            // Le calibre propre à cette boucle-ci : un fil ou un ruban, et
            // **proportionné à sa propre étendue** (voir [`RAPPORT_TUBE`]).
            let (calibre, n) = tube(
                etendue(a_w, b_w, b.apex, r),
                etendue(a_w, b_w, b.apex.min(b.apex_max), r),
                b.epaisseur,
            );
            // Un **ruban** et non plus un chapelet de billboards : la
            // continuité longitudinale est ce qui manquait le plus — on
            // devinait les pastilles une à une le long d'une arche isolée
            // (`astres_v2_boucles.md` §2). Et c'est moins cher : deux sommets
            // par point au lieu de quatre.
            let courbe = le_long(a_w, b_w, profil(b), r, n);
            let mut brin: Vec<crate::impostor::PointRuban> = Vec::with_capacity(n + 1);
            for (i, point) in courbe.iter().enumerate() {
                // `t` est ici la **fraction de trajet**, pas le paramètre de
                // la cubique : les points sont équidistants le long de la
                // courbe (voir [`le_long`]).
                let t = i as f32 / n as f32;
                let monde = c + *point;
                if cache_par_sphere(monde, c, r, cam.pos) {
                    // Derrière le soleil : le ruban se coupe ici et reprend
                    // au premier point revenu en vue. Un `continue` aurait
                    // relié les deux bords par-dessus l'étoile.
                    quads += emettre(&mut self.verts, &mut self.inds, cam, &mut brin);
                    continue;
                }
                let (demi, fondu) = plancher_plasma(calibre * galbe(t), dist, vh);
                // onde de matière qui circule + intensité de la boucle
                let flow = 0.55 + 0.45 * (t * 16.0 - self.temps * 6.0).sin();
                let lum = (b.intensite * (0.55 + 0.45 * flow)).clamp(0.0, 1.0);
                // plasma chaud, additif -> les recouvrements forment un cœur blanc
                let chaud = vec3(1.0, 0.8 + 0.15 * lum, 0.5 + 0.3 * lum);
                // **Ancrage du pied** : près de la surface, le plasma cède la
                // place à la teinte de l'étoile et perd de l'éclat. Sans ça,
                // l'arche s'arrête net sur la photosphère et se lit comme
                // posée devant elle plutôt qu'ancrée dedans.
                let dedans = ancrage(t);
                let teinte = self.couleur.lerp(chaud, dedans);
                let col = Color::new(
                    teinte.x,
                    teinte.y,
                    teinte.z,
                    lum * fondu * (0.45 + 0.55 * dedans),
                );
                brin.push((monde, demi, col));
            }
            quads += emettre(&mut self.verts, &mut self.inds, cam, &mut brin);
            if quads >= QUADS_PAR_LOT {
                flush(&mut self.verts, &mut self.inds, &self.halo);
                quads = 0;
            }

            // Une lueur au pied, qui fait l'articulation avec la surface.
            for pied in [a_w, b_w] {
                let monde = c + pied * r;
                if cache_par_sphere(monde, c, r, cam.pos) {
                    continue;
                }
                let (taille, fondu) = plancher_plasma(calibre * 2.4, dist, vh);
                let lueur = self.couleur.lerp(vec3(1.0, 0.85, 0.6), 0.5);
                let a = b.intensite.clamp(0.0, 1.0) * fondu * 0.35;
                let col = Color::new(lueur.x, lueur.y, lueur.z, a);
                push_quad(&mut self.verts, &mut self.inds, cam, monde, taille, col);
                quads += 1;
            }
            if quads >= QUADS_PAR_LOT {
                flush(&mut self.verts, &mut self.inds, &self.halo);
                quads = 0;
            }
        }

        // Cœur dense brillant au sommet des éjections (matière de proéminence).
        for b in &self.boucles {
            if b.etat == EtatBoucle::Rupture {
                let a_w = vers_monde(b.sa, rot, axe_rot);
                let b_w = vers_monde(b.sb, rot, axe_rot);
                // Le milieu du **trajet**, pas le milieu du paramètre : sur
                // une arche étirée les deux sont très loin l'un de l'autre.
                let courbe = le_long(a_w, b_w, profil(b), r, 8);
                let pos = c + courbe[courbe.len() / 2];
                if !cache_par_sphere(pos, c, r, cam.pos) {
                    let (calibre, _) = tube(1.0, etendue(a_w, b_w, b.apex.min(b.apex_max), r), b.epaisseur);
                    let (taille_c, fondu_c) = plancher_plasma(calibre * 3.2, dist, vh);
                    let lum = b.intensite.clamp(0.0, 1.0);
                    let col = Color::new(1.0, 0.72, 0.45, lum * fondu_c);
                    push_quad(&mut self.verts, &mut self.inds, cam, pos, taille_c, col);
                    quads += 1;
                    if quads >= QUADS_PAR_LOT {
                        flush(&mut self.verts, &mut self.inds, &self.halo);
                        quads = 0;
                    }
                }
            }
        }
        // ---- 3) Jets en 3D (lot 12) : même pinceau que la queue des
        // comètes — un chapelet de nuages face caméra, qui n'a pas
        // d'orientation propre et se lit sous tous les angles — avec ses
        // bouffées, qui sont ici les **nœuds Herbig-Haro** des vrais jets.
        if self.jets_3d > 0.0 {
            let axe = self.axe_jets();
            let jetcol = self.couleur.lerp(vec3(0.6, 0.8, 1.0), 0.55);
            for (k, sens) in [1.0_f32, -1.0].into_iter().enumerate() {
                let brut = jet(c, axe * sens, r, self.jets_3d, jetcol);
                let mut nuages = crate::comete::nuage(&brut);
                // Graine décalée par côté : les deux jets ne crachent pas leurs
                // nœuds en même temps.
                crate::comete::fluctuer(&mut nuages, &FLUCT_JETS, self.temps, self.graine + k as f32 * 3.7);
                for (pos, demi, col) in nuages {
                    let (taille, fondu) = plancher_plasma(demi, dist, vh);
                    // Prémultiplié comme le reste du plasma (additif).
                    let a = col.a * fondu;
                    let col = Color::new(col.r * a, col.g * a, col.b * a, a);
                    push_quad(&mut self.verts, &mut self.inds, cam, pos, taille, col);
                    quads += 1;
                    if quads >= QUADS_PAR_LOT {
                        flush(&mut self.verts, &mut self.inds, &self.halo);
                        quads = 0;
                    }
                }
            }
        }
        flush(&mut self.verts, &mut self.inds, &self.halo);
        gl_use_default_material();

        // Moitié avant APRÈS tout le reste : elle passe devant l'horizon.
        for d in &mut self.disques {
            d.draw_moitie(cam, 1.0);
        }
    }
}

/// **Les nœuds des jets** : rapides et serrés — la matière y file bien plus
/// vite que la poussière d'une comète, et un vrai jet de jeune étoile est un
/// chapelet de condensations (objets Herbig-Haro), pas un trait continu.
const FLUCT_JETS: crate::comete::Fluctuation = crate::comete::Fluctuation {
    cadence: 1.2,
    traversee: 1.8,
    eclat: 0.9,
    largeur: 0.35,
    taille: 0.04,
    creux: 0.45,
};

/// **Un jet**, de la surface vers le large le long de `axe` (unitaire) :
/// `longueur` en rayons, cône qui s'évase, éclat qui s'éteint vers le bout.
/// Pur : la géométrie se teste, le dessin non.
fn jet(c: Vec3, axe: Vec3, r: f32, longueur: f32, couleur: Vec3) -> Vec<crate::impostor::PointRuban> {
    const N: usize = 24;
    (0..=N)
        .map(|k| {
            let s = k as f32 / N as f32;
            let pos = c + axe * (r * 1.02 + r * longueur * s);
            // Étroit au pied — la matière sort collimatée — puis s'ouvre.
            let demi = r * (0.10 + 0.28 * s);
            let alpha = 0.9 * (1.0 - s).powf(0.8);
            (pos, demi, Color::new(couleur.x, couleur.y, couleur.z, alpha))
        })
        .collect()
}

/// **Demi-taille écran minimale d'un billboard de plasma.**
///
/// En dessous, le quad tombe sous le pixel et le rasterizer l'avale : c'est
/// ce qui faisait disparaître les éjections alors que leur géométrie
/// s'étendait sur des dizaines de pixels. Même valeur d'esprit que le
/// `px_min` des débris, et pour la même raison.
const PLASMA_PX_MIN: f32 = 1.5;

/// Taille d'un billboard de plasma, plancher appliqué, **et le fondu qui va
/// avec**.
///
/// ⚠️ Le fondu n'est pas une coquetterie. Agrandir un quad sans pâlir
/// revient à **ajouter de la matière** : en blending additif, une éjection
/// vue de loin deviendrait une tache blanche plus brillante que de près,
/// exactement l'inverse de ce qu'on veut. On rend donc à l'alpha ce que le
/// plancher a pris à la taille.
fn plancher_plasma(taille: f32, dist: f32, vh: f32) -> (f32, f32) {
    if taille <= 0.0 {
        return (taille, 1.0); // rien à dessiner, et pas de division par zéro
    }
    let planchee = crate::impostor::rayon_plancher(taille, dist, vh, PLASMA_PX_MIN);
    // Le quad couvre (planchee/taille)² fois plus de surface ; on lui retire
    // autant d'éclat, avec un plancher pour qu'il ne s'éteigne jamais tout à
    // fait — un fil de lumière reste plus lisible que rien.
    //
    // ⚠️ Pas de cas particulier « déjà assez gros » : `rayon_plancher` rend
    // alors la taille **inchangée**, donc le rapport vaut 1 et le fondu
    // aussi. Une première version en avait un ; le sabotage qui l'a retiré
    // n'a fait rougir aucun test, et pour cause — il ne servait à rien
    // (`docs/methode.md` : vérifier qu'un sabotage produit bien le défaut
    // visé avant d'accuser le test).
    let rapport = taille / planchee;
    (planchee, (rapport * rapport).clamp(0.12, 1.0))
}

/// Passe une direction du repère "surface" (où vivent taches/boucles) au repère monde,
/// en inversant la rotation appliquée par le shader.
/// Du repère de la surface (qui tourne autour de +Y) au repère monde : la
/// rotation propre, puis l'**inclinaison de l'axe** (`axe_rot`, identité pour
/// la plupart des étoiles). ⚠️ Inverse exact de `tourner` dans
/// `soleil.frag.glsl` : c'est ce qui fait naître une arche sur sa tache.
fn vers_monde(s: Vec3, a: f32, axe_rot: Quat) -> Vec3 {
    let (sa, ca) = a.sin_cos();
    axe_rot * vec3(s.x * ca + s.z * sa, s.y, -s.x * sa + s.z * ca)
}

/// Point sur l'arche (Bézier quadratique a -> sommet -> b), relatif au centre.
/// `skew` décale le sommet le long de la ligne des pieds (arches asymétriques).
/// La silhouette d'une boucle, au-delà de sa hauteur : de quoi distinguer
/// deux boucles de même portée.
#[derive(Clone, Copy)]
pub(super) struct Profil {
    pub apex: f32,
    pub skew: f32,
    pub pincee: f32,
    pub inclinaison: f32,
}

fn profil(b: &super::eruptions::Boucle) -> Profil {
    Profil { apex: b.apex, skew: b.skew, pincee: b.pincee, inclinaison: b.inclinaison }
}

/// **Élancement d'un tube de plasma** : son épaisseur, en part de sa propre
/// étendue.
///
/// ⚠️ L'épaisseur était `rayon_de_l_étoile × 0,05` — indexée sur l'**étoile**
/// et non sur la boucle. Une grande arche y trouvait son compte, mais une
/// petite boucle mesurait à peine plus que son propre tube : 48 billboards
/// épais empilés sur un dixième de la distance, ce qui en blending additif
/// donne une **boule** et non une arche — signalé à l'écran. Une boucle
/// coronale réelle est dix à trente fois plus longue que large ; les fils les
/// plus fins descendent encore plus bas via `Boucle::epaisseur`.
const RAPPORT_TUBE: f32 = 0.06;

/// **Longueur sur laquelle un pied se fond dans la surface**, en part du
/// trajet.
///
/// Rend 0 au pied et 1 au-delà. Un ruban qui garde son éclat jusqu'au bout
/// s'arrête **net** sur la photosphère et se lit comme posé devant elle ;
/// un ruban qui s'éteint complètement n'a plus de pied du tout. Le fondu
/// partiel, plus la lueur au pied, donnent la lecture « ça sort de la
/// surface ». ⚠️ Réglage esthétique (`astres_v2_boucles.md` §5).
const ANCRAGE: f32 = 0.18;

fn ancrage(t: f32) -> f32 {
    let bord = t.min(1.0 - t).max(0.0);
    let u = (bord / ANCRAGE).clamp(0.0, 1.0);
    u * u * (3.0 - 2.0 * u)
}

/// Émet le brin courant en ruban et rend le nombre de quads produits, en
/// vidant le brin. Un brin de moins de deux points ne donne rien.
fn emettre(
    verts: &mut Vec<Vertex>,
    inds: &mut Vec<u16>,
    cam: &CameraInfo,
    brin: &mut Vec<crate::impostor::PointRuban>,
) -> usize {
    crate::impostor::push_ruban(verts, inds, cam, brin);
    let quads = brin.len().saturating_sub(1);
    brin.clear();
    quads
}

/// Sur-échantillonnage utilisé pour mesurer la courbe avant de la reparcourir
/// à pas constant. Quatre fois suffit : l'erreur d'une corde sur un arc décroît
/// comme le carré du pas.
/// **Évasement d une boucle avec la hauteur**, en part de son apex.
///
/// Il écarte les deux points de contrôle à mesure que la boucle monte, pour
/// que son sommet reste un arc et ne devienne pas une pointe. Réglage
/// esthétique, mais borné par une mesure : le virage entre deux points
/// consécutifs doit rester sous ~35°, au-delà le ruban montre un coin.
const EVASEMENT: f32 = 0.45;

const SUR_ECHANTILLON: usize = 4;

/// **Les points d'une boucle, espacés également LE LONG DE LA COURBE.**
///
/// ⚠️ Échantillonner uniformément en `t` est le pire choix pour une cubique
/// étirée. Mesuré sur une arche éjectée (apex 8) : le pas variait d'un
/// facteur **41** entre les jambes — presque droites, donc gavées de points —
/// et l'apex, où toute la courbure se concentre et où le trajet tournait de
/// **82° d'un point au suivant**. Un ruban qui vire de 82° en un segment
/// montre un **coin**, pas une courbe : c'est l'effet pointu signalé à
/// l'écran au sommet des grandes arches. Ajouter des points n'y changeait
/// rien — ils tombaient tous là où la courbe était déjà droite.
///
/// On mesure donc la courbe une fois, densément, puis on la reparcourt à pas
/// constant. L'indice du point devient sa **fraction de trajet**, ce qui est
/// aussi la bonne variable pour le galbe et le fondu d'ancrage.
fn le_long(a: Vec3, b: Vec3, p: Profil, rayon: f32, n: usize) -> Vec<Vec3> {
    let m = (n * SUR_ECHANTILLON).max(2);
    let brut: Vec<Vec3> =
        (0..=m).map(|i| point_boucle(a, b, p, i as f32 / m as f32, rayon)).collect();
    let mut cumul = Vec::with_capacity(brut.len());
    let mut total = 0.0;
    cumul.push(0.0);
    for w in brut.windows(2) {
        total += (w[1] - w[0]).length();
        cumul.push(total);
    }
    // `is_finite` autant que `> 0` : un trajet NaN traverserait la recherche
    // ci-dessous sans jamais avancer.
    if !total.is_finite() || total <= 0.0 {
        return vec![brut[0]; n + 1]; // boucle dégénérée : rien à parcourir
    }
    let mut sortie = Vec::with_capacity(n + 1);
    let mut j = 0;
    for i in 0..=n {
        let vise = total * i as f32 / n as f32;
        while j + 1 < cumul.len() - 1 && cumul[j + 1] < vise {
            j += 1;
        }
        let (c0, c1) = (cumul[j], cumul[j + 1]);
        let u = if c1 > c0 { (vise - c0) / (c1 - c0) } else { 0.0 };
        sortie.push(brut[j].lerp(brut[j + 1], u));
    }
    sortie
}

/// Étendue d'une boucle : la corde entre ses pieds, plus ce que sa hauteur
/// ajoute au trajet. Approximation grossière d'une longueur d'arc — elle sert
/// à proportionner, pas à mesurer.
fn etendue(a: Vec3, b: Vec3, apex: f32, rayon: f32) -> f32 {
    (b - a).length() * rayon + 1.6 * apex * rayon
}

/// **Calibre du tube et nombre de billboards** pour une boucle de cette
/// étendue.
///
/// Le nombre n'est pas fixe : il suit l'étendue, de sorte que l'espacement
/// entre billboards reste le même quelle que soit la taille de la boucle. À
/// 48 quads pour tout le monde, une petite boucle les recevait sur un
/// dixième de la distance — en blending additif, ça somme en **boule**.
/// ⚠️ **Deux étendues, et non une.** `portee` est celle qui fixe l'épaisseur,
/// `trajet` la distance réellement parcourue. Elles ne diffèrent que pendant
/// une **éjection** : l'apex y gonfle jusqu'à trois fois sa valeur d'arrivée,
/// et un tube indexé dessus finissait large comme un dixième de l'étoile. Une
/// corde de flux qui s'éjecte **s'étire**, elle ne s'épaissit pas — d'où une
/// épaisseur figée à la portée de l'arche, et un nombre de billboards qui,
/// lui, suit le trajet pour que la bande reste dense en s'allongeant.
fn tube(trajet: f32, portee: f32, epaisseur: f32) -> (f32, usize) {
    let calibre = portee * RAPPORT_TUBE * epaisseur.max(0.01);
    let n = ((trajet / (calibre * 0.6).max(1e-6)).ceil() as usize).clamp(8, 64);
    (calibre, n)
}

/// **Galbe du tube le long de la boucle** : pieds serrés, ventre plus large.
///
/// Un tube de flux s'évase en s'élevant, la pression du plasma environnant
/// chutant avec l'altitude. Sans ce galbe, une boucle est un cylindre de
/// billboards tous identiques — le « chapelet » qu'on lit à l'écran. La
/// moyenne reste proche de 1 : le galbe redistribue l'épaisseur, il ne
/// l'augmente pas.
fn galbe(t: f32) -> f32 {
    0.72 + 0.56 * (std::f32::consts::PI * t.clamp(0.0, 1.0)).sin()
}

/// Un point de la boucle, en `t ∈ [0 ; 1]` d'un pied à l'autre.
///
/// ⚠️ **Cubique, et non quadratique.** Une Bézier à un seul point de
/// contrôle ne sait dessiner qu'une **parabole** : toutes les boucles
/// avaient donc exactement la même forme, à l'échelle près — « ça fait très
/// fonction du second degré », signalé à l'écran. Deux points de contrôle
/// permettent un col pincé ou un dôme évasé, et une inclinaison hors du plan
/// radial. Même coût : un polynôme évalué par point.
///
/// Le facteur `4/3` conserve le sens d'`apex` : au sommet (`t = 0,5`), la
/// cubique atteint `3·(h₁ + h₂)/8`, soit exactement `apex × rayon`.
fn point_boucle(a: Vec3, b: Vec3, p: Profil, t: f32, rayon: f32) -> Vec3 {
    let pa = a * rayon;
    let pb = b * rayon;
    let mid = (pa + pb) * 0.5;
    let outward = mid.normalize_or_zero();
    let along = (pb - pa).normalize_or_zero();
    let lateral = outward.cross(along);
    let corde = (pb - pa).length();

    let haut = outward * (4.0 / 3.0 * p.apex * rayon);
    let penche = lateral * (p.inclinaison * p.apex * rayon);
    let col = along * (p.pincee * corde);
    let biais = along * (p.skew * rayon);
    // ⚠️ **Une boucle s'évase en montant.** Sans ce terme, les deux points de
    // contrôle grimpent côte à côte : mesuré sur une arche éjectée à huit
    // rayons, ils culminaient à 10,7 unités en restant à 0,3 l'un de l'autre,
    // et la courbe y faisait demi-tour en **157°** — une pointe, pas un
    // sommet. C'est l'effet signalé à l'écran, et aucun échantillonnage ne
    // pouvait le corriger : la pointe était dans la géométrie. Une corde de
    // flux réelle se dilate en s'élevant, la pression du plasma chutant avec
    // l'altitude ; ses pieds, eux, restent ancrés.
    let evase = along * (EVASEMENT * p.apex * rayon);
    let c1 = pa + haut + col - evase + biais + penche;
    let c2 = pb + haut - col + evase + biais + penche;

    let u = 1.0 - t;
    pa * (u * u * u) + c1 * (3.0 * u * u * t) + c2 * (3.0 * u * t * t) + pb * (t * t * t)
}

/// Vrai si le point `p` est masqué par la sphère (centre `c`, rayon `r`) vu depuis `cam`.
fn cache_par_sphere(p: Vec3, c: Vec3, r: f32, cam: Vec3) -> bool {
    let axe = (cam - c).normalize_or_zero(); // du centre vers la caméra
    let rel = p - c;
    let along = rel.dot(axe);
    if along >= 0.0 {
        return false; // côté caméra -> visible
    }
    let perp = rel - axe * along; // distance à l'axe de visée
    perp.length() < r * 0.98 // dans la silhouette ET derrière -> caché
}

fn flush(verts: &mut Vec<Vertex>, inds: &mut Vec<u16>, tex: &Texture2D) {
    if inds.is_empty() {
        return;
    }
    let mesh = Mesh {
        vertices: std::mem::take(verts),
        indices: std::mem::take(inds),
        texture: Some(tex.clone()),
    };
    draw_mesh(&mesh);
    let mut v = mesh.vertices;
    v.clear();
    let mut i = mesh.indices;
    i.clear();
    *verts = v;
    *inds = i;
}

/// ⚠️ **Le générateur a déménagé dans `impostor`**, à côté de `push_ruban` : sa
/// doc y expliquait déjà que les UV d'un ruban traversent cette texture par son
/// milieu pour donner le profil d'un tube. La queue d'une comète (lot 9.2) en a
/// besoin du même, et deux générateurs d'un même dégradé auraient fini par ne
/// plus se ressembler.
pub(super) fn texture_halo(taille: usize) -> Texture2D {
    crate::impostor::texture_halo(taille)
}

#[cfg(test)]
mod tests_profil_boucle {
    use super::*;

    fn plat() -> Profil {
        Profil { apex: 0.4, skew: 0.0, pincee: 0.0, inclinaison: 0.0 }
    }

    /// **La courbe part d'un pied, arrive à l'autre, et culmine à `apex`.**
    /// Le passage de la quadratique à la cubique a introduit un facteur
    /// `4/3` sur les points de contrôle : s'il dérive, toutes les boucles
    /// changent silencieusement de hauteur.
    #[test]
    fn la_courbe_relie_les_pieds_et_culmine_a_l_apex() {
        let (a, b) = (vec3(1.0, 0.2, 0.0).normalize(), vec3(1.0, -0.2, 0.0).normalize());
        let r = 2.0;
        assert!((point_boucle(a, b, plat(), 0.0, r) - a * r).length() < 1e-4, "ne part pas du pied");
        assert!((point_boucle(a, b, plat(), 1.0, r) - b * r).length() < 1e-4, "n'arrive pas au pied");
        let sommet = point_boucle(a, b, plat(), 0.5, r);
        let mid = (a * r + b * r) * 0.5;
        let hauteur = (sommet - mid).length();
        assert!(
            (hauteur - plat().apex * r).abs() < 1e-3,
            "sommet à {hauteur} au lieu de {}",
            plat().apex * r
        );
    }

    /// **Aucune boucle ne montre de coin, même éjectée à huit rayons.**
    ///
    /// C'est le défaut signalé à l'écran — une pointe au sommet des grandes
    /// arches — et il tenait à deux causes que ce test mesure d'un coup.
    ///
    /// 1. **L'échantillonnage était uniforme en `t`**, ce qui est le pire
    ///    choix pour une cubique étirée : mesuré, le pas variait d'un facteur
    ///    **41** entre les jambes presque droites, gavées de points, et
    ///    l'apex qui les concentrait toute la courbure.
    /// 2. **La courbe avait une vraie pointe** : les deux points de contrôle
    ///    montaient côte à côte, et le trajet y virait de **157°**. Aucun
    ///    échantillonnage ne corrige ça — d'où [`EVASEMENT`].
    #[test]
    fn aucune_boucle_ne_montre_de_coin() {
        let (a, b) = (vec3(1.0, 0.25, 0.0).normalize(), vec3(1.0, -0.25, 0.0).normalize());
        let r = 1.0;
        // Jusqu'à 8 : la borne au-delà de laquelle `eruptions` retire une
        // boucle éjectée.
        for apex in [0.2_f32, 0.5, 1.0, 2.0, 4.0, 6.0, 8.0] {
            let p = Profil { apex, skew: 0.1, pincee: 0.2, inclinaison: 0.2 };
            let (_, n) = tube(etendue(a, b, apex, r), etendue(a, b, apex.min(1.0), r), 1.0);
            let pts = le_long(a, b, p, r, n);
            let mut virage: f32 = 0.0;
            let (mut court, mut long) = (f32::INFINITY, 0.0_f32);
            for i in 1..pts.len() - 1 {
                let t0 = (pts[i] - pts[i - 1]).normalize_or_zero();
                let t1 = (pts[i + 1] - pts[i]).normalize_or_zero();
                virage = virage.max(t0.dot(t1).clamp(-1.0, 1.0).acos());
                let l = (pts[i + 1] - pts[i]).length();
                court = court.min(l);
                long = long.max(l);
            }
            assert!(
                virage.to_degrees() < 35.0,
                "apex {apex} : le trajet vire de {:.0}° d'un point au suivant — c'est un coin",
                virage.to_degrees()
            );
            assert!(
                long / court.max(1e-6) < 1.3,
                "apex {apex} : le pas varie d'un facteur {:.1} — l'apex est sous-échantillonné",
                long / court.max(1e-6)
            );
        }
    }

    /// **Le pied se fond dans la surface, sans disparaître.**
    ///
    /// Un ruban qui garde tout son éclat jusqu'au bout s'arrête net sur la
    /// photosphère et se lit comme posé devant elle ; un ruban qui s'éteint
    /// complètement n'a plus de pied. Le fondu est donc partiel, et
    /// symétrique — les deux pieds sont des pieds.
    #[test]
    fn le_pied_se_fond_dans_la_surface_sans_disparaitre() {
        assert_eq!(ancrage(0.0), 0.0, "le pied ne se fond pas du tout");
        assert_eq!(ancrage(1.0), 0.0, "le second pied n'est pas traité comme un pied");
        assert!((ancrage(0.5) - 1.0).abs() < 1e-6, "le sommet est encore en fondu");
        // Symétrique, et le fondu est bien fini avant le milieu : sinon toute
        // l'arche serait pâle, pas seulement ses extrémités.
        for t in [0.02_f32, 0.1, 0.3, 0.45] {
            assert!((ancrage(t) - ancrage(1.0 - t)).abs() < 1e-6, "asymétrique en {t}");
        }
        assert!((ancrage(ANCRAGE) - 1.0).abs() < 1e-6, "le fondu dépasse sa portée");
        assert!(ancrage(ANCRAGE * 0.5) < 0.9, "le fondu est trop brusque pour se voir");
        // Hors bornes : la courbe reste bornée, jamais négative.
        for t in [-1.0_f32, 0.0, 1.0, 2.0] {
            assert!((0.0..=1.0).contains(&ancrage(t)), "{t} -> {}", ancrage(t));
        }
    }

    /// **Une petite boucle a un petit tube, et pas plus de billboards qu'il
    /// n'en faut.**
    ///
    /// L'épaisseur était indexée sur le rayon de l'**étoile** : une petite
    /// boucle mesurait à peine plus que son propre tube, et recevait les 48
    /// mêmes quads sur un dixième de la distance. En blending additif, ça
    /// somme en **boule** au lieu de dessiner une arche — signalé à l'écran.
    #[test]
    fn une_petite_boucle_a_un_tube_a_sa_mesure() {
        let (gros, n_gros) = tube(1.2, 1.2, 1.0);
        let (petit, n_petit) = tube(0.12, 0.12, 1.0);
        assert!(
            petit < gros * 0.2,
            "une boucle dix fois plus courte garde un tube de {petit} contre {gros}"
        );
        // L'élancement est ce qui se conserve : c'est lui qui fait qu'on lit
        // un tube et non une boule.
        assert!((gros / 1.2 - petit / 0.12).abs() < 1e-6, "l'élancement n'est pas conservé");
        // Et l'espacement des billboards aussi, donc la densité lumineuse.
        assert_eq!(n_gros, n_petit, "la petite boucle est plus dense que la grande");
        assert!((8..=64).contains(&n_petit), "{n_petit} billboards");
    }

    /// **Une corde de flux qui s'éjecte s'étire, elle ne s'épaissit pas.**
    ///
    /// Pendant une éjection l'apex gonfle jusqu'à trois fois sa valeur
    /// d'arrivée. Indexer l'épaisseur dessus donnait un tube large comme un
    /// dixième de l'étoile au bout de sa course — une masse informe là où il
    /// fallait un filament étiré.
    #[test]
    fn une_ejection_etire_le_tube_au_lieu_de_l_epaissir() {
        let (pose, _) = tube(1.0, 1.0, 1.0);
        // Même arche, apex trois fois plus haut : le trajet a triplé, la
        // portée non.
        let (ejecte, n_ejecte) = tube(3.0, 1.0, 1.0);
        assert_eq!(ejecte, pose, "le tube s'est épaissi en s'éjectant");
        let (_, n_pose) = tube(1.0, 1.0, 1.0);
        assert!(
            n_ejecte > n_pose,
            "le trajet a triplé sans un billboard de plus : la bande se clairsème"
        );
    }

    /// **Un fil reste un fil, un ruban reste un ruban**, et un calibre
    /// aberrant ne fait pas exploser le nombre de quads.
    #[test]
    fn l_epaisseur_module_le_tube_sans_l_affoler() {
        let (fil, n_fil) = tube(1.0, 1.0, 0.45);
        let (ruban, n_ruban) = tube(1.0, 1.0, 2.1);
        assert!(fil < ruban, "le fil n'est pas plus fin que le ruban");
        assert!(n_fil > n_ruban, "un tube plus fin demande plus de billboards");
        for epaisseur in [0.0_f32, 1e-9, 1.0, 50.0] {
            let (c, n) = tube(1.0, 1.0, epaisseur);
            assert!(c > 0.0 && c.is_finite(), "calibre {c} pour epaisseur {epaisseur}");
            assert!((8..=64).contains(&n), "{n} billboards pour epaisseur {epaisseur}");
        }
        // Étendue nulle (deux pieds confondus) : rien ne doit paniquer.
        let (c, n) = tube(0.0, 0.0, 1.0);
        assert!(c.is_finite() && (8..=64).contains(&n), "étendue nulle -> {c}, {n}");
    }

    /// **Le tube est plus fin à ses pieds qu'à son ventre**, symétriquement,
    /// et sans gonfler l'épaisseur moyenne — sinon toutes les boucles
    /// grossiraient d'un coup.
    #[test]
    fn le_galbe_amincit_les_pieds_sans_gonfler_la_boucle() {
        assert!(galbe(0.0) < galbe(0.5), "le tube n'est pas plus fin à son pied");
        assert!((galbe(0.0) - galbe(1.0)).abs() < 1e-6, "les deux pieds diffèrent");
        assert!(galbe(0.25) < galbe(0.5) && galbe(0.25) > galbe(0.0), "le galbe n'est pas régulier");
        let moyenne: f32 = (0..=100).map(|i| galbe(i as f32 / 100.0)).sum::<f32>() / 101.0;
        assert!((moyenne - 1.0).abs() < 0.12, "épaisseur moyenne {moyenne} : le galbe n'est pas neutre");
        // Et rien ne part à zéro : un tube d'épaisseur nulle disparaît.
        for i in 0..=20 {
            assert!(galbe(i as f32 / 20.0) > 0.5);
        }
    }

    /// **Deux boucles de même portée et de même hauteur ne se ressemblent
    /// pas.** C'est tout l'objet de la cubique : une quadratique n'a qu'un
    /// point de contrôle, donc ne sait tracer qu'une parabole, et toutes les
    /// boucles avaient la même forme à l'échelle près.
    #[test]
    fn le_pincement_et_l_inclinaison_changent_la_silhouette() {
        let (a, b) = (vec3(1.0, 0.25, 0.0).normalize(), vec3(1.0, -0.25, 0.0).normalize());
        let etroite = Profil { pincee: 0.3, ..plat() };
        let evasee = Profil { pincee: -0.2, ..plat() };
        let penchee = Profil { inclinaison: 0.5, ..plat() };
        // Au quart du parcours, là où la silhouette se joue.
        let p0 = point_boucle(a, b, plat(), 0.25, 1.0);
        for (autre, nom) in
            [(etroite, "pincée"), (evasee, "évasée"), (penchee, "penchée")]
        {
            let p = point_boucle(a, b, autre, 0.25, 1.0);
            assert!((p - p0).length() > 0.01, "la boucle {nom} suit la même courbe que la plate");
        }
        // L'inclinaison sort bien du plan radial, les autres non.
        let normale = a.cross(b).normalize();
        assert!(point_boucle(a, b, penchee, 0.5, 1.0).dot(normale).abs() > 0.01);
        assert!(point_boucle(a, b, etroite, 0.5, 1.0).dot(normale).abs() < 1e-4);
    }
}

#[cfg(test)]
mod tests_plancher_plasma {
    use super::*;

    /// Hauteur de fenêtre d'essai (celle du jeu, `main.rs`).
    const VH: f32 = 700.0;
    /// Taille d'un billboard de boucle pour une étoile de rayon solaire :
    /// `r × 0,05`, la valeur du code de dessin.
    const TAILLE: f32 = 1.116 * 0.05;

    /// **Le pinceau ne descend plus sous le pixel.** C'est tout l'objet du
    /// lot : l'éjection avait une géométrie large de 24 px et un pinceau de
    /// 0,20 px — le rasterizer l'avalait, donc elle n'existait pas.
    #[test]
    fn un_billboard_trop_fin_est_ramene_au_plancher() {
        let dist = crate::camera::DIST_DEFAUT;
        let (planchee, fondu) = plancher_plasma(TAILLE, dist, VH);
        assert!(planchee > TAILLE, "rien n'a été planchéié : {planchee} vs {TAILLE}");
        let px = crate::impostor::px_rayon(planchee, dist, VH);
        assert!(px >= PLASMA_PX_MIN - 1e-3, "toujours sous le plancher : {px} px");
        assert!(fondu < 1.0, "agrandi sans pâlir : de la matière créée de rien");
    }

    /// **De près, on ne touche à rien.** Un plancher qui s'appliquerait aussi
    /// à un billboard déjà gros gonflerait le plasma en approche — l'inverse
    /// du but.
    #[test]
    fn un_billboard_deja_visible_n_est_pas_touche() {
        let (planchee, fondu) = plancher_plasma(TAILLE, 5.0, VH);
        assert_eq!(planchee, TAILLE, "taille modifiée alors qu'elle suffisait");
        assert_eq!(fondu, 1.0, "éclat modifié alors que rien n'a été agrandi");
    }

    /// **Ce qu'on rend en taille, on le retire en éclat.** Le plasma est
    /// dessiné en blending **additif** : agrandir sans pâlir rendrait une
    /// éjection lointaine plus brillante que la même vue de près.
    #[test]
    fn le_fondu_compense_la_surface_gagnee() {
        for dist in [200.0_f32, 600.0, crate::camera::DIST_DEFAUT] {
            let (planchee, fondu) = plancher_plasma(TAILLE, dist, VH);
            let rapport = TAILLE / planchee;
            let attendu = rapport * rapport;
            assert!(
                (fondu - attendu).abs() < 1e-3 || fondu == 0.12,
                "dist {dist} : fondu {fondu}, attendu {attendu}"
            );
        }
        // Et il ne s'éteint jamais tout à fait : un fil de lumière reste plus
        // lisible que rien du tout.
        let (_, tres_loin) = plancher_plasma(TAILLE, 30000.0, VH);
        assert!(tres_loin >= 0.12, "le plasma s'est éteint : {tres_loin}");
    }
}

#[cfg(test)]
mod tests_jets {
    use super::*;

    /// **Les jets sortent par les pôles.** L'axe de rotation incliné sur celui
    /// du disque amène le pôle nord de la surface **exactement** sur l'axe des
    /// jets, à tout instant de la rotation : sinon les jets partiraient des
    /// flancs de l'étoile, comme signalé à l'écran.
    #[test]
    fn les_jets_sortent_par_les_poles() {
        for graine in [0.0_f32, 3.3, 517.2] {
            let axe = super::super::normale_disque(graine);
            let q = super::super::quat_axe(axe);
            for a in [0.0_f32, 1.0, 4.4] {
                let pole = vers_monde(Vec3::Y, a, q);
                assert!((pole - axe).length() < 1e-4, "graine {graine}, rotation {a} : pôle {pole} hors de l'axe {axe}");
            }
        }
        // Et sans inclinaison, rien ne change : le pôle reste en haut.
        assert!((vers_monde(Vec3::Y, 2.0, Quat::IDENTITY) - Vec3::Y).length() < 1e-5);
    }


    /// **Les jets suivent l'axe du disque** — c'est tout l'objet du lot : peints
    /// sur l'écran, ils partaient de travers dès que le disque s'inclinait. Et
    /// ils partent **de la surface**, vers le large, en s'évasant.
    #[test]
    fn les_jets_suivent_l_axe_du_disque() {
        for graine in [0.0_f32, 3.3, 517.2] {
            let axe = super::super::normale_disque(graine);
            let c = vec3(5.0, -2.0, 1.0);
            let (r, longueur) = (0.9, 3.0);
            let pts = jet(c, axe, r, longueur, Vec3::ONE);
            for p in &pts {
                let rel = p.0 - c;
                let le_long = rel.dot(axe);
                assert!(le_long > r, "graine {graine} : un point du jet est dans l'étoile");
                assert!((rel - axe * le_long).length() < 1e-3, "graine {graine} : le jet quitte l'axe du disque");
            }
            assert!(pts.last().unwrap().1 > pts[0].1 * 2.0, "le jet ne s'évase pas");
            assert!(((pts.last().unwrap().0 - c).length() - r * (1.02 + longueur)).abs() < 1e-3);
        }
    }
}
