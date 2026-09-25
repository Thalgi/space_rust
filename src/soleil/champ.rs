//! **Le champ magnétique d'une étoile** (chantier astres, sous-lot 3.1 —
//! `docs/conception/astres_v2.md` §6 bis, et `astres_v2_champ_partage.md`).
//!
//! ## Pourquoi il existe
//!
//! Jusqu'ici, **rien ne disait où le champ était fort**. Les taches se
//! posaient donc au hasard sur la sphère (`eruptions.rs`, `dir_aleatoire`) et
//! les arches ailleurs : deux systèmes sans rapport, d'où des taches qui ont
//! l'air d'**autocollants** posés sur la surface et des boucles qui ne
//! partent de nulle part. Ce n'était pas un défaut de forme — c'était une
//! absence de source commune.
//!
//! Ce module est cette source. Il ne dessine rien et ne décide de rien : il
//! répond à une seule question, « quelle est l'intensité du champ dans cette
//! direction ». Les sous-lots suivants s'en servent — les taches naissent sur
//! ses maxima (3.2), les pieds d'une arche sur deux polarités opposées (3.3),
//! et le shader l'utilisera pour assombrir la surface (3.4).
//!
//! ## Ce qu'il modélise, et ce qu'il ne modélise pas
//!
//! Des **paires** de pôles de polarités opposées, posées côte à côte : c'est
//! la forme minimale qui produise une **région active bipolaire**, avec sa
//! ligne neutre entre les deux — la structure dont découlent, dans la
//! réalité, les groupes de taches et les arcades de boucles
//! (`astres_v2_recherche_visuelle.md`).
//!
//! Ce n'est pas une simulation : pas de lignes de champ intégrées, pas de
//! dipôle exact, pas d'évolution. Une somme de noyaux, bornée et sans
//! singularité, qu'on peut interroger des milliers de fois par frame.

use crate::planete::terrain::Rng;
use macroquad::prelude::*;

/// Nombre de **paires** de pôles par étoile.
///
/// Trois régions actives : assez pour que la surface ne se résume pas à deux
/// taches figées, assez peu pour qu'elles restent distinctes et que le champ
/// garde des creux entre elles. ⚠️ Ce chiffre est un **réglage esthétique**,
/// il ne se teste pas — il se juge à l'œil au-delà d'environ 10 px de rayon
/// stellaire (`astres_v2.md` §6 bis, risque 1).
pub(super) const NB_PAIRES: usize = 3;

/// Écartement angulaire des deux pôles d'une paire, en radians.
///
/// C'est lui qui fixe la **taille d'une région active** : trop serré, les
/// deux polarités s'annulent et il ne reste rien ; trop large, la paire
/// cesse de se lire comme un groupe et donne deux taches sans rapport.
const ECART_PAIRE: f32 = 0.28;

/// Portée angulaire d'un pôle : au-delà, sa contribution est négligeable.
const PORTEE: f32 = 0.42;

/// **Champ minimal pour qu'une tache s'y forme.**
///
/// Une tache est de la convection **bloquée** : sous un certain champ, rien
/// ne bloque rien et la surface reste granuleuse. Le seuil sert surtout de
/// garde-fou — les paires étant posées au hasard, celle d'une région peut
/// tomber près d'un pôle opposé d'une autre et s'annuler avec lui. Sans lui,
/// un groupe naissait dans un champ **plus faible que la moyenne de la
/// sphère** : une tache à nouveau posée au hasard, exactement ce que ce
/// sous-lot corrige.
const CHAMP_MIN: f32 = 0.35;

/// Un pôle magnétique, posé sur la sphère.
#[derive(Clone, Copy, Debug)]
pub(super) struct Pole {
    /// Direction unitaire depuis le centre de l'étoile.
    pub dir: Vec3,
    /// **Force signée** : le signe est la polarité. Deux pôles de signes
    /// opposés forment une région active, et c'est entre eux que les arches
    /// s'ancreront (sous-lot 3.3).
    pub force: f32,
}

/// Les pôles d'une étoile, **dérivés de sa graine**.
///
/// ⚠️ **Dérivés, jamais tirés.** Un `gen_range` de plus dans `Soleil::new`
/// consommerait le RNG **global** de macroquad, décalant tout ce qui est tiré
/// ensuite : à graine égale, le système entier changerait — planètes
/// comprises. Le dépôt a déjà rencontré ce piège (`genese/mod.rs`, la
/// multiplicité tirée du seed « pour laisser la séquence intacte »). D'où un
/// générateur déterministe local, semé par la graine de l'étoile.
pub(super) fn poles(graine: f32) -> Vec<Pole> {
    let mut rng = Rng::new(graine.to_bits() as u64);
    let mut out = Vec::with_capacity(NB_PAIRES * 2);
    for _ in 0..NB_PAIRES {
        let centre = direction(&mut rng);
        // Une direction tangente quelconque, pour écarter les deux polarités
        // de part et d'autre du centre de la région active.
        let axe = tangente(centre, &mut rng);
        let force = 0.6 + 0.4 * rng.f32();
        for signe in [1.0_f32, -1.0] {
            let d = (centre + axe * (signe * ECART_PAIRE)).normalize_or(centre);
            out.push(Pole { dir: d, force: force * signe });
        }
    }
    out
}

/// **Intensité signée du champ** dans la direction `d` (unitaire).
///
/// Somme de noyaux en cloche, un par pôle : proche de sa force au pôle,
/// nulle loin de tous. Le **signe** dit la polarité dominante, ce dont les
/// pieds d'arche auront besoin ; sa **valeur absolue** dit où la convection
/// est bloquée, donc où une tache peut naître.
pub(super) fn champ(poles: &[Pole], d: Vec3) -> f32 {
    poles
        .iter()
        .map(|p| {
            let ang = p.dir.dot(d).clamp(-1.0, 1.0).acos();
            let t = ang / PORTEE;
            p.force * (-t * t).exp()
        })
        .sum()
}

/// **Où une tache peut naître**, et sa taille relative.
///
/// `poids` vaut `1` pour la plus forte du groupe et décroît pour les autres :
/// c'est l'intensité du champ au site, normalisée. L'appelant en tire la
/// taille et la durée de vie — une tache principale est grosse et dure.
pub(super) struct Site {
    pub dir: Vec3,
    pub poids: f32,
}

/// **Le groupe de taches d'une région active**, trié de la principale à la
/// plus petite.
///
/// Une tache ne naît plus au hasard (`eruptions::dir_aleatoire`) et ne naît
/// plus seule : une région active porte une **tache de tête** sur une
/// polarité, une **suiveuse** sur l'autre, et une petite accolée à la
/// première — la forme la plus simple d'un groupe bipolaire réel
/// (`astres_v2_recherche_visuelle.md`). Les trois tailles sortent du champ
/// lui-même, pas d'un tirage : c'est ce qui les rend inégales *et* cohérentes
/// avec l'endroit où elles se posent.
///
/// Rend une liste vide si la paire n'existe pas — l'appelant garde alors son
/// comportement d'avant.
pub(super) fn groupe(poles: &[Pole], paire: usize) -> Vec<Site> {
    let (a, b) = match (poles.get(paire * 2), poles.get(paire * 2 + 1)) {
        (Some(a), Some(b)) => (a, b),
        _ => return Vec::new(),
    };
    // La troisième penche nettement vers `a` : posée entre les deux, elle
    // tomberait sur la ligne neutre, là où le champ s'annule — justement là où
    // une tache ne se forme pas.
    let accolee = (a.dir * 0.72 + b.dir * 0.28).normalize_or(a.dir);
    let mut sites: Vec<Site> = [a.dir, b.dir, accolee]
        .into_iter()
        .map(|dir| Site { dir, poids: champ(poles, dir).abs() })
        .filter(|s| s.poids >= CHAMP_MIN)
        .collect();
    let fort = sites.iter().map(|s| s.poids).fold(0.0_f32, f32::max);
    if fort <= 0.0 {
        return Vec::new(); // région noyée par une voisine : rien n'y naît
    }
    for s in &mut sites {
        s.poids /= fort;
    }
    sites.sort_by(|x, y| y.poids.total_cmp(&x.poids));
    sites
}

/// **Part du rayon d'une tache sur laquelle son pied peut coulisser.**
///
/// Les vraies boucles coronales ne rayonnent pas d'un point : elles s'alignent
/// **côte à côte** le long de la ligne neutre d'une région active, en arcade
/// (`astres_v2_recherche_visuelle.md`). Reste à dire *sur quelle longueur*.
///
/// ⚠️ La première version mesurait cet étalement contre la **région**
/// (0,34 rad, l'échelle de l'écart entre deux pôles). Mesuré ensuite :
/// **100 % des pieds tombaient hors de leur tache**, à 1,26 rayon du centre
/// en médiane — signalé à l'écran, « les boucles partent de zones externes
/// aux taches ». Une tache plafonne à ~0,26 rad de rayon et en vaut souvent
/// 0,15 : l'arcade se mesure en **rayons de tache**, jamais en écart de
/// pôles.
const PART_PIED: f32 = 0.55;

/// Les deux pieds d'une arche, chacun sur une polarité de la même région.
pub(super) struct Pieds {
    pub a: Vec3,
    pub b: Vec3,
}

/// **Où ancrer une arche**, entre deux taches de polarités opposées.
///
/// Chaque ancre est une tache : sa direction et son **rayon angulaire**. Une
/// arche relie deux polarités opposées — elle enjambe la ligne neutre. C'est
/// ce qui manquait : les pieds étaient posés de part et d'autre du centre
/// d'une seule tache, sur une tangente **tirée au hasard**, et toutes les
/// arches partaient donc du même point en éventail.
///
/// `glissement` fait coulisser chaque pied dans `[-1 ; 1]`, le long de la
/// ligne neutre et **à l'intérieur de sa propre tache** ([`PART_PIED`]). Deux
/// valeurs différentes cisaillent l'arche : c'est de là que vient la variété
/// de silhouette, maintenant que l'écartement des pieds est celui des taches
/// et non plus un tirage.
///
/// Rend `None` si les deux ancres sont confondues — il n'y aurait rien à
/// enjamber, et aucune ligne neutre à suivre.
pub(super) fn pieds(a: (Vec3, f32), b: (Vec3, f32), glissement: (f32, f32)) -> Option<Pieds> {
    let centre = (a.0 + b.0).try_normalize()?;
    let axe = (b.0 - a.0).try_normalize()?;
    // La ligne neutre court perpendiculairement à l'axe qui joint les deux.
    // Pas de garde ici : `a + b` et `b - a` sont orthogonaux dès que `a` et
    // `b` sont unitaires (leur produit scalaire vaut |b|² - |a|²), donc leur
    // produit vectoriel est déjà unitaire — les deux `try_normalize` au-dessus
    // couvrent les seuls cas dégénérés, ancres confondues ou opposées.
    let neutre = centre.cross(axe);
    let glisser = |(d, rayon): (Vec3, f32), t: f32| {
        (d + neutre * (t.clamp(-1.0, 1.0) * rayon * PART_PIED)).normalize_or(d)
    };
    Some(Pieds { a: glisser(a, glissement.0), b: glisser(b, glissement.1) })
}

/// **À quelle région active appartient cette direction** — celle du pôle le
/// plus proche. Sert à retrouver le groupe d'une tache pour y ancrer une
/// arche : la tache et l'arche cessent ainsi d'être décidées séparément.
pub(super) fn paire_de(poles: &[Pole], d: Vec3) -> Option<usize> {
    (0..poles.len())
        .max_by(|i, j| poles[*i].dir.dot(d).total_cmp(&poles[*j].dir.dot(d)))
        .map(|i| i / 2)
}

/// Direction unitaire uniformément répartie sur la sphère.
///
/// ⚠️ La latitude passe par `asin(2u-1)` et non par un angle uniforme : sans
/// ça, les tirages s'accumulent aux pôles géographiques, et toutes les
/// régions actives d'une étoile naîtraient en haut et en bas.
fn direction(rng: &mut Rng) -> Vec3 {
    let lon = rng.f32() * std::f32::consts::TAU;
    let lat = (2.0 * rng.f32() - 1.0).clamp(-1.0, 1.0).asin();
    let (sl, cl) = lat.sin_cos();
    vec3(cl * lon.cos(), sl, cl * lon.sin())
}

/// Une direction unitaire **perpendiculaire** à `d`, orientée au hasard
/// autour d'elle.
fn tangente(d: Vec3, rng: &mut Rng) -> Vec3 {
    // Deux références pour éviter le produit vectoriel dégénéré quand `d`
    // frôle l'axe choisi — le même piège que `camera::base_camera`.
    let ref_ = if d.y.abs() > 0.9 { Vec3::X } else { Vec3::Y };
    let u = d.cross(ref_).normalize_or(Vec3::X);
    let v = d.cross(u).normalize_or(Vec3::Z);
    let a = rng.f32() * std::f32::consts::TAU;
    (u * a.cos() + v * a.sin()).normalize_or(u)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **Même graine, même champ.** C'est la promesse du dépôt : un système
    /// se reconstruit à l'identique (`bucketlist_globale.md` §1). Une étoile
    /// dont les régions actives changeraient à chaque reconstruction
    /// casserait aussi le hot-reload des shaders, qui rebâtit le système.
    #[test]
    fn les_poles_ne_dependent_que_de_la_graine() {
        let a = poles(123.456);
        let b = poles(123.456);
        assert_eq!(a.len(), b.len());
        for (p, q) in a.iter().zip(b.iter()) {
            assert_eq!(p.dir, q.dir);
            assert_eq!(p.force, q.force);
        }
        // Et deux étoiles différentes ne portent pas la même carte.
        let autre = poles(987.2);
        assert!(
            a.iter().zip(autre.iter()).any(|(p, q)| p.dir != q.dir),
            "deux graines donnent les mêmes pôles : les étoiles resteraient jumelles"
        );
    }

    /// **Les pôles vont par paires de polarités opposées, et proches.**
    ///
    /// C'est toute la structure : une région active est bipolaire, et c'est
    /// entre ses deux polarités que passe la ligne neutre le long de laquelle
    /// s'alignent les arches (sous-lot 3.3). Deux pôles de même signe, ou
    /// trop éloignés, ne formeraient pas un groupe.
    #[test]
    fn chaque_paire_est_bipolaire_et_serree() {
        let p = poles(42.0);
        assert_eq!(p.len(), NB_PAIRES * 2, "il manque des pôles");
        for paire in p.chunks(2) {
            assert!(
                paire[0].force * paire[1].force < 0.0,
                "paire de même polarité : {} et {}",
                paire[0].force,
                paire[1].force
            );
            let ecart = paire[0].dir.dot(paire[1].dir).clamp(-1.0, 1.0).acos();
            assert!(
                ecart < 3.0 * ECART_PAIRE,
                "polarités trop écartées ({ecart} rad) : ce n'est plus un groupe"
            );
            assert!(ecart > 0.1, "polarités confondues ({ecart} rad) : elles s'annulent");
        }
        // Toutes les directions sont unitaires : le champ lit un `dot` dessus.
        for pole in &p {
            assert!((pole.dir.length() - 1.0).abs() < 1e-4, "direction non unitaire");
        }
    }

    /// **Le champ est fort près d'un pôle, nul loin de tous, et son signe dit
    /// la polarité.** C'est le contrat dont les sous-lots suivants dépendent :
    /// 3.2 cherche les maxima, 3.3 cherche deux signes opposés.
    #[test]
    fn le_champ_marque_les_poles_et_s_efface_entre_eux() {
        let p = poles(7.0);
        for pole in &p {
            let au_pole = champ(&p, pole.dir);
            assert!(
                au_pole.signum() == pole.force.signum(),
                "au pôle, le champ ({au_pole}) contredit la polarité ({})",
                pole.force
            );
            // À l'opposé exact de ce pôle, sa contribution est éteinte.
            let oppose = champ(&p, -pole.dir).abs();
            assert!(
                oppose < au_pole.abs(),
                "le champ est aussi fort à l'opposé ({oppose}) qu'au pôle ({})",
                au_pole.abs()
            );
        }
    }

    /// **Le champ reste borné.** Il alimentera un assombrissement et un
    /// placement de taches : une valeur qui s'emballerait donnerait une
    /// étoile noire ou une tache géante. La somme de `NB_PAIRES × 2` noyaux
    /// de force ≤ 1 ne peut pas dépasser leur compte.
    #[test]
    fn le_champ_ne_s_emballe_pas() {
        let p = poles(3.0);
        let plafond = (NB_PAIRES * 2) as f32;
        for graine in 0..200 {
            let mut rng = Rng::new(graine);
            let d = direction(&mut rng);
            let v = champ(&p, d);
            assert!(v.is_finite(), "champ non fini en {d:?}");
            assert!(v.abs() <= plafond, "champ {v} au-delà de {plafond}");
        }
    }

    /// **Un groupe naît là où le champ est fort**, pas n'importe où. C'est
    /// tout le sous-lot 3.2 : la tache cesse d'être un autocollant posé au
    /// hasard sur la sphère.
    #[test]
    fn un_groupe_naît_sur_les_maxima_du_champ() {
        let p = poles(11.0);
        let sites = groupe(&p, 0);
        assert!(!sites.is_empty(), "la première région active ne donne aucun site");
        // Référence : le champ moyen sur la sphère, que les sites doivent
        // dominer largement.
        let mut rng = Rng::new(5);
        let moyen: f32 = (0..400).map(|_| champ(&p, direction(&mut rng)).abs()).sum::<f32>() / 400.0;
        for s in &sites {
            let ici = champ(&p, s.dir).abs();
            assert!(
                ici > moyen * 2.0,
                "site à {ici} de champ, pour une moyenne de {moyen} : c'est une tache posée au hasard"
            );
            assert!((s.dir.length() - 1.0).abs() < 1e-4, "site non unitaire");
            // ⚠️ Et le site appartient bien à **la région demandée**. Sans
            // cette garde, `CHAMP_MIN` suffisait à satisfaire l'assertion
            // ci-dessus : des sites tirés au hasard qui passent le seuil sont
            // par construction dans un champ fort, mais ne forment plus un
            // groupe — et le sous-lot 3.3 ne saurait plus où ancrer ses pieds.
            let a_la_paire = [p[0].dir, p[1].dir]
                .iter()
                .map(|d| d.dot(s.dir).clamp(-1.0, 1.0).acos())
                .fold(f32::INFINITY, f32::min);
            assert!(
                a_la_paire <= ECART_PAIRE,
                "site à {a_la_paire} rad de sa paire : il n'appartient pas à la région"
            );
        }
    }

    /// **Une principale, des suiveuses.** Des taches toutes de même taille
    /// donneraient un semis régulier, pas un groupe — et le sous-lot 3.3 a
    /// besoin de savoir laquelle mène pour y ancrer ses arches.
    #[test]
    fn le_groupe_a_une_principale_et_des_suiveuses() {
        let mut vu_accompagne = false;
        for graine in [1.0_f32, 42.0, 907.5] {
            for paire in 0..NB_PAIRES {
                let sites = groupe(&poles(graine), paire);
                if sites.is_empty() {
                    continue; // région noyée : légitime, voir `CHAMP_MIN`
                }
                assert!((sites[0].poids - 1.0).abs() < 1e-6, "la tête ne porte pas le poids 1");
                for f in sites.windows(2) {
                    assert!(
                        f[0].poids > f[1].poids,
                        "graine {graine} : deux taches de même poids ({}) — ce n'est plus un groupe",
                        f[0].poids
                    );
                }
                vu_accompagne |= sites.len() > 1;
            }
        }
        assert!(vu_accompagne, "toutes les taches naissent seules : il n'y a pas de groupe");
    }

    /// **Une étoile n'est jamais sans région active.** `CHAMP_MIN` écarte les
    /// paires noyées par une voisine ; s'il les écartait toutes, plus aucune
    /// tache ne naîtrait et la surface serait morte pour la session entière.
    #[test]
    fn chaque_etoile_garde_au_moins_une_region_active() {
        for k in 0..120 {
            let graine = k as f32 * 8.3;
            let p = poles(graine);
            let total: usize = (0..NB_PAIRES).map(|i| groupe(&p, i).len()).sum();
            assert!(total > 0, "graine {graine} : aucune tache ne peut naître");
        }
    }

    /// **Les deux polarités sont représentées.** C'est la condition du
    /// sous-lot 3.3 : une arche enjambe un changement de signe. Un groupe
    /// entièrement d'une polarité ne pourrait porter aucun pied.
    #[test]
    fn le_groupe_couvre_les_deux_polarites() {
        let p = poles(3.5);
        let sites = groupe(&p, 2);
        let signes: Vec<f32> = sites.iter().map(|s| champ(&p, s.dir).signum()).collect();
        assert!(
            signes.iter().any(|s| *s > 0.0) && signes.iter().any(|s| *s < 0.0),
            "groupe unipolaire : {signes:?}"
        );
    }

    /// **Une région qui n'existe pas ne fait rien naître** : l'appelant tire
    /// un indice, il ne doit pas paniquer s'il dépasse.
    #[test]
    fn une_paire_hors_borne_reste_sans_effet() {
        let p = poles(2.0);
        assert!(groupe(&p, NB_PAIRES).is_empty());
        assert!(groupe(&[], 0).is_empty());
    }

    /// **Un pied ne sort jamais de sa tache.** C'est la régression signalée à
    /// l'écran (« les boucles partent de zones externes aux taches ») :
    /// l'étalement de l'arcade était mesuré contre l'écart entre pôles
    /// (0,34 rad) et non contre le rayon d'une tache (~0,15 à 0,26 rad), si
    /// bien que **100 % des pieds** tombaient dehors, à 1,26 rayon du centre
    /// en médiane. Un pied hors de sa tache est une arche qui part de nulle
    /// part.
    #[test]
    fn le_pied_ne_sort_jamais_de_sa_tache() {
        let (grosse, petite) = ((vec3(1.0, 0.0, 0.0), 0.26), (vec3(0.6, 0.8, 0.0).normalize(), 0.09));
        for g in [-1.0_f32, -0.7, -0.2, 0.0, 0.3, 1.0] {
            for h in [-1.0_f32, 0.0, 1.0] {
                let pi = pieds(grosse, petite, (g, h)).expect("deux ancres distinctes");
                for (pied, (centre, rayon), nom) in
                    [(pi.a, grosse, "grosse"), (pi.b, petite, "petite")]
                {
                    let d = centre.dot(pied).clamp(-1.0, 1.0).acos();
                    assert!(
                        d <= rayon,
                        "pied {nom} à {d} rad du centre pour un rayon de {rayon} ({g}/{h})"
                    );
                }
            }
        }
    }

    /// **Mais il coulisse vraiment**, sinon toutes les arches d'une région
    /// naîtraient au même endroit : c'est l'**arcade** — plusieurs arches
    /// côte à côte le long de la ligne neutre — au lieu d'un éventail partant
    /// d'un point unique, cause du « croissant ».
    #[test]
    fn le_glissement_etale_l_arcade() {
        let (a, b) = ((vec3(1.0, 0.0, 0.0), 0.26), (vec3(0.0, 1.0, 0.0), 0.26));
        let gauche = pieds(a, b, (-1.0, -1.0)).expect("pas de pieds à gauche");
        let droite = pieds(a, b, (1.0, 1.0)).expect("pas de pieds à droite");
        let ecart = gauche.a.dot(droite.a).clamp(-1.0, 1.0).acos();
        assert!(
            ecart > 0.26 * PART_PIED,
            "les deux extrêmes de l'arcade ne sont séparés que de {ecart} rad : rien n'est étalé"
        );
    }

    /// **Deux ancres confondues ne font pas d'arche** : il n'y aurait rien à
    /// enjamber, et aucune ligne neutre à suivre.
    #[test]
    fn deux_ancres_confondues_ne_font_pas_d_arche() {
        let a = (vec3(1.0, 0.0, 0.0), 0.2);
        assert!(pieds(a, a, (0.5, -0.5)).is_none());
        assert!(pieds(a, (-a.0, 0.2), (0.5, -0.5)).is_none(), "deux ancres opposées non plus");
    }

    /// **Une direction est rattachée à sa région.** C'est le lien qui manquait
    /// entre les deux moitiés du spectacle : la tache disait où elle était, et
    /// l'arche naissait sans le savoir.
    #[test]
    fn paire_de_reconnait_la_region_d_une_direction() {
        let p = poles(88.0);
        for (i, pole) in p.iter().enumerate() {
            assert_eq!(paire_de(&p, pole.dir), Some(i / 2), "le pôle {i} est mal rattaché");
        }
        assert_eq!(paire_de(&[], Vec3::X), None);
    }
}
