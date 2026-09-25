use super::champ::{self, Site};
use super::{Soleil, MAX_TACHES};
use macroquad::prelude::*;
use macroquad::rand::gen_range;

/// Sursauts simultanés au plus : le tableau `spots[8]` est **partagé** avec
/// les taches (voir [`encoder_flare`]) — ce moteur plafonne pipelines et
/// uniforms (`docs/suivi/technique.md`), pas de second tableau pour ça. Une
/// poignée de taches actives plus deux ou trois flares tient large sous 8.
pub(super) const MAX_FLARES: usize = 3;

/// **Arches vivantes au plus.** Au-delà, elles cessent de se lire une à une :
/// leurs billboards additifs se recouvrent et l'étoile se retrouve ceinte
/// d'un croissant lumineux continu. Quatre laissent voir la forme de chacune.
pub(super) const MAX_BOUCLES: usize = 4;

/// **Délai minimal entre deux naissances d'arches**, en secondes de
/// spectacle. Il valait 0,12 s — huit arches par seconde, toutes au même
/// endroit. Une protubérance monte en ~2 s : en espacer les départs d'au
/// moins une demi-seconde laisse l'œil suivre chacune.
pub(super) const CADENCE_MIN: f32 = 0.55;

#[derive(Clone, Copy, PartialEq)]
pub(super) enum EtatBoucle {
    Montee,
    Tenue,
    Rupture,
    Retombee,
}

/// Tache active (région sombre) sur la surface, repère "surface" (tourne avec le soleil).
pub(super) struct Tache {
    pub dir: Vec3,
    pub taille: f32, // rayon angulaire (rad)
    pub age: f32,
    pub vie_max: f32,
    pub intensite: f32,
}

/// Sursaut lumineux localisé (flare), ancré sur une tache — repère "surface".
/// Bref et intense, à l'inverse de la tache qu'il accompagne : il éclaircit
/// au lieu d'assombrir (voir [`encoder_flare`]).
pub(super) struct Flare {
    pub dir: Vec3,
    pub taille: f32, // rayon angulaire (rad), avant fondu d'intensité
    pub age: f32,
    pub duree: f32,
    pub intensite: f32,
}

/// Boucle coronale ancrée par deux pieds (repère surface).
pub(super) struct Boucle {
    pub sa: Vec3,
    pub sb: Vec3,
    pub apex: f32,
    pub apex_max: f32,
    pub skew: f32, // décalage latéral du sommet -> arches asymétriques
    pub etat: EtatBoucle,
    pub timer: f32,
    pub tenue_max: f32,
    pub rompt: bool,
    pub intensite: f32,
    pub apex_vel: f32, // vitesse d'expansion lors de l'éjection (corde de flux)
}

/// **Quelle tache porte la prochaine arche.**
///
/// Une protubérance se lit *parce qu*'elle relie deux pieds d'une tache ; on
/// en choisit une **active** (`intensite > 0`) quand il y en a, sinon `None`
/// — l'appelant retombe alors sur un tirage aléatoire (une étoile sans tache
/// continue d'éruptionner).
///
/// ⚠️ **Pondéré, et non « la plus intense ».** La première version prenait
/// systématiquement le maximum : toutes les arches naissaient donc au même
/// endroit et s'empilaient en un **croissant** informe au lieu de se lire
/// comme des boucles distinctes — signalé à l'écran. La tache la plus
/// intense reste la plus probable, mais les autres ont leur part.
pub(super) fn tache_porteuse(taches: &[Tache], tirage: f32) -> Option<usize> {
    let total: f32 = taches.iter().map(|t| t.intensite.max(0.0)).sum();
    if total <= 0.0 {
        return None; // aucune tache active : l'appelant retombe sur l'aléatoire
    }
    // Roue de la fortune pondérée par l'intensité. `tirage` vient de
    // l'appelant plutôt que d'un `gen_range` interne : la décision reste
    // pure, donc testable — `macroquad::rand` exige le contexte graphique.
    let mut seuil = tirage.clamp(0.0, 1.0) * total;
    for (i, t) in taches.iter().enumerate() {
        seuil -= t.intensite.max(0.0);
        if seuil <= 0.0 {
            return Some(i);
        }
    }
    // Arrondi malheureux en fin de course : la dernière active fait l'affaire.
    taches.iter().rposition(|t| t.intensite > 0.0)
}

/// **Cosinus de l'angle en deçà duquel deux taches se confondent.**
/// ≈ 0,28 rad : l'ordre de grandeur d'un rayon de tache. Sert à ne pas faire
/// renaître un groupe là où le précédent vit encore.
const COS_OCCUPE: f32 = 0.96;

/// **La région est-elle déjà prise ?**
///
/// Les pôles étant fixes pour la session, le tirage retombe forcément sur des
/// régions déjà actives ; sans ce garde-fou, les taches s'empileraient aux
/// trois mêmes endroits jusqu'à saturer `MAX_TACHES`, et aucune autre région
/// n'aurait jamais sa chance.
pub(super) fn region_occupee(taches: &[Tache], sites: &[Site]) -> bool {
    sites
        .iter()
        .any(|s| taches.iter().any(|t| t.dir.dot(s.dir) > COS_OCCUPE))
}

/// Encode le rayon effectif d'une **tache** (assombrissante) dans `spots[i].w`.
/// **Positif.** Voir [`encoder_flare`] pour l'autre sens de ce même canal.
pub(super) fn encoder_tache(rayon: f32) -> f32 {
    rayon
}

/// Encode le rayon effectif d'un **flare** (éclaircissant) dans `spots[i].w`.
///
/// Le moteur plafonne pipelines et uniforms (`docs/suivi/technique.md`) : un
/// second tableau d'uniforms pour les flares coûterait ce plafond. Le signe
/// distingue les deux cas dans le tableau `spots[8]` déjà là — **négatif**
/// pour un flare. Un rayon nul reste neutre dans les deux encodages
/// (`-0.0 == 0.0` : aucune branche shader ne le lit comme un flare).
pub(super) fn encoder_flare(rayon: f32) -> f32 {
    -rayon
}

// La lecture du canal (le décodage) n'a lieu qu'au fragment shader
// (`soleil.frag.glsl`), qui duplique forcément cette même logique en GLSL —
// intestable directement (`docs/methode.md`). Ces deux fonctions n'ont donc
// pas d'appelant en dehors des tests : elles fixent le contrat que le GLSL
// doit respecter, et c'est tout ce qu'on peut vérifier ici.
#[cfg(test)]
/// Vrai si `w` (canal `spots[i].w`) encode un flare plutôt qu'une tache.
fn est_flare(w: f32) -> bool {
    w < 0.0
}

#[cfg(test)]
/// Rayon effectif porté par `w`, quel que soit son encodage (tache ou flare).
fn rayon_effectif(w: f32) -> f32 {
    w.abs()
}

impl Soleil {
    /// **Fait naître une région active entière** : une tache de tête et ses
    /// suiveuses, sur les maxima du champ magnétique (sous-lot 3.2).
    ///
    /// Avant, une tache naissait seule et n'importe où (`dir_aleatoire`) :
    /// rien ne la reliait à la surface, d'où l'effet d'**autocollant**. Ici,
    /// c'est le champ — la même source que liront les arches (3.3) et le
    /// shader (3.4) — qui décide de l'endroit *et* des tailles.
    ///
    /// ⚠️ Les pôles ne bougeant pas de la session, les régions reviennent aux
    /// mêmes endroits. C'est assumé (`astres_v2_champ_partage.md` §7, « le
    /// champ doit-il vivre ? ») : une dérive lente coûterait un float, elle
    /// n'est pas de ce sous-lot.
    pub(super) fn naitre_groupe(&mut self) {
        let paire = ((gen_range(0.0, 1.0) * champ::NB_PAIRES as f32) as usize)
            .min(champ::NB_PAIRES - 1);
        let sites = champ::groupe(&self.poles, paire);
        if sites.is_empty() || region_occupee(&self.taches, &sites) {
            return; // le prochain rendez-vous retentera, ailleurs
        }
        // Une seule taille de référence pour tout le groupe : ce sont les
        // poids du champ qui les rendent inégales, pas trois tirages sans
        // rapport entre eux.
        let base = gen_range(0.20, 0.32);
        let vie = gen_range(8.0, 16.0);
        for s in sites {
            if self.taches.len() >= MAX_TACHES {
                break;
            }
            self.taches.push(Tache {
                dir: s.dir,
                taille: (base * s.poids).max(0.06),
                age: 0.0,
                // Une petite tache se dissout plus vite qu'une grosse.
                vie_max: vie * (0.55 + 0.45 * s.poids),
                intensite: 0.0,
            });
        }
    }

    /// Crée une boucle ancrée sur une tache active (tirée au poids de son
    /// intensité) s'il y en a une, sinon dispersée au hasard.
    pub(super) fn declencher_boucle(&mut self) {
        let centre = match tache_porteuse(&self.taches, gen_range(0.0, 1.0)) {
            Some(i) => self.taches[i].dir,
            None => dir_aleatoire(),
        };
        let taille_spot: f32 = gen_range(0.12, 0.26);

        let p = self.puissance;
        let f = self.forme;
        let j = 0.3 + 1.7 * self.alea; // amplitude de l'aléa sur la forme

        // Deux pieds qui enjambent le centre de la tache.
        // "forme" donne l'écart de base ; "aléa" ajoute du jitter.
        let rnd = dir_aleatoire();
        let tang = (rnd - centre * centre.dot(rnd)).normalize_or_zero();
        let half: f32 =
            (taille_spot * (0.4 + f * 0.9 + gen_range(-0.4, 0.6) * j)).clamp(0.05, 1.4);
        let sa = (centre * half.cos() + tang * half.sin()).normalize();
        let sb = (centre * half.cos() - tang * half.sin()).normalize();

        // Puissance -> hauteur ; forme -> biais ; aléa -> dispersion hauteur/asymétrie.
        let apex_base = 0.25 + 1.3 * p;
        let apex_max: f32 =
            (apex_base * (1.0 + f * 0.3) * (1.0 + gen_range(-0.5, 0.6) * j)).clamp(0.1, 2.6);
        let skew = gen_range(-0.8, 0.8) * j * apex_max;

        // Plus la boucle est grande, plus elle part dans l'espace.
        let prob = smoothstep(0.3, 1.3, apex_max);
        let rompt = gen_range(0.0, 1.0) < prob;

        self.boucles.push(Boucle {
            sa,
            sb,
            apex: 0.0,
            apex_max,
            skew,
            etat: EtatBoucle::Montee,
            timer: 0.0,
            tenue_max: gen_range(0.6, 1.8) + gen_range(0.0, 1.0) * j,
            rompt,
            intensite: 1.0,
            apex_vel: 0.0,
        });
    }

    pub(super) fn maj(&mut self, dt: f32) {
        self.temps += dt;

        // Trou noir : pas de taches ni d'éruptions de surface (l'horizon est noir).
        if self.couronne_type > 4.5 {
            return;
        }

        // --- Taches : apparition / fondu / disparition ---
        if self.temps >= self.prochaine_tache && self.taches.len() < MAX_TACHES {
            self.naitre_groupe();
            self.prochaine_tache = self.temps + gen_range(3.0, 7.0);
        }
        for t in &mut self.taches {
            t.age += dt;
            let fade_in = (t.age / 1.5).clamp(0.0, 1.0);
            let fade_out = ((t.vie_max - t.age) / 2.0).clamp(0.0, 1.0);
            t.intensite = fade_in.min(fade_out);
        }
        self.taches.retain(|t| t.age < t.vie_max);

        // --- Flares : sursaut lumineux localisé, bref, ancré sur une tache ---
        //
        // `self.freq` porte déjà l'activité propre au type (`Soleil::set_eruptions`,
        // §1.3) : une naine M active en enchaîne, une géante rarement. Suit
        // `self.temps`, alimenté plus haut par `dt` — l'horloge de spectacle
        // (`DtAnimation`), jamais l'horloge orbitale accélérable.
        if self.temps >= self.prochain_flare && self.flares.len() < MAX_FLARES {
            let dir = match tache_porteuse(&self.taches, gen_range(0.0, 1.0)) {
                Some(i) => self.taches[i].dir,
                None => dir_aleatoire(),
            };
            self.flares.push(Flare {
                dir,
                taille: gen_range(0.10, 0.22),
                age: 0.0,
                duree: gen_range(0.8, 2.2),
                intensite: 0.0,
            });
            let moy = 6.0 - 4.5 * self.freq;
            self.prochain_flare = self.temps + (gen_range(0.6, 1.6) * moy).max(0.3);
        }
        for f in &mut self.flares {
            f.age += dt;
            // Montée quasi immédiate (un sursaut n'a pas la lenteur d'une tache),
            // retombée sur le reste de sa durée.
            let fade_in = (f.age / 0.15).clamp(0.0, 1.0);
            let fade_out = ((f.duree - f.age) / (f.duree * 0.6).max(0.05)).clamp(0.0, 1.0);
            f.intensite = fade_in.min(fade_out);
        }
        self.flares.retain(|f| f.age < f.duree);

        // --- Déclenchement des boucles (sur les taches) ---
        // ⚠️ **Plafond d'arches vivantes.** Rien ne les bornait : seule leur
        // extinction les retirait, et le plancher de cadence à 0,12 s
        // autorisait jusqu'à huit naissances par seconde. Ajouté à l'ancrage
        // sur une seule tache, ça empilait les arches en un **croissant**
        // opaque — le défaut signalé à l'écran. Le remède tient en deux
        // bornes : combien peuvent vivre ensemble, et à quelle cadence au
        // plus vite.
        if self.temps >= self.prochaine_eruption && self.boucles.len() < MAX_BOUCLES {
            // Plusieurs éruptions par cycle (dispersées), d'autant plus que freq est haut.
            let n_loops = 1 + (self.freq * 2.5) as usize; // 1..=3
            for _ in 0..n_loops {
                if self.boucles.len() >= MAX_BOUCLES {
                    break;
                }
                self.declencher_boucle();
            }
            // Fréquence haute -> intervalle court.
            let moy = 3.5 - 3.2 * self.freq;
            self.prochaine_eruption = self.temps + (gen_range(0.5, 1.5) * moy).max(CADENCE_MIN);
        }

        // --- Boucles : machine à états ---
        for b in &mut self.boucles {
            b.timer += dt;
            match b.etat {
                EtatBoucle::Montee => {
                    b.apex += (b.apex_max - b.apex) * (dt * 2.5).min(1.0);
                    if b.apex >= b.apex_max * 0.96 {
                        b.etat = EtatBoucle::Tenue;
                        b.timer = 0.0;
                    }
                }
                EtatBoucle::Tenue => {
                    if b.timer >= b.tenue_max {
                        if b.rompt {
                            // Début d'éjection : l'arche commence à s'étendre vers l'espace.
                            b.apex_vel = 0.4 + 0.5 * b.apex_max;
                            b.etat = EtatBoucle::Rupture;
                        } else {
                            b.etat = EtatBoucle::Retombee;
                        }
                        b.timer = 0.0;
                    }
                }
                EtatBoucle::Rupture => {
                    // Corde de flux qui s'éjecte : l'arche gonfle en accélérant,
                    // les pieds restent ancrés (les jambes s'étirent), et elle s'estompe.
                    b.apex_vel += 1.2 * dt; // accélération
                    b.apex += b.apex_vel * dt;
                    b.intensite -= dt * 0.45;
                }
                EtatBoucle::Retombee => {
                    b.apex -= b.apex * (dt * 1.5).min(1.0) + dt * 0.05;
                    b.intensite -= dt * 0.8;
                }
            }
        }
        self.boucles.retain(|b| b.intensite > 0.0 && b.apex < 8.0);
    }
}

fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

pub(super) fn dir_aleatoire() -> Vec3 {
    loop {
        let p = vec3(
            gen_range(-1.0, 1.0),
            gen_range(-1.0, 1.0),
            gen_range(-1.0, 1.0),
        );
        let l2 = p.length_squared();
        if l2 > 0.0001 && l2 <= 1.0 {
            return p / l2.sqrt();
        }
    }
}

#[cfg(test)]
mod tests_tache_porteuse {
    use super::*;

    fn tache(dir: Vec3, intensite: f32) -> Tache {
        Tache { dir, taille: 0.2, age: 1.0, vie_max: 10.0, intensite }
    }

    /// **Une tache active est choisie quand il y en a**, et le tirage porte
    /// bien sur toutes : chacune occupe sa part de l'intervalle, à la mesure
    /// de son intensité.
    #[test]
    fn chaque_tache_active_peut_porter_une_arche() {
        let taches = vec![
            tache(vec3(1.0, 0.0, 0.0), 0.4),
            tache(vec3(0.0, 1.0, 0.0), 0.9),
            tache(vec3(0.0, 0.0, 1.0), 0.1),
        ];
        // Total 1,4 : la première tient [0 ; 0,4/1,4[, la deuxième jusqu'à
        // 1,3/1,4, la dernière la fin.
        assert_eq!(tache_porteuse(&taches, 0.1), Some(0));
        assert_eq!(tache_porteuse(&taches, 0.5), Some(1));
        assert_eq!(tache_porteuse(&taches, 0.99), Some(2));
    }

    /// **La plus intense reste la plus probable.** C'est ce qui garde le sens
    /// physique — une grosse tache porte plus d'arches — sans les y empiler
    /// toutes, ce que faisait la première version (le « croissant »).
    #[test]
    fn la_plus_intense_occupe_la_plus_grande_part() {
        let taches = vec![tache(vec3(1.0, 0.0, 0.0), 0.1), tache(vec3(0.0, 1.0, 0.0), 0.9)];
        let parts = (0..100).filter(|k| tache_porteuse(&taches, *k as f32 / 100.0) == Some(1)).count();
        assert!(parts > 80, "la tache intense ne prend que {parts} % des tirages");
        assert!(parts < 100, "elle les prend TOUS : c'est le défaut du croissant");
    }

    /// **Le repli joue quand il n'y en a pas** : ni taches, ni taches actives
    /// (toutes en train d'apparaître ou de s'éteindre, intensité nulle) — une
    /// étoile sans tache visible doit continuer d'éruptionner ailleurs.
    #[test]
    fn replie_sur_aucune_quand_rien_n_est_actif() {
        assert_eq!(tache_porteuse(&[], 0.5), None);
        let taches = vec![tache(vec3(1.0, 0.0, 0.0), 0.0), tache(vec3(0.0, 1.0, 0.0), 0.0)];
        assert_eq!(tache_porteuse(&taches, 0.5), None);
    }

    /// **Déterministe à tirage égal** : même liste, même tirage, même choix.
    /// C'est ce qui rend un système reproductible à graine fixée.
    #[test]
    fn deterministe_a_liste_et_tirage_egaux() {
        let taches = vec![tache(vec3(1.0, 0.0, 0.0), 0.5), tache(vec3(0.0, 1.0, 0.0), 0.7)];
        assert_eq!(tache_porteuse(&taches, 0.33), tache_porteuse(&taches, 0.33));
        // Et les bornes du tirage ne sortent jamais de la liste.
        assert!(tache_porteuse(&taches, 0.0).is_some());
        assert!(tache_porteuse(&taches, 1.0).is_some());
    }
}

#[cfg(test)]
mod tests_region_occupee {
    use super::*;

    fn tache(dir: Vec3) -> Tache {
        Tache { dir: dir.normalize(), taille: 0.2, age: 1.0, vie_max: 10.0, intensite: 1.0 }
    }

    fn site(dir: Vec3) -> Site {
        Site { dir: dir.normalize(), poids: 1.0 }
    }

    /// **Un groupe ne renaît pas sur lui-même.** Les pôles sont fixes : sans
    /// ce refus, chaque rendez-vous rechargerait les trois mêmes régions
    /// jusqu'à saturer `MAX_TACHES`, et la surface se figerait.
    #[test]
    fn une_region_qui_porte_deja_une_tache_est_refusee() {
        let sites = [site(vec3(1.0, 0.0, 0.0)), site(vec3(0.0, 1.0, 0.0))];
        // Exactement dessus, et un peu à côté (0,15 rad) : même région.
        assert!(region_occupee(&[tache(vec3(1.0, 0.0, 0.0))], &sites));
        assert!(region_occupee(&[tache(vec3(1.0, 0.15, 0.0))], &sites));
        // Et il suffit qu'**une** des taches du groupe soit encore là.
        assert!(region_occupee(&[tache(vec3(0.0, 1.0, 0.02))], &sites));
    }

    /// **Et une région libre reste libre** : un refus systématique
    /// empêcherait toute tache de naître.
    #[test]
    fn une_region_eloignee_reste_disponible() {
        let sites = [site(vec3(1.0, 0.0, 0.0))];
        assert!(!region_occupee(&[], &sites));
        assert!(!region_occupee(&[tache(vec3(0.0, 0.0, 1.0))], &sites));
        // Un demi-radian plus loin : deux taches distinctes, pas un doublon.
        assert!(!region_occupee(&[tache(vec3(1.0, 0.55, 0.0))], &sites));
    }
}

#[cfg(test)]
mod tests_encodage_spots {
    use super::*;

    /// **Tache et flare se distinguent dans la même donnée** : le signe de `w`,
    /// pas un second tableau (le moteur plafonne les uniforms).
    #[test]
    fn le_signe_distingue_tache_et_flare() {
        let w_tache = encoder_tache(0.2);
        let w_flare = encoder_flare(0.2);
        assert!(!est_flare(w_tache), "une tache lue comme un flare");
        assert!(est_flare(w_flare), "un flare lu comme une tache");
        assert_eq!(rayon_effectif(w_tache), 0.2);
        assert_eq!(rayon_effectif(w_flare), 0.2);
    }

    /// **Un rayon nul reste neutre**, quel que soit l'encodage — sinon un
    /// flare éteint (intensité retombée à 0) resterait lu comme actif.
    #[test]
    fn un_rayon_nul_reste_neutre() {
        assert!(!est_flare(encoder_tache(0.0)));
        assert!(!est_flare(encoder_flare(0.0)));
        assert_eq!(rayon_effectif(encoder_flare(0.0)), 0.0);
    }
}
