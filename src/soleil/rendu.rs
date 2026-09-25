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
        let rot = self.temps * 0.12;

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
            mat.set_uniform_array("spots", &spots[..]);

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
        let (taille_l, fondu_l) = plancher_plasma(r * 0.05, dist, vh);
        for b in &self.boucles {
            let a_w = vers_monde(b.sa, rot);
            let b_w = vers_monde(b.sb, rot);
            let n = 48;
            for i in 0..=n {
                let t = i as f32 / n as f32;
                let pos = point_boucle(a_w, b_w, profil(b), t, r);
                let monde = c + pos;
                if cache_par_sphere(monde, c, r, cam.pos) {
                    continue; // derrière le soleil -> pas visible à travers
                }
                // onde de matière qui circule + intensité de la boucle
                let flow = 0.55 + 0.45 * (t * 16.0 - self.temps * 6.0).sin();
                let lum = (b.intensite * (0.55 + 0.45 * flow)).clamp(0.0, 1.0);
                // plasma chaud, additif -> les recouvrements forment un cœur blanc
                let col = Color::new(1.0, 0.8 + 0.15 * lum, 0.5 + 0.3 * lum, lum * fondu_l);
                push_quad(&mut self.verts, &mut self.inds, cam, monde, taille_l, col);
                quads += 1;
                if quads >= QUADS_PAR_LOT {
                    flush(&mut self.verts, &mut self.inds, &self.halo);
                    quads = 0;
                }
            }
        }

        // Cœur dense brillant au sommet des éjections (matière de proéminence).
        for b in &self.boucles {
            if b.etat == EtatBoucle::Rupture {
                let a_w = vers_monde(b.sa, rot);
                let b_w = vers_monde(b.sb, rot);
                let pos = c + point_boucle(a_w, b_w, profil(b), 0.5, r);
                if !cache_par_sphere(pos, c, r, cam.pos) {
                    let (taille_c, fondu_c) = plancher_plasma(r * 0.16, dist, vh);
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
        flush(&mut self.verts, &mut self.inds, &self.halo);
        gl_use_default_material();
    }
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
fn vers_monde(s: Vec3, a: f32) -> Vec3 {
    let (sa, ca) = a.sin_cos();
    vec3(s.x * ca + s.z * sa, s.y, -s.x * sa + s.z * ca)
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
    let c1 = pa + haut + col + biais + penche;
    let c2 = pb + haut - col + biais + penche;

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

pub(super) fn texture_halo(taille: usize) -> Texture2D {
    let mut bytes = vec![0u8; taille * taille * 4];
    let centre = (taille as f32 - 1.0) / 2.0;
    for y in 0..taille {
        for x in 0..taille {
            let dx = (x as f32 - centre) / centre;
            let dy = (y as f32 - centre) / centre;
            let a = (1.0 - (dx * dx + dy * dy).sqrt()).clamp(0.0, 1.0);
            let a = a * a;
            let i = (y * taille + x) * 4;
            bytes[i] = 255;
            bytes[i + 1] = 255;
            bytes[i + 2] = 255;
            bytes[i + 3] = (a * 255.0) as u8;
        }
    }
    let t = Texture2D::from_rgba8(taille as u16, taille as u16, &bytes);
    t.set_filter(FilterMode::Linear);
    t
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
