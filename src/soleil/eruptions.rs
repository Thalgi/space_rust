use super::champ::{self, Pieds, Site};
use super::{Soleil, MAX_TACHES};
use macroquad::prelude::*;
use macroquad::rand::gen_range;

/// Sursauts simultanés au plus : le tableau `spots[8]` est **partagé** avec
/// les taches (voir [`encoder_flare`]) — ce moteur plafonne pipelines et
/// uniforms (`docs/suivi/technique.md`), pas de second tableau pour ça. Une
/// poignée de taches actives plus deux ou trois flares tient large sous 8.
pub(super) const MAX_FLARES: usize = 3;

/// **Grandes arches vivantes au plus.** Au-delà, elles cessaient de se lire
/// une à une : leurs billboards additifs se recouvraient et l'étoile se
/// retrouvait ceinte d'un croissant lumineux continu.
pub(super) const MAX_ARCHES: usize = 4;

/// **Petites boucles vivantes au plus** — un plafond **séparé**.
///
/// Un plafond commun les mettait en concurrence avec les grandes arches : en
/// faire naître plus revenait à en voir moins. Or elles ne coûtent pas la
/// même chose à la lecture — une petite boucle est un fil court et fin, une
/// grande arche traverse la moitié d'une région. C'est pour ça qu'il y en a
/// deux. ⚠️ Réglage esthétique, comme son voisin.
pub(super) const MAX_COURTES: usize = 12;

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
    /// **Pincement du col**, en part de l'écart des pieds. Positif : les
    /// jambes se resserrent et la boucle s'étrangle à la base, silhouette
    /// haute et étroite. Négatif : elle s'évase en dôme large et bas.
    pub pincee: f32,
    /// **Épaisseur du tube**, en part de l'épaisseur de référence. Un tube de
    /// flux n'a pas de calibre standard : une protubérance massive est un
    /// ruban épais, une boucle ténue un fil. Toutes les boucles avaient la
    /// même — un chapelet de billboards du même gabarit.
    pub epaisseur: f32,
    /// **Boucle courte** : ses deux pieds sont dans la *même* tache, elle ne
    /// relie rien. Sa seule utilité en mémoire est de compter à part des
    /// grandes arches ([`MAX_COURTES`]), pour qu'en faire naître plus ne
    /// revienne pas à en voir moins.
    pub courte: bool,
    /// **Inclinaison hors du plan radial**, en part de la hauteur. Une vraie
    /// boucle ne se tient pas droite dans le plan qui contient le centre de
    /// l'étoile : elle penche. Sans ça, deux boucles d'un même groupe
    /// présentent exactement la même silhouette à la caméra.
    pub inclinaison: f32,
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

/// **Distance de répulsion entre deux boucles**, en part de leur empreinte
/// moyenne.
///
/// Trop près, deux boucles ne se lisent plus comme deux : leurs billboards
/// sont en blending **additif**, donc leurs halos **somment**, et c'est là —
/// *entre* les boucles, jamais dans une seule — que naît le « croissant »
/// signalé à l'écran (`astres_v2_boucles.md`). Le lot 3 bis en avait réduit le
/// nombre et ralenti la cadence : il traitait le symptôme.
///
/// ⚠️ **Relative, et non un angle fixe.** Elle valait 0,07 rad pour tout le
/// monde, ce qui empêchait les petites boucles de se serrer alors que leurs
/// halos sont petits eux aussi (le calibre suit l'étendue, voir
/// `rendu::RAPPORT_TUBE`). L'écart qui compte est celui du **tube**, pas
/// celui de la boucle : d'où une fraction de l'empreinte. Réglage
/// esthétique, entre « croissant » et « boucle solitaire ».
const MARGE_ARCHES: f32 = 0.16;

/// **Plus de place pour une boucle de cette catégorie ?**
///
/// Les deux catégories comptent **séparément**. Sous un plafond commun,
/// augmenter la part des petites boucles revenait à voir moins de grandes
/// arches : elles se disputaient les mêmes places alors qu'elles ne coûtent
/// pas la même chose à la lecture.
pub(super) fn categorie_pleine(boucles: &[Boucle], courte: bool) -> bool {
    let vivantes = boucles.iter().filter(|b| b.courte == courte).count();
    vivantes >= if courte { MAX_COURTES } else { MAX_ARCHES }
}

/// Empreinte angulaire d'une boucle : l'écart entre ses deux pieds.
fn empreinte(a: Vec3, b: Vec3) -> f32 {
    a.dot(b).clamp(-1.0, 1.0).acos()
}

/// Direction du sommet d'une arche : le milieu de ses deux pieds. Deux arches
/// dont les sommets se confondent se recouvrent, quelles que soient leurs
/// hauteurs.
fn milieu(a: Vec3, b: Vec3) -> Vec3 {
    (a + b).normalize_or(a)
}

/// **Cette place est-elle déjà occupée par une arche ?**
pub(super) fn arche_trop_proche(boucles: &[Boucle], p: &Pieds) -> bool {
    let m = milieu(p.a, p.b);
    let e = empreinte(p.a, p.b);
    boucles.iter().any(|b| {
        let ecart = milieu(b.sa, b.sb).dot(m).clamp(-1.0, 1.0).acos();
        ecart < (e + empreinte(b.sa, b.sb)) * 0.5 * MARGE_ARCHES
    })
}

/// **Les deux taches entre lesquelles ancrer une arche** dans la région
/// `paire` : la plus grosse de chaque polarité.
///
/// ⚠️ Les **taches**, pas les pôles. Ancrer sur les pôles paraissait
/// équivalent — le champ y est maximal, c'est là que les taches naissent —
/// mais `champ::groupe` écarte les sites trop faibles : une région peut donc
/// ne porter de tache que d'un côté, et un pied planté sur l'autre pôle
/// tombait sur une surface vide. Sans compter qu'une tache **meurt** (8 à
/// 16 s) là où un pôle reste.
///
/// `None` s'il manque une polarité : une arche a besoin de deux pieds, et
/// mieux vaut aucune arche qu'une qui sort de nulle part.
/// Une tache réduite à ce dont un pied a besoin : sa direction et son rayon
/// angulaire.
pub(super) type Ancre = (Vec3, f32);

/// **Rayon effectivement dessiné d'une tache** : son rayon nominal fondu par
/// son intensité.
///
/// ⚠️ **Source unique.** `rendu.rs` l'encode pour le shader, et les pieds
/// d'arche s'y posent. Les deux calculaient chacun le leur : le rendu avec
/// l'intensité, l'ancrage sans — si bien qu'un pied visait le rayon
/// **nominal** d'une tache qui n'atteint cette taille qu'au sommet de sa vie.
/// Pendant son apparition (1,5 s) et son extinction (2 s), la tache dessinée
/// est bien plus petite, et le pied se retrouvait dehors : le « léger offset »
/// signalé à l'écran.
pub(super) fn rayon_visible(t: &Tache) -> f32 {
    t.taille * t.intensite
}

/// **Intensité minimale pour qu'une tache porte une arche.**
///
/// Une tache trop pâle est à peine dessinée, et son ancrage vieillit mal :
/// les pieds d'une arche sont fixés à sa naissance, alors qu'une tache
/// mourante continue de rétrécir sous eux. Mieux vaut ne pas y ancrer. (Le
/// suivi *dynamique* des pieds relève du sous-lot 3.5.)
const ANCRAGE_MIN: f32 = 0.35;

/// La tache vue par un pied d'arche — `None` si elle est trop pâle pour en
/// porter un.
pub(super) fn ancre_de(t: &Tache) -> Option<Ancre> {
    (t.intensite >= ANCRAGE_MIN).then(|| (t.dir, rayon_visible(t)))
}

pub(super) fn ancres(champ_de: impl Fn(Vec3) -> f32, taches: &[Ancre]) -> Option<(Ancre, Ancre)> {
    let (mut plus, mut moins): (Option<Ancre>, Option<Ancre>) = (None, None);
    for t in taches {
        let cible = if champ_de(t.0) > 0.0 { &mut plus } else { &mut moins };
        if cible.is_none_or(|c| c.1 < t.1) {
            *cible = Some(*t);
        }
    }
    Some((plus?, moins?))
}

/// **Part des boucles qui ne relient rien** et retombent dans leur propre
/// tache. Réglage esthétique : il arbitre entre « toutes les boucles se
/// ressemblent » et « plus aucune grande arche ». Se juge à l'œil.
const PART_COURTES: f32 = 0.72;

/// **Hauteur d'une arche, À PROPORTION de l'écart de ses pieds.**
///
/// ⚠️ Elle était **absolue** (`0,25 + 1,3 × puissance`, quelle que soit la
/// portée) : toutes les arches montaient donc à la même altitude, et comme
/// l'écart entre les deux taches d'une région ne varie presque pas, elles
/// avaient toutes **la même taille et la même forme** — signalé à l'écran.
/// Une boucle courte doit rester basse, une longue arche monter haut : c'est
/// le rapport qui se tire au sort, pas la hauteur.
fn apex_pour(sa: Vec3, sb: Vec3, rapport: f32) -> f32 {
    let ecart = sa.dot(sb).clamp(-1.0, 1.0).acos();
    (ecart * rapport.max(0.0)).clamp(0.02, 2.6)
}

/// Ramène un glissement dans `[-1 ; 1]` en l'enroulant : les places d'une
/// arcade forment un anneau, pas un segment avec des bords où s'entasser.
fn enrouler(t: f32) -> f32 {
    (t + 1.0).rem_euclid(2.0) - 1.0
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

    /// **Crée une arche entre deux polarités opposées** de la région active
    /// que porte une tache (sous-lot 3.3).
    ///
    /// Avant, les deux pieds enjambaient le centre d'une tache sur une
    /// tangente **tirée au hasard** : toutes les arches d'une même tache
    /// partaient du même point en éventail, et leurs halos additifs
    /// sommaient en croissant. Ici, les pieds sont ceux du champ, et une
    /// arche **refuse de naître** là où une autre vit déjà — la répulsion
    /// qui attaque la cause et non le symptôme (voir [`COS_ARCHES`]).
    ///
    /// ⚠️ **Sans tache, pas d'arche.** Un repli dispersé existait, « pour
    /// qu'une étoile sans tache continue d'éruptionner » : il faisait naître
    /// des arches au hasard dès qu'une tache disparaissait — signalé à
    /// l'écran. Une protubérance *est* l'arche d'une région active ; sans
    /// région, il n'y a rien à dessiner, et une étoile calme a le droit
    /// d'être calme.
    pub(super) fn declencher_boucle(&mut self) {
        // La catégorie se décide d'abord, parce que chacune a **son** plafond.
        let courte = gen_range(0.0, 1.0) < PART_COURTES;
        if categorie_pleine(&self.boucles, courte) {
            return;
        }
        let Some(i) = tache_porteuse(&self.taches, gen_range(0.0, 1.0)) else {
            return;
        };
        let Some(paire) = champ::paire_de(&self.poles, self.taches[i].dir) else {
            return;
        };
        // Aucune place libre dans l'arcade : on n'en ajoute pas. Empiler ici
        // est exactement ce qui faisait le croissant.
        if let Some(pi) = self.place_libre(paire, courte) {
            self.pousser_boucle(pi.a, pi.b, courte);
        }
    }

    /// **Cherche une place libre dans l'arcade** de la région `paire` : trois
    /// positions le long de la ligne neutre, la première non occupée gagne.
    ///
    /// C'est la répulsion sous sa forme utile — l'arche est *déplacée* plutôt
    /// que refusée. Rien de libre (ou région noyée) : `None`, et il ne naît
    /// rien cette fois-ci.
    fn place_libre(&self, paire: usize, courte: bool) -> Option<Pieds> {
        // Les taches de cette région seulement : une arche ne relie pas deux
        // groupes distincts.
        let ici: Vec<Ancre> = self
            .taches
            .iter()
            .filter(|t| champ::paire_de(&self.poles, t.dir) == Some(paire))
            .filter_map(ancre_de)
            .collect();
        if ici.is_empty() {
            return None;
        }

        // **Une boucle courte ne relie rien** : elle repart dans la tache
        // d'où elle sort. Sans elles, une étoile n'a que de grandes arches,
        // toutes du même gabarit — l'écart entre deux taches d'une région ne
        // variant presque pas. Ce sont les plus nombreuses dans la réalité,
        // petites et éparses.
        if courte {
            let anc = ici[(gen_range(0.0, 1.0) * ici.len() as f32) as usize % ici.len()];
            return (0..3).find_map(|_| {
                let centre = vec2(gen_range(-1.0, 1.0), gen_range(-1.0, 1.0));
                let pi = champ::pieds_courts(
                    anc,
                    centre,
                    gen_range(0.0, std::f32::consts::TAU),
                    gen_range(0.14, 0.5),
                )?;
                (!arche_trop_proche(&self.boucles, &pi)).then_some(pi)
            });
        }

        let (a, b) = ancres(|d| champ::champ(&self.poles, d), &ici)?;

        let depart = gen_range(-1.0, 1.0);
        // Un pied qui glisse un peu moins que l'autre cisaille l'arche : c'est
        // ce qui remplace l'ancien écartement tiré au hasard comme source de
        // variété de silhouette.
        let cisaille = gen_range(-0.35, 0.35);
        (0..3).find_map(|k| {
            let t = enrouler(depart + k as f32 * 0.66);
            let pi = champ::pieds(a, b, (t, enrouler(t + cisaille)))?;
            (!arche_trop_proche(&self.boucles, &pi)).then_some(pi)
        })
    }

    /// Donne sa silhouette à une arche ancrée sur `sa`/`sb`, et l'ajoute.
    fn pousser_boucle(&mut self, sa: Vec3, sb: Vec3, courte: bool) {
        let p = self.puissance;
        let f = self.forme;
        let j = 0.3 + 1.7 * self.alea; // amplitude de l'aléa sur la forme

        // Puissance et forme gouvernent le rapport hauteur/écart ; l'aléa le
        // disperse. L'écart, lui, vient des pieds — voir [`apex_pour`].
        let rapport = (0.30 + 0.95 * p) * (1.0 + f * 0.3) * (1.0 + gen_range(-0.45, 0.85) * j);
        let apex_max = apex_pour(sa, sb, rapport);
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
            // Deux tirages de plus, et deux seulement : ce sont eux qui font
            // que deux boucles de même portée ne se ressemblent pas.
            pincee: gen_range(-0.22, 0.34) * j,
            inclinaison: gen_range(-0.55, 0.55) * j,
            // Un fil ténu ou un ruban massif, et tout l'entre-deux. Borné bas
            // pour que le plancher en pixels ait encore quelque chose à
            // relever, haut pour qu'un tube n'avale pas l'étoile.
            epaisseur: (1.0 + gen_range(-0.5, 0.9) * j).clamp(0.45, 2.1),
            courte,
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
        // Même règle que les arches : un sursaut est une reconnexion de
        // champ, il ne flambe pas sur une surface vide. Le repli aléatoire
        // d'avant posait un éclair lumineux là où rien ne se passait.
        if self.temps >= self.prochain_flare && self.flares.len() < MAX_FLARES {
            if let Some(i) = tache_porteuse(&self.taches, gen_range(0.0, 1.0)) {
                self.flares.push(Flare {
                    dir: self.taches[i].dir,
                    taille: gen_range(0.10, 0.22),
                    age: 0.0,
                    duree: gen_range(0.8, 2.2),
                    intensite: 0.0,
                });
            }
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
        if self.temps >= self.prochaine_eruption {
            // Plusieurs éruptions par cycle (dispersées), d'autant plus que freq est haut.
            // Les plafonds sont vérifiés par catégorie dans `declencher_boucle` :
            // les petites boucles ne prennent pas la place des grandes arches.
            let n_loops = 1 + (self.freq * 2.5) as usize; // 1..=3
            for _ in 0..n_loops {
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
mod tests_repulsion {
    use super::*;

    fn boucle(sa: Vec3, sb: Vec3) -> Boucle {
        Boucle {
            sa: sa.normalize(),
            sb: sb.normalize(),
            apex: 1.0,
            apex_max: 1.0,
            skew: 0.0,
            pincee: 0.0,
            epaisseur: 1.0,
            courte: false,
            inclinaison: 0.0,
            etat: EtatBoucle::Tenue,
            timer: 0.0,
            tenue_max: 1.0,
            rompt: false,
            intensite: 1.0,
            apex_vel: 0.0,
        }
    }

    fn pieds(a: Vec3, b: Vec3) -> Pieds {
        Pieds { a: a.normalize(), b: b.normalize() }
    }

    /// **Deux arches ne naissent pas à la même place.** C'est la cause du
    /// « croissant » : en blending additif, les halos de deux arches voisines
    /// somment et forment un arc continu où l'on ne distingue plus rien.
    #[test]
    fn une_place_deja_prise_est_refusee() {
        let vivante = boucle(vec3(1.0, 0.2, 0.0), vec3(1.0, -0.2, 0.0));
        // Exactement la même place, puis décalée de 0,04 rad : même arche à
        // l'œil.
        assert!(arche_trop_proche(&[vivante], &pieds(vec3(1.0, 0.2, 0.0), vec3(1.0, -0.2, 0.0))));
        assert!(arche_trop_proche(
            &[boucle(vec3(1.0, 0.2, 0.0), vec3(1.0, -0.2, 0.0))],
            &pieds(vec3(1.0, 0.2, 0.04), vec3(1.0, -0.2, 0.04))
        ));
    }

    /// **Et une place libre le reste** : un refus systématique éteindrait
    /// l'étoile — plus aucune arche ne naîtrait jamais.
    #[test]
    fn une_place_libre_accueille_une_arche() {
        let p = pieds(vec3(1.0, 0.2, 0.0), vec3(1.0, -0.2, 0.0));
        assert!(!arche_trop_proche(&[], &p));
        // De l'autre côté de l'étoile, et à 0,1 rad : deux arches d'une même
        // arcade, qui doivent pouvoir coexister côte à côte.
        assert!(!arche_trop_proche(&[boucle(vec3(-1.0, 0.2, 0.0), vec3(-1.0, -0.2, 0.0))], &p));
        assert!(!arche_trop_proche(&[boucle(vec3(1.0, 0.2, 0.1), vec3(1.0, -0.2, 0.1))], &p));
    }

    /// **L'arche s'ancre sur la plus grosse tache de chaque polarité.** Les
    /// deux pieds appartiennent ainsi à des taches **réelles** — ce que
    /// l'ancrage sur les pôles ne garantissait pas : `champ::groupe` écarte
    /// les sites trop faibles, et une tache meurt là où un pôle reste.
    #[test]
    fn l_arche_s_ancre_sur_la_plus_grosse_de_chaque_polarite() {
        let nord = vec3(1.0, 0.0, 0.0);
        let sud = vec3(-1.0, 0.0, 0.0);
        // Le champ est positif d'un côté de l'étoile, négatif de l'autre.
        let champ_de = |d: Vec3| d.dot(nord);
        let taches = [(nord, 0.1), (nord * 0.99 + vec3(0.0, 0.1, 0.0), 0.3), (sud, 0.2)];
        let (a, b) = ancres(champ_de, &taches).expect("les deux polarités sont là");
        assert_eq!(a.1, 0.3, "ce n'est pas la plus grosse du côté positif");
        assert_eq!(b.1, 0.2, "ce n'est pas la tache du côté négatif");
    }

    fn tache(taille: f32, intensite: f32) -> Tache {
        Tache { dir: vec3(1.0, 0.0, 0.0), taille, age: 1.0, vie_max: 10.0, intensite }
    }

    /// **Un pied se pose contre le rayon DESSINÉ, pas le rayon nominal.**
    ///
    /// Une tache n'atteint sa taille nominale qu'au sommet de sa vie : le
    /// shader la dessine fondue par son intensité. L'ancrage visait l'autre —
    /// le « léger offset » signalé à l'écran, qui ne se voyait qu'en début et
    /// en fin de vie d'une tache.
    #[test]
    fn le_pied_vise_le_rayon_dessine() {
        let pleine = tache(0.24, 1.0);
        let naissante = tache(0.24, 0.5);
        assert_eq!(ancre_de(&pleine).unwrap().1, 0.24);
        assert_eq!(
            ancre_de(&naissante).unwrap().1,
            0.12,
            "le pied vise encore le rayon nominal d'une tache à moitié dessinée"
        );
    }

    /// **Une tache trop pâle ne porte pas d'arche.** Ses pieds sont fixés à sa
    /// naissance, mais elle continue de rétrécir sous eux : l'arche finirait
    /// par flotter à côté. (Le suivi dynamique des pieds est le sous-lot 3.5.)
    #[test]
    fn une_tache_trop_pale_ne_porte_pas_d_arche() {
        assert!(ancre_de(&tache(0.24, 0.05)).is_none(), "une tache à peine visible porte une arche");
        assert!(ancre_de(&tache(0.24, 0.0)).is_none());
        assert!(ancre_de(&tache(0.24, 0.9)).is_some(), "une tache bien installée n'en porte plus");
    }

    /// **Sans les deux polarités, pas d'arche.** Mieux vaut aucune arche
    /// qu'une dont un pied sort de nulle part — la régression signalée à
    /// l'écran.
    #[test]
    fn sans_les_deux_polarites_aucune_arche() {
        let champ_de = |d: Vec3| d.dot(vec3(1.0, 0.0, 0.0));
        assert!(ancres(champ_de, &[]).is_none());
        assert!(
            ancres(champ_de, &[(vec3(1.0, 0.0, 0.0), 0.2), (vec3(0.9, 0.3, 0.0), 0.1)]).is_none(),
            "une arche a été ancrée avec une seule polarité"
        );
    }

    /// **Une boucle courte reste basse, une longue arche monte haut.**
    ///
    /// La hauteur était **absolue** : toutes les boucles montaient à la même
    /// altitude quelle que soit leur portée, et comme l'écart entre les deux
    /// taches d'une région ne varie presque pas, elles avaient toutes la même
    /// taille et la même forme — « ça fait très fonction du second degré ».
    #[test]
    fn la_hauteur_suit_l_ecart_des_pieds() {
        let proches = (vec3(1.0, 0.05, 0.0).normalize(), vec3(1.0, -0.05, 0.0).normalize());
        let lointains = (vec3(1.0, 0.5, 0.0).normalize(), vec3(1.0, -0.5, 0.0).normalize());
        let courte = apex_pour(proches.0, proches.1, 1.0);
        let longue = apex_pour(lointains.0, lointains.1, 1.0);
        assert!(
            longue > courte * 4.0,
            "une arche dix fois plus large ne monte qu'à {longue} contre {courte} : la hauteur ne suit pas la portée"
        );
        // Et le rapport reste le levier du réglage : à écart égal, une
        // puissance double monte deux fois plus haut.
        assert!(apex_pour(proches.0, proches.1, 2.0) > courte * 1.9);
    }

    /// **La hauteur reste bornée**, même sur des entrées aberrantes : elle
    /// pilote la géométrie dessinée, un emballement ferait sortir la boucle
    /// du système.
    #[test]
    fn la_hauteur_ne_s_emballe_pas() {
        let (a, b) = (vec3(1.0, 0.0, 0.0), vec3(-1.0, 0.0, 0.0)); // pieds opposés
        for rapport in [-5.0_f32, 0.0, 1.0, 100.0] {
            let h = apex_pour(a, b, rapport);
            assert!((0.02..=2.6).contains(&h), "rapport {rapport} -> {h}");
        }
        // Deux pieds confondus donnent un écart nul : la borne basse tient.
        assert_eq!(apex_pour(a, a, 1.0), 0.02);
    }

    /// **Les petites boucles ne prennent pas la place des grandes arches.**
    ///
    /// Sous un plafond commun, en faire naître plus revenait à en voir moins :
    /// les deux catégories se disputaient les mêmes places alors qu'une petite
    /// boucle est un fil court et fin, et une grande arche traverse la moitié
    /// d'une région.
    #[test]
    fn chaque_categorie_a_son_propre_plafond() {
        let courte = || {
            let mut b = boucle(vec3(1.0, 0.03, 0.0), vec3(1.0, -0.03, 0.0));
            b.courte = true;
            b
        };
        // Autant de petites boucles qu'il en tient : les arches gardent leurs
        // places.
        let pleines: Vec<Boucle> = (0..MAX_COURTES).map(|_| courte()).collect();
        assert!(categorie_pleine(&pleines, true), "les petites devraient être au complet");
        assert!(
            !categorie_pleine(&pleines, false),
            "les petites boucles ont mangé les places des grandes arches"
        );
        // Et réciproquement.
        let arches: Vec<Boucle> = (0..MAX_ARCHES)
            .map(|_| boucle(vec3(1.0, 0.3, 0.0), vec3(1.0, -0.3, 0.0)))
            .collect();
        assert!(categorie_pleine(&arches, false));
        assert!(!categorie_pleine(&arches, true), "une arche bloque une petite boucle");
        // Il y a bien plus de place pour les petites : c'est tout l'objet.
        const { assert!(MAX_COURTES > MAX_ARCHES) };
        assert!(!categorie_pleine(&[], true) && !categorie_pleine(&[], false));
    }

    /// **Deux petites boucles se serrent plus que deux grandes arches.**
    ///
    /// La répulsion valait un angle **fixe** pour tout le monde, ce qui
    /// interdisait aux petites boucles de se grouper alors que leurs halos
    /// sont petits eux aussi — le calibre suit l'étendue
    /// (`rendu::RAPPORT_TUBE`). C'est l'écart du **tube** qui compte, pas
    /// celui de la boucle.
    #[test]
    fn une_petite_boucle_se_serre_plus_qu_une_grande_arche() {
        // Deux boucles côte à côte, séparées de 0,05 rad.
        let decale = |p: Vec3| (p + vec3(0.0, 0.0, 0.05)).normalize();
        let petite = |o: f32| {
            let a = vec3(1.0, 0.03, o).normalize();
            let b = vec3(1.0, -0.03, o).normalize();
            (a, b)
        };
        let grande = |o: f32| {
            let a = vec3(1.0, 0.3, o).normalize();
            let b = vec3(1.0, -0.3, o).normalize();
            (a, b)
        };
        let (pa, pb) = petite(0.0);
        let (ga, gb) = grande(0.0);
        assert!(
            !arche_trop_proche(&[boucle(decale(pa), decale(pb))], &pieds(pa, pb)),
            "deux petites boucles voisines se repoussent : aucun groupe ne peut se former"
        );
        assert!(
            arche_trop_proche(&[boucle(decale(ga), decale(gb))], &pieds(ga, gb)),
            "deux grandes arches à la même distance se tolèrent : le croissant revient"
        );
    }

    /// **Les places forment un anneau**, pas un segment : un glissement qui
    /// dépasse revient de l'autre bord au lieu de s'écraser contre lui — sans
    /// quoi les arches s'entasseraient aux deux extrémités de l'arcade.
    #[test]
    fn les_places_de_l_arcade_s_enroulent() {
        assert!((enrouler(0.4) - 0.4).abs() < 1e-6);
        assert!((enrouler(1.4) - (-0.6)).abs() < 1e-6);
        assert!((enrouler(-1.4) - 0.6).abs() < 1e-6);
        for t in [-3.0_f32, -1.0, 0.0, 1.0, 2.7] {
            assert!((-1.0..=1.0).contains(&enrouler(t)), "{t} sort de l'arcade");
        }
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
