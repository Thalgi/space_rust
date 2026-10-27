mod rendu;
pub mod trajectoire;

use crate::astre::{Astre, CameraInfo, Categorie, Foyer};
use crate::stellaire::ArbreStellaire;
use macroquad::prelude::*;

pub const G: f32 = 1.0; // constante gravitationnelle

/// Le système : un ensemble d'astres soumis à la gravité mutuelle.
pub struct Systeme {
    astres: Vec<Box<dyn Astre>>,
    /// Lumière de secours (pos, couleur) utilisée quand il n'y a pas d'étoile
    /// (ex. vue d'une planète isolée). Ignorée dès qu'une étoile est présente.
    lumiere_manuelle: Option<(Vec3, Vec3)>,
    /// Temps de simulation cumulé (secondes) : sert à l'évaluation analytique `f(t)`.
    temps: f64,
    /// Arbre stellaire hiérarchique (systèmes multiples). Absent = étoile unique fixe.
    arbre: Option<ArbreStellaire>,
    /// Vue par défaut suggérée : (astre à focaliser, distance caméra). Utilisé pour
    /// cadrer d'emblée la zone planétaire d'un système type S (trop étalé sinon).
    vue: Option<(usize, f32)>,
}

/// **Les calques d'aide à la lecture** de la vue système : ce qu'on superpose
/// aux corps pour comprendre le système sans le survoler.
///
/// Regroupés plutôt que passés en quatre booléens de suite : quatre `bool`
/// positionnels du même type s'intervertissent sans que le compilateur bronche,
/// et c'est le défaut qu'on venait de corriger sur les deux horloges. Nommés,
/// ils se lisent à l'appel.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Calques {
    /// Trajectoires des planètes, centrées sur leur foyer stellaire.
    pub planetes: bool,
    /// Trajectoires des planètes en **trait plein gris** plutôt qu'en semis
    /// de points sarcelle (Skymap V2).
    ///
    /// Un interrupteur plutôt qu'un changement pur et simple : la V1 est le
    /// repli tant que la V2 n'est pas adoptée, et elle doit rendre
    /// exactement comme avant.
    pub planetes_pleines: bool,
    /// Orbites des étoiles entre elles (systèmes multiples).
    pub etoiles: bool,
    /// Trajectoires des **comètes**, à part de celles des planètes
    /// (2026-10-20) : une orbite cométaire traverse tout le système, et sur un
    /// système garni elle relie visuellement des planètes qui n'ont rien à
    /// voir entre elles. Son propre interrupteur, et une teinte plus sombre.
    pub cometes: bool,
    /// Orbites des lunes, centrées sur leur planète.
    pub lunes: bool,
    /// Anneaux de la zone habitable.
    pub zone: bool,
    /// Astre focalisé par la caméra, s'il y en a un.
    ///
    /// Les traces qui ne lui sont **pas liées** s'effacent presque : sur un
    /// système garni, une douzaine de cercles superposés empêchent de voir
    /// lequel appartient au corps qu'on vient de désigner. Voir
    /// `rendu::liee_au_focus` pour ce que « lié » veut dire.
    pub focus: Option<usize>,
    /// Aperçu de trajet en cours d'armement (Lot D,
    /// `docs/conception/trajectoires.md` §6) : la polyligne d'une
    /// [`trajectoire::Trajectoire`] calculée à titre d'essai vers l'astre
    /// survolé, jamais confirmée. `None` hors armement ou sans destination
    /// survolée valide — voir `Skymap::previsualisation_trajectoire`.
    ///
    /// Ce n'est **pas** un plan de manœuvre (décision 1, `trajectoires.md`
    /// §4) : rien ici ne se règle, l'arc affiché est celui qu'un lancement
    /// immédiat produirait, point.
    pub previsualisation: Option<Vec<Vec3>>,
}

/// **Horloge de spectacle** : le temps qui fait vivre l'animation propre des
/// astres (éruptions solaires, taches, rotation du voile des anneaux).
///
/// C'est un type à part, et pas un `f32`, pour une raison éprouvée : les deux
/// horloges de `Systeme::update` sont des durées en secondes, donc
/// interchangeables à l'œil du compilateur. Passer la cadence orbitale aux deux
/// compilait sans un mot — et c'est précisément le défaut qu'on corrige ici. Un
/// test ne peut pas garder ça : il vérifie le contrat de `update`, pas le
/// câblage de chaque vue. Le type, lui, le refuse à la compilation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DtAnimation(pub f32);

impl Systeme {
    pub fn new() -> Self {
        Self {
            astres: Vec::new(),
            lumiere_manuelle: None,
            temps: 0.0,
            arbre: None,
            vue: None,
        }
    }

    /// Installe l'arbre stellaire hiérarchique (systèmes multiples).
    pub fn definir_arbre(&mut self, arbre: ArbreStellaire) {
        self.arbre = Some(arbre);
    }

    /// Définit la vue par défaut : focaliser l'astre `idx` à la distance `dist`.
    pub fn definir_vue(&mut self, idx: usize, dist: f32) {
        self.vue = Some((idx, dist));
    }

    /// Vue par défaut suggérée, si présente : (astre à focaliser, distance caméra).
    pub fn vue(&self) -> Option<(usize, f32)> {
        self.vue
    }

    /// Rayon englobant approximatif (unités monde), pour cadrer la caméra sur un
    /// système dont l'étendue varie beaucoup (mono-étoile compact ↔ multiple large).
    pub fn rayon_englobant(&self) -> f32 {
        let mut r: f32 = 1.0;
        for a in &self.astres {
            r = r.max(a.corps().position.length());
            for p in a.orbite() {
                r = r.max(p.length());
            }
        }
        if let Some(arbre) = &self.arbre {
            for poly in arbre.orbites_etoiles(self.temps) {
                for p in &poly {
                    r = r.max(p.length());
                }
            }
        }
        r
    }

    /// Définit une lumière directionnelle de secours (sans étoile dans la scène).
    pub fn set_lumiere(&mut self, pos: Vec3, couleur: Vec3) {
        self.lumiere_manuelle = Some((pos, couleur));
    }

    pub(crate) fn lumiere_secours(&self) -> Option<(Vec3, Vec3)> {
        self.lumiere_manuelle
    }

    /// Ajoute un astre et renvoie son index (utile pour rattacher des lunes).
    pub fn ajouter(&mut self, a: Box<dyn Astre>) -> usize {
        self.astres.push(a);
        self.astres.len() - 1
    }

    /// Lance l'astre `idx` sur `trajectoire`, **au même index** — décision 8
    /// (`docs/conception/trajectoires.md` §4). Les indices servent de
    /// références stables ailleurs (foyer, parent, focus caméra, fiche
    /// ouverte) ; `Vec::remove` + `Vec::insert` au même point remet tout en
    /// place sans en décaler aucun, malgré le passage par `Astre::
    /// tenter_lancer` qui a besoin de posséder l'astre un court instant.
    ///
    /// Rend `false` (rien ne bouge) si `idx` est hors bornes ou si l'astre
    /// n'est pas pilotable (`tenter_lancer` rend `None` — voir
    /// [`Astre::pilotable`]).
    pub fn lancer(&mut self, idx: usize, trajectoire: trajectoire::Trajectoire) -> bool {
        if idx >= self.astres.len() {
            return false;
        }
        let mut astre = self.astres.remove(idx);
        let lance = astre.tenter_lancer(trajectoire);
        let reussi = lance.is_some();
        self.astres.insert(idx, lance.unwrap_or(astre));
        reussi
    }

    /// Nombre d'astres. Le sélecteur en a besoin pour parcourir le système.
    pub fn nb_astres(&self) -> usize {
        self.astres.len()
    }

    /// Catégorie de l'astre `idx`, `None` si l'index est invalide.
    pub fn categorie_de(&self, idx: usize) -> Option<Categorie> {
        self.astres.get(idx).map(|a| a.categorie())
    }

    /// Parent de l'astre `idx` (une lune orbite une planète), `None` s'il n'en
    /// a pas ou si l'index est invalide.
    pub fn parent_de(&self, idx: usize) -> Option<usize> {
        self.astres.get(idx).and_then(|a| a.parent())
    }

    /// **Nom du système** : celui de son étoile hôte, si elle en porte un.
    ///
    /// Dérivé des astres, et non d'une chaîne libre tenue à part : depuis que
    /// les corps portent leur nom (I.1), le titre de la vue n'a plus de raison
    /// d'être une seconde source. `None` pour un système engendré, dont les
    /// étoiles n'ont pas de nom propre — l'écran retombe alors sur son libellé
    /// de génération.
    pub fn nom_systeme(&self) -> Option<&'static str> {
        self.astres
            .iter()
            .find(|a| a.categorie() == Categorie::Etoile && a.nom().is_some())
            .and_then(|a| a.nom())
    }

    /// Rayon **visuel** : anneau compris s'il y en a un. Sert au cadrage de la
    /// vignette, où un corps à anneau doit être vu de plus loin.
    pub fn rayon_visuel_de(&self, idx: usize) -> Option<f32> {
        self.astres.get(idx).map(|a| a.corps().rayon * a.etendue_visuelle())
    }

    /// Normale du plan de l'anneau de l'astre `idx`, s'il en a un — voir
    /// [`Astre::normale_anneau`].
    pub fn normale_anneau_de(&self, idx: usize) -> Option<Vec3> {
        self.astres.get(idx).and_then(|a| a.normale_anneau())
    }

    /// Le rendu de l'astre `idx` est-il définitif ? Faux pour un index
    /// invalide (par prudence : l'appelant doit de toute façon déjà avoir
    /// vérifié l'index ailleurs) — voir [`Astre::rendu_pret`].
    pub fn rendu_pret_de(&self, idx: usize) -> bool {
        self.astres.get(idx).is_some_and(|a| a.rendu_pret())
    }

    /// Position de la lumière principale — l'étoile hôte. La vignette s'en sert
    /// pour se placer du **côté éclairé** : de face contre la lumière on ne
    /// verrait qu'un croissant, et à contre-jour un disque noir.
    pub fn position_lumiere(&self) -> Vec3 {
        self.lumiere_principale().0
    }

    /// Dessine **un seul** astre, avec l'éclairage complet du système.
    ///
    /// Réutilise le calcul d'éclairage de `draw_corps` plutôt que d'en refaire
    /// un : la vignette doit montrer le corps sous la même lumière que la vue,
    /// sinon le portrait ne ressemble pas à ce qu'on voit.
    pub fn dessiner_astre(&mut self, idx: usize, cam: &CameraInfo) {
        if idx >= self.astres.len() {
            return;
        }
        let c = self.eclairage(*cam);
        self.astres[idx].draw(&c);
    }

    /// Foyer de l'astre `idx` : autour de quoi il orbite. `None` s'il n'en a
    /// pas (une étoile, une ceinture) ou si l'index est invalide.
    ///
    /// C'est ce qui donne sa **hiérarchie** au sélecteur : une planète de type S
    /// se range sous son étoile hôte, une circumbinaire sous le barycentre.
    pub fn foyer_de(&self, idx: usize) -> Option<Foyer> {
        self.astres.get(idx).and_then(|a| a.foyer())
    }

    /// Horloge de simulation courante (secondes), la même base que
    /// [`Systeme::position_future_de`] — sert à calculer une date d'arrivée
    /// future (`temps() + duree_du_trajet`).
    #[allow(dead_code)] // Consommé par l'UI de départ du Lot C (trajectoires.md §6), pas encore câblé.
    pub fn temps(&self) -> f64 {
        self.temps
    }

    /// Position **prédite** de l'astre `idx` au temps absolu `t`, sans
    /// avancer la simulation — voir [`Astre::position_future`]. `None` si
    /// l'index est invalide ou si l'astre n'a pas d'orbite de Kepler connue
    /// (lunes, engins, ceintures — voir `docs/conception/trajectoires.md`
    /// §7).
    ///
    /// Le foyer est pris à sa position **actuelle**, pas prédite : correct
    /// pour une étoile fixe (système simple, le cas par défaut), mais
    /// approximatif si elle bouge elle-même (système multiple, où l'arbre
    /// stellaire la déplace) — dette assumée, pas un oubli, voir le même §7.
    #[allow(dead_code)] // Consommé par le Lot C (trajectoires.md §6), pas encore câblé au jeu.
    pub fn position_future_de(&self, idx: usize, t: f64) -> Option<Vec3> {
        let a = self.astres.get(idx)?;
        let foyer = match a.foyer() {
            Some(Foyer::Etoile(i)) => self.astres.get(i)?.corps().position,
            _ => Vec3::ZERO,
        };
        a.position_future(foyer, t)
    }

    /// Résout une [`trajectoire::Trajectoire`] de `depart` vers l'astre
    /// `destination`, en tenant compte du fait que la cible **bouge**.
    ///
    /// Point fixe sur la durée : la position d'arrivée dépend de la durée du
    /// trajet, qui dépend elle-même de la distance à la position d'arrivée.
    /// La position prédite est une fonction continue de `t` (jamais
    /// chaotique, voir `position_future_de`), donc la boucle converge —
    /// mais **pas forcément vite** : dans le pire cas (cible qui s'éloigne
    /// droit devant le vaisseau), l'erreur ne se réduit qu'à un facteur
    /// `vitesse_cible / vitesse_vaisseau` par itération. Mesuré : une cible
    /// à la moitié de la vitesse du vaisseau demande une quinzaine
    /// d'itérations pour retomber sous 1 cm d'écart — d'où la marge prise
    /// ici plutôt qu'un « quelques itérations » optimiste. Le coût reste
    /// négligeable : chaque itération n'est qu'un appel à
    /// `position_future_de`, pas une simulation.
    ///
    /// `None` si `destination` n'a pas de position prédictible
    /// (`position_future_de` rend `None` — une lune, un engin, une ceinture,
    /// pas encore couverts, voir `docs/conception/trajectoires.md` §7).
    pub fn calculer_trajectoire(
        &self,
        destination: usize,
        depart: Vec3,
        t_depart: f64,
        vitesse: f32,
        normale_plan: Vec3,
    ) -> Option<trajectoire::Trajectoire> {
        const ITERATIONS: usize = 24;
        let mut t_arrivee_estime = t_depart;
        let mut position_arrivee = self.position_future_de(destination, t_arrivee_estime)?;
        for _ in 0..ITERATIONS {
            let distance = (position_arrivee - depart).length();
            let duree = if vitesse > 1e-4 { (distance / vitesse) as f64 } else { 0.0 };
            t_arrivee_estime = t_depart + duree;
            position_arrivee = self.position_future_de(destination, t_arrivee_estime)?;
        }
        Some(trajectoire::Trajectoire::nouvelle(depart, position_arrivee, t_depart, vitesse, normale_plan))
    }

    /// Nombre d'étoiles du système. Décide de la forme de l'arbre : à une seule
    /// étoile, tout se range sous elle et le barycentre n'a pas lieu d'être.
    #[allow(dead_code)] // Interrogation gardee par les tests du selecteur ; aucune vue ne la lit.
    pub fn nb_etoiles(&self) -> usize {
        self.astres.iter().filter(|a| a.categorie() == Categorie::Etoile).count()
    }

    /// Teinte propre de l'astre `idx`, tirée de son apparence. `None` s'il n'en
    /// a pas (ceinture) ou si l'index est invalide.
    pub fn teinte_de(&self, idx: usize) -> Option<Vec3> {
        self.astres.get(idx).and_then(|a| a.teinte())
    }

    /// Rayon de l'astre `idx`, `None` si l'index est invalide.
    pub fn rayon_de(&self, idx: usize) -> Option<f32> {
        self.astres.get(idx).map(|a| a.corps().rayon)
    }

    /// L'astre `idx` est-il un vaisseau **pilotable** ? Faux si l'index est
    /// invalide — voir [`Astre::pilotable`].
    pub fn pilotable_de(&self, idx: usize) -> bool {
        self.astres.get(idx).is_some_and(|a| a.pilotable())
    }

    /// Ses statistiques de vol, s'il est pilotable — voir
    /// [`Astre::stats_vol`].
    pub fn stats_vol_de(&self, idx: usize) -> Option<crate::vaisseau::vol::StatsVol> {
        self.astres.get(idx).and_then(|a| a.stats_vol())
    }

    /// L'astre `idx` est-il une géante gazeuse ? `None` si l'index est
    /// invalide ou si l'astre n'a pas de type planétaire (étoile, lune,
    /// ceinture, engin) — voir [`Astre::gazeuse`].
    pub fn gazeuse_de(&self, idx: usize) -> Option<bool> {
        self.astres.get(idx).and_then(|a| a.gazeuse())
    }

    /// Le type de l'astre `idx`, s'il en a un — voir
    /// [`Astre::classe_planete`].
    pub fn classe_planete_de(&self, idx: usize) -> Option<crate::planete::classe::ClassePlanete> {
        self.astres.get(idx).and_then(|a| a.classe_planete())
    }

    /// **Luminosité cumulée** de toutes les étoiles du système.
    ///
    /// C'est la grandeur dont dépend la zone habitable, y compris circumbinaire
    /// — `systeme/rendu.rs` somme déjà de la même façon pour la tracer. Une
    /// seule source : la fiche d'astre et le tracé ne peuvent pas diverger.
    pub fn luminosite_totale(&self) -> f32 {
        self.astres.iter().filter_map(|a| a.luminosite()).sum()
    }

    /// **Les étoiles typées auxquelles l'astre `idx` est exposé**, avec leur
    /// distance en UA — de quoi juger son habitabilité en contexte
    /// (`planete::contexte`).
    pub fn etoiles_exposees(&self, idx: usize) -> Vec<(&'static str, f32)> {
        let p = self.position(idx);
        (0..self.astres.len())
            .filter_map(|i| {
                let t = self.astres[i].type_etoile()?;
                Some((t, (self.position(i) - p).length() / crate::etoile::UA))
            })
            .collect()
    }

    /// Donne un nom propre à l'astre `idx`. Sans effet si l'index est invalide.
    ///
    /// Réservé aux presets écrits à la main : voir [`Self::designation`] pour ce
    /// qui arrive aux autres.
    pub fn nommer(&mut self, idx: usize, nom: &'static str) {
        if let Some(a) = self.astres.get_mut(idx) {
            a.corps_mut().nom = Some(nom);
        }
    }

    /// **Comment on désigne un astre à l'écran** : son nom propre s'il en a un,
    /// sinon sa place dans le système.
    ///
    /// Le repli n'est pas un pis-aller : c'est la convention astronomique
    /// réelle. Une planète sans nom est « III », la deuxième lune de la III est
    /// « III-2 ». On préfère un rang exact à un nom inventé — le rang, lui, est
    /// vrai (`docs/conception/interface.md` §2.2a).
    ///
    /// Le rang se compte **par distance au foyer**, pas par ordre d'ajout : un
    /// preset qui déclare ses planètes dans le désordre doit quand même donner
    /// I à la plus proche. C'est ce qui distingue une désignation d'un simple
    /// index.
    pub fn designation(&self, idx: usize) -> String {
        let Some(a) = self.astres.get(idx) else { return String::new() };
        if let Some(n) = a.nom() {
            return n.to_string();
        }
        match a.categorie() {
            Categorie::Etoile => match self.rang(idx) {
                // Une seule étoile : elle n'a pas besoin d'être numérotée.
                Some(0) if self.nb_de(Categorie::Etoile, None) == 1 => "ETOILE".to_string(),
                Some(r) => format!("ETOILE {}", romain(r + 1)),
                None => "ETOILE".to_string(),
            },
            Categorie::Planete if a.parent().is_some() => {
                // Une lune : rang **sous son parent**, désigné par le parent.
                let parent = a.parent().unwrap_or(0);
                match self.rang(idx) {
                    Some(r) => format!("{}-{}", self.designation(parent), r + 1),
                    None => self.designation(parent),
                }
            }
            Categorie::Planete => match self.rang(idx) {
                Some(r) => romain(r + 1),
                None => "?".to_string(),
            },
            Categorie::Lune => {
                let parent = a.parent().unwrap_or(0);
                match self.rang(idx) {
                    Some(r) => format!("{}-{}", self.designation(parent), r + 1),
                    None => self.designation(parent),
                }
            }
            Categorie::Asteroide => "CEINTURE".to_string(),
            Categorie::Comete => "COMETE".to_string(),
            Categorie::Engin => "ENGIN".to_string(),
        }
    }

    /// Rang de `idx` parmi ses pairs — même catégorie **et** même parent —,
    /// classés par distance au corps de référence (le parent, ou l'origine).
    ///
    /// `None` si l'index est invalide.
    fn rang(&self, idx: usize) -> Option<usize> {
        let a = self.astres.get(idx)?;
        let (cat, parent) = (a.categorie(), a.parent());
        let centre = parent.map_or(Vec3::ZERO, |p| self.position(p));
        let d = |i: usize| (self.position(i) - centre).length();
        let mien = d(idx);
        let mut rang = 0;
        for (i, b) in self.astres.iter().enumerate() {
            if i == idx || b.categorie() != cat || b.parent() != parent {
                continue;
            }
            // Départage par index à distance égale : sans ça, deux corps
            // co-orbitaux (Phobos et Déimos partagent un rayon) recevraient le
            // même rang, et deux entrées du sélecteur seraient identiques.
            let db = d(i);
            if db < mien || (db == mien && i < idx) {
                rang += 1;
            }
        }
        Some(rang)
    }

    /// Combien d'astres de cette catégorie (et de ce parent, si précisé).
    fn nb_de(&self, cat: Categorie, parent: Option<usize>) -> usize {
        self.astres
            .iter()
            .filter(|a| a.categorie() == cat && (parent.is_none() || a.parent() == parent))
            .count()
    }

    /// Nombre de lunes déjà attachées à l'astre `parent`. Sert à `ajouter_lune`
    /// pour placer chaque nouvelle lune sur un créneau orbital croissant (système
    /// emboîté, sans chevauchement) plutôt qu'à un rayon aléatoire indépendant.
    pub fn nb_lunes(&self, parent: usize) -> usize {
        self.astres.iter().filter(|a| a.parent() == Some(parent)).count()
    }

    /// **Indices des enfants directs** de `parent` : tout astre dont
    /// `parent()` le désigne, dans l'ordre de `self.astres`.
    ///
    /// Générique à dessein : pas de filtre de catégorie. Une lune en est un
    /// enfant, mais aussi un engin posté autour de la planète (l'ISS autour de
    /// la Terre). C'est à l'appelant de choisir ce qu'il garde — les marqueurs
    /// de la Skymap V2 ne gardent que les lunes.
    pub fn lunes_de(&self, parent: usize) -> Vec<usize> {
        self.astres
            .iter()
            .enumerate()
            .filter(|(_, a)| a.parent() == Some(parent))
            .map(|(i, _)| i)
            .collect()
    }

    /// Polyligne d'orbite d'une lune, **relative à son parent** (vide pour
    /// tout autre astre, ou pour un index invalide) — voir
    /// [`Astre::orbite_lune`].
    ///
    /// Exposée pour le tracé en 2D de la Skymap V2 (`ecran/trace_orbite.rs`),
    /// qui a besoin des points eux-mêmes et non d'un appel de dessin : un
    /// trait large ne peut pas se faire en 3D (`draw_line_3d` est à un pixel).
    pub fn orbite_lune_de(&self, idx: usize) -> &[Vec3] {
        self.astres.get(idx).map_or(&[], |a| a.orbite_lune())
    }

    /// Position d'un astre par index (origine si invalide).
    pub fn position(&self, idx: usize) -> Vec3 {
        self.astres
            .get(idx)
            .map(|a| a.corps().position)
            .unwrap_or(Vec3::ZERO)
    }

    /// Sélection au rayon : renvoie l'astre touché le plus proche (hors ceinture).
    pub fn pick(&self, origine: Vec3, dir: Vec3) -> Option<usize> {
        let mut best: Option<(f32, usize)> = None;
        for (i, a) in self.astres.iter().enumerate() {
            if a.categorie() == Categorie::Asteroide {
                continue;
            }
            let centre = a.corps().position;
            // Rayon DESSINÉ, pas physique : l'étoile est agrandie par un
            // plancher en pixels, et une cible de clic restée sur son rayon vrai
            // ne couvrirait qu'un cinquième du disque visible.
            let rayon = a.rayon_dessine(origine).max(0.3) * 1.4; // marge pour cliquer facilement
            let oc = centre - origine;
            let tca = oc.dot(dir);
            if tca < 0.0 {
                continue; // derrière la caméra
            }
            let d2 = oc.length_squared() - tca * tca;
            let rr = rayon * rayon;
            if d2 <= rr {
                let t = tca - (rr - d2).sqrt();
                if best.is_none_or(|(bt, _)| t < bt) {
                    best = Some((t, i));
                }
            }
        }
        best.map(|(_, i)| i)
    }

    /// Avance le système. **Deux horloges, et elles ne mesurent pas la même chose.**
    ///
    /// - `dt` : temps **orbital**. Multiplié par la vitesse de jeu, parce qu'une
    ///   période de Kepler se compte en années et qu'il faut bien la voir tourner.
    /// - `dt_anim` : temps de l'**animation propre** des astres — éruptions
    ///   solaires, taches, rotation du voile des anneaux.
    ///
    /// Les deux étaient confondues. Les rythmes d'éruption sont pourtant réglés
    /// à la main (une tache vit 8 à 16 s, une boucle monte en ~2 s) : ce sont des
    /// durées **de spectacle**, pas de physique. Passées au multiplicateur
    /// orbital, à ×16 une tache naît et meurt en une demi-seconde et les boucles
    /// stroboscopent — on ne lit plus une protubérance, on lit du bruit.
    pub fn update(&mut self, dt: f32, dt_anim: DtAnimation) {
        self.temps += dt as f64;
        let t = self.temps;

        // Étoiles : positions issues de l'arbre stellaire hiérarchique (barycentres
        // composés). Étoile unique = pas d'arbre -> reste à sa position (origine).
        if let Some(arbre) = self.arbre.take() {
            arbre.evaluer(t, &mut self.astres);
            self.arbre = Some(arbre);
        }

        // Planètes « sur rails » : chacune orbite son foyer (étoile hôte S-type
        // ou barycentre P-type). Étoiles déjà repositionnées juste au-dessus.
        let pos: Vec<Vec3> = self.astres.iter().map(|a| a.corps().position).collect();
        for a in &mut self.astres {
            // ⚠️ **Le critère est d'avoir un foyer, pas d'être une planète.**
            //
            // La condition portait sur `Categorie::Planete`, ce qui revenait à
            // supposer « sur rails ⇒ planète ». Dès qu'une **comète** est arrivée
            // avec sa propre catégorie (lot 9.1), elle a cessé d'être avancée :
            // posée à son périastre, elle y restait pour toujours. Signalé à
            // l'écran — « la comète n'a pas de mouvement ».
            //
            // C'est le piège de la catégorie morte **par l'autre bout** : rendre
            // la catégorie juste a suffi à exclure l'astre d'un traitement qui le
            // concernait. Un filtre doit porter sur ce que l'astre **sait faire**,
            // jamais sur ce qu'il est. `maj_rail` est de toute façon inerte pour
            // qui n'a pas d'orbite de Kepler.
            if let Some(foyer) = a.foyer() {
                let f = match foyer {
                    Foyer::Etoile(i) => pos.get(i).copied().unwrap_or(Vec3::ZERO),
                    _ => Vec3::ZERO, // Barycentre
                };
                a.maj_rail(f, t);
            }
        }

        // Animation propre de chaque astre (éruptions du soleil, etc.) : horloge
        // de spectacle, pas l'horloge orbitale (voir la doc de cette fonction).
        for a in &mut self.astres {
            a.update(dt_anim.0);
        }

        // Lunes : orbite analytique autour de leur parent (positions courantes).
        let pos: Vec<Vec3> = self.astres.iter().map(|a| a.corps().position).collect();
        for a in &mut self.astres {
            if let Some(p) = a.parent() {
                a.orbiter_autour(pos[p], dt);
            }
        }

        // Vaisseaux en transit (Lot B) : suivent une trajectoire déjà
        // connue d'avance, ni foyer ni parent à consulter — `t` suffit.
        for a in &mut self.astres {
            a.suivre_trajectoire(t);
        }
    }

    /// Transmet les réglages d'éruptions à l'étoile.
    pub fn reglages_etoile(&mut self, freq: f32, forme: f32, puissance: f32, alea: f32) {
        for a in &mut self.astres {
            if a.categorie() == Categorie::Etoile {
                a.set_eruptions(freq, forme, puissance, alea);
            }
        }
    }
}

/// Chiffre romain de `n` (1 → « I »). Rend la décimale au-delà de la table :
/// une planète au-delà du rang 39 est plus lisible en chiffres qu'en `XXXX`.
///
/// Sert à la désignation des astres sans nom propre (`Systeme::designation`).
pub fn romain(n: usize) -> String {
    const TABLE: [(usize, &str); 6] =
        [(10, "X"), (9, "IX"), (5, "V"), (4, "IV"), (1, "I"), (0, "")];
    if n == 0 || n > 39 {
        return n.to_string();
    }
    let mut reste = n;
    let mut out = String::new();
    for (valeur, signe) in TABLE {
        while reste >= valeur.max(1) && valeur > 0 {
            out.push_str(signe);
            reste -= valeur;
        }
    }
    out
}


#[cfg(test)]
mod tests_designation {
    use super::*;
    use crate::astre::{CameraInfo, CorpsBase};

    // ⚠️ **Aucun test ne peut construire un vrai systeme.** `genese` tire des
    // nombres aleatoires par `macroquad::rand`, qui exige le contexte graphique
    // (`THREAD_ID.is_some()`) : hors boucle de rendu, tout appel a
    // `construire_systeme` ou `construire_preset_*` panique. C'est pourquoi
    // aucun test du depot n'en batit, et pourquoi celui-ci pose son propre
    // corps d'essai.
    //
    // Consequence assumee : la **numerotation** se teste ici de bout en bout,
    // mais le fait que le preset solaire porte bien « Mercure », « Titan », etc.
    // se verifie **a l'ecran** (etape I.2). C'est la meme limite que 6.6 sur le
    // rendu — ce qui ne se teste pas doit au moins etre dit.

    /// Corps minimal : juste ce qu'il faut pour que `Systeme` le classe et le
    /// situe. Ne dessine rien, ne tire aucun aleatoire.
    struct CorpsEssai {
        base: CorpsBase,
        cat: Categorie,
        parent: Option<usize>,
        /// Foyer d'orbite. Distinct de `parent` : une lune tient à sa planète
        /// par `parent`, une planète à son étoile par son foyer — c'est
        /// exactement la distinction qu'`attache` doit réconcilier.
        foyer: Option<Foyer>,
        /// Cumul des `dt` reçus par `Astre::update` — l'horloge d'animation.
        /// Partagé avec le test, qui ne peut pas lire à travers `Box<dyn Astre>`.
        dt_anim_recu: std::rc::Rc<std::cell::Cell<f32>>,
        /// Décalage fixe (indépendant de `t`) que `position_future` ajoute
        /// au foyer reçu, si posé — sert à vérifier que `Systeme` résout et
        /// transmet le **bon** foyer, sans avoir besoin d'une vraie orbite
        /// de Kepler pour ça (déjà testée dans `planete/mod.rs`).
        decalage_futur: Option<Vec3>,
        /// Dernier `t` reçu par `Astre::suivre_trajectoire` (-1 = jamais
        /// appelé). Partagé avec le test, même raison que `dt_anim_recu`.
        t_trajectoire_recu: std::rc::Rc<std::cell::Cell<f64>>,
        /// Dernier `t` reçu par `Astre::maj_rail` (-1 = jamais appelé).
        ///
        /// ⚠️ Posé après que la **comète du lot 9.1 est restée immobile** : la
        /// boucle « sur rails » ne traitait que `Categorie::Planete`, et rien ne
        /// disait qu'un astre à foyer pouvait être oublié.
        t_rail_recu: std::rc::Rc<std::cell::Cell<f64>>,
    }

    impl CorpsEssai {
        /// Construit un `CorpsEssai` sans le ranger dans un `Systeme` — pour
        /// `Systeme::remplacer`, qui prend l'astre déjà construit.
        fn neuf(cat: Categorie, x: f32) -> Self {
            Self {
                base: CorpsBase::new(vec3(x, 0.0, 0.0), 1.0, 1.0),
                cat,
                parent: None,
                foyer: None,
                dt_anim_recu: std::rc::Rc::new(std::cell::Cell::new(0.0)),
                decalage_futur: None,
                t_trajectoire_recu: std::rc::Rc::new(std::cell::Cell::new(-1.0)),
                t_rail_recu: std::rc::Rc::new(std::cell::Cell::new(-1.0)),
            }
        }

        fn poser(sys: &mut Systeme, cat: Categorie, x: f32, parent: Option<usize>) -> usize {
            Self::poser_mouchard(sys, cat, x, parent).0
        }

        /// Idem, en rendant le mouchard d'horloge d'animation.
        fn poser_mouchard(
            sys: &mut Systeme,
            cat: Categorie,
            x: f32,
            parent: Option<usize>,
        ) -> (usize, std::rc::Rc<std::cell::Cell<f32>>) {
            let mouchard = std::rc::Rc::new(std::cell::Cell::new(0.0));
            let idx = sys.ajouter(Box::new(CorpsEssai {
                base: CorpsBase::new(vec3(x, 0.0, 0.0), 1.0, 1.0),
                cat,
                parent,
                foyer: None,
                dt_anim_recu: mouchard.clone(),
                decalage_futur: None,
                t_trajectoire_recu: std::rc::Rc::new(std::cell::Cell::new(-1.0)),
                t_rail_recu: std::rc::Rc::new(std::cell::Cell::new(-1.0)),
            }));
            (idx, mouchard)
        }
    }

    impl Astre for CorpsEssai {
        fn categorie(&self) -> Categorie {
            self.cat
        }
        fn corps(&self) -> &CorpsBase {
            &self.base
        }
        fn corps_mut(&mut self) -> &mut CorpsBase {
            &mut self.base
        }
        fn parent(&self) -> Option<usize> {
            self.parent
        }
        fn foyer(&self) -> Option<Foyer> {
            self.foyer
        }
        /// Enregistre le `dt` reçu : c'est ce qui permet de vérifier quelle
        /// horloge alimente l'animation propre des astres.
        fn update(&mut self, dt: f32) {
            self.dt_anim_recu.set(self.dt_anim_recu.get() + dt);
        }
        fn draw(&mut self, _cam: &CameraInfo) {}
        fn position_future(&self, foyer_predit: Vec3, _t: f64) -> Option<Vec3> {
            self.decalage_futur.map(|d| foyer_predit + d)
        }
        /// Enregistre le `t` reçu — sert à vérifier que `Systeme::update`
        /// appelle vraiment `suivre_trajectoire` sur chaque astre, avec la
        /// bonne horloge absolue.
        fn suivre_trajectoire(&mut self, t: f64) {
            self.t_trajectoire_recu.set(t);
        }
        /// Enregistre le `t` reçu — sert à vérifier qu'un astre **à foyer** est
        /// bien avancé sur ses rails, quelle que soit sa catégorie.
        fn maj_rail(&mut self, _foyer: Vec3, t: f64) {
            self.t_rail_recu.set(t);
        }
    }

    /// **Tout astre à foyer est avancé sur ses rails, quelle que soit sa
    /// catégorie.**
    ///
    /// ⚠️ La boucle ne traitait que `Categorie::Planete` — « sur rails ⇒
    /// planète ». Dès qu'une **comète** est arrivée avec sa propre catégorie
    /// (lot 9.1), elle a cessé d'être avancée : posée à son périastre, elle y
    /// restait pour toujours, et sa queue avec elle. Signalé à l'écran.
    ///
    /// C'est le piège de la catégorie morte **par l'autre bout** : rendre la
    /// catégorie juste a suffi à exclure l'astre d'un traitement qui le
    /// concernait. Un filtre doit porter sur ce que l'astre **sait faire**.
    ///
    /// Le test balaie **toutes** les catégories, pour qu'une nouvelle venue
    /// n'ait pas à réapprendre la leçon.
    #[test]
    fn tout_astre_a_foyer_est_avance_sur_ses_rails() {
        for cat in [
            Categorie::Planete,
            Categorie::Comete,
            Categorie::Asteroide,
            Categorie::Lune,
            Categorie::Engin,
        ] {
            let mut sys = Systeme::new();
            let mut corps = CorpsEssai::neuf(cat, 5.0);
            corps.foyer = Some(Foyer::Barycentre);
            let mouchard = corps.t_rail_recu.clone();
            sys.ajouter(Box::new(corps));
            sys.update(0.5, DtAnimation(0.5));
            assert!(
                mouchard.get() >= 0.0,
                "{cat:?} : à foyer mais jamais avancé sur ses rails — il restera figé"
            );
        }
        // Et un astre **sans** foyer n'est pas appelé pour rien : une étoile n'a
        // pas d'orbite de Kepler, et l'y envoyer masquerait le vrai critère.
        let mut sys = Systeme::new();
        let corps = CorpsEssai::neuf(Categorie::Etoile, 0.0);
        let mouchard = corps.t_rail_recu.clone();
        sys.ajouter(Box::new(corps));
        sys.update(0.5, DtAnimation(0.5));
        assert_eq!(mouchard.get(), -1.0, "un astre sans foyer a été mis sur rails");
    }

    /// **`attache` réconcilie deux façons d'être en orbite.**
    ///
    /// Une lune tient à sa planète par `parent`, une planète à son étoile par son
    /// `foyer`. Deux champs, une seule idée — « ce corps tourne autour de
    /// celui-là ».
    ///
    /// ⚠️ Ce test existe parce que le précédent ne l'attrapait pas : il modélisait
    /// `parent` comme rendant déjà l'étoile d'une planète, ce que le vrai système
    /// ne fait pas. Il est donc resté **vert** alors que désigner une étoile
    /// effaçait toutes ses planètes. Un test qui postule ce qu'il devrait
    /// vérifier ne vérifie rien.
    #[test]
    fn l_attache_couvre_la_lune_par_parent_et_la_planete_par_foyer() {
        let mut sys = Systeme::new();
        let etoile = CorpsEssai::poser(&mut sys, Categorie::Etoile, 0.0, None);
        let planete = CorpsEssai::poser(&mut sys, Categorie::Planete, 10.0, None);
        let lune = CorpsEssai::poser(&mut sys, Categorie::Lune, 11.0, Some(planete));
        let errante = CorpsEssai::poser(&mut sys, Categorie::Planete, 50.0, None);

        // La planète ne tient à son étoile QUE par son foyer : `parent` est vide.
        assert_eq!(sys.astres[planete].parent(), None, "une planète n'a pas de parent");

        // Sans foyer posé, elle n'est encore rattachée à rien.
        assert_eq!(sys.attache(planete), None);

        // Avec le foyer stellaire, elle l'est — c'est le cas que la version
        // fautive manquait.
        poser_foyer(&mut sys, planete, Foyer::Etoile(etoile));
        assert_eq!(sys.attache(planete), Some(etoile));

        // La lune, elle, passe par `parent`.
        assert_eq!(sys.attache(lune), Some(planete));

        // Un foyer barycentrique ne désigne aucun astre.
        poser_foyer(&mut sys, errante, Foyer::Barycentre);
        assert_eq!(sys.attache(errante), None);

        // Et l'étoile n'est accrochée à rien.
        assert_eq!(sys.attache(etoile), None);
    }

    /// Pose un foyer sur un `CorpsEssai` déjà rangé dans le système.
    fn poser_foyer(sys: &mut Systeme, idx: usize, f: Foyer) {
        let c = CorpsEssai {
            base: CorpsBase::new(sys.position(idx), 1.0, 1.0),
            cat: sys.astres[idx].categorie(),
            parent: sys.astres[idx].parent(),
            foyer: Some(f),
            dt_anim_recu: std::rc::Rc::new(std::cell::Cell::new(0.0)),
            decalage_futur: None,
            t_trajectoire_recu: std::rc::Rc::new(std::cell::Cell::new(-1.0)),
            t_rail_recu: std::rc::Rc::new(std::cell::Cell::new(-1.0)),
        };
        sys.astres[idx] = Box::new(c);
    }

    // **`position_future_de` compose le bon foyer.** Une planète autour d'une
    // étoile doit recevoir la position de CETTE étoile, pas d'une autre — le
    // décalage renvoyé par le corps d'essai le prouve sans avoir besoin d'une
    // vraie orbite de Kepler (déjà testée dans `planete/mod.rs`).
    #[test]
    fn position_future_de_compose_le_bon_foyer() {
        let mut sys = Systeme::new();
        let etoile = CorpsEssai::poser(&mut sys, Categorie::Etoile, 100.0, None);
        let autre_etoile = CorpsEssai::poser(&mut sys, Categorie::Etoile, -100.0, None);
        let decalage = vec3(5.0, 0.0, 0.0);
        let planete = sys.ajouter(Box::new(CorpsEssai {
            base: CorpsBase::new(Vec3::ZERO, 1.0, 1.0),
            cat: Categorie::Planete,
            parent: None,
            foyer: Some(Foyer::Etoile(etoile)),
            dt_anim_recu: std::rc::Rc::new(std::cell::Cell::new(0.0)),
            decalage_futur: Some(decalage),
            t_trajectoire_recu: std::rc::Rc::new(std::cell::Cell::new(-1.0)),
            t_rail_recu: std::rc::Rc::new(std::cell::Cell::new(-1.0)),
        }));

        let attendu = sys.position(etoile) + decalage;
        assert_eq!(sys.position_future_de(planete, 42.0), Some(attendu));
        assert_ne!(attendu, sys.position(autre_etoile) + decalage, "les deux étoiles ne sont pas au même endroit");
    }

    // Un foyer barycentrique (P-type) ou un astre sans foyer (une étoile
    // elle-même) résolvent à l'origine, pas à une étoile au hasard.
    #[test]
    fn position_future_de_retombe_sur_lorigine_sans_etoile_hote() {
        let mut sys = Systeme::new();
        let decalage = vec3(3.0, 0.0, 0.0);
        let circumbinaire = sys.ajouter(Box::new(CorpsEssai {
            base: CorpsBase::new(Vec3::ZERO, 1.0, 1.0),
            cat: Categorie::Planete,
            parent: None,
            foyer: Some(Foyer::Barycentre),
            dt_anim_recu: std::rc::Rc::new(std::cell::Cell::new(0.0)),
            decalage_futur: Some(decalage),
            t_trajectoire_recu: std::rc::Rc::new(std::cell::Cell::new(-1.0)),
            t_rail_recu: std::rc::Rc::new(std::cell::Cell::new(-1.0)),
        }));
        assert_eq!(sys.position_future_de(circumbinaire, 42.0), Some(decalage));
    }

    // **`Systeme::update` appelle vraiment `suivre_trajectoire`**, avec
    // l'horloge absolue cumulée (`self.temps`), pas un `dt` brut ni rien du
    // tout — sinon un vaisseau en transit (Lot B) ne bougerait jamais.
    #[test]
    fn update_appelle_suivre_trajectoire_avec_le_temps_absolu() {
        let mut sys = Systeme::new();
        let mouchard = std::rc::Rc::new(std::cell::Cell::new(-1.0_f64));
        sys.ajouter(Box::new(CorpsEssai {
            base: CorpsBase::new(Vec3::ZERO, 1.0, 1.0),
            cat: Categorie::Asteroide,
            parent: None,
            foyer: None,
            dt_anim_recu: std::rc::Rc::new(std::cell::Cell::new(0.0)),
            decalage_futur: None,
            t_trajectoire_recu: mouchard.clone(),
            t_rail_recu: std::rc::Rc::new(std::cell::Cell::new(-1.0)),
        }));

        sys.update(3.0, DtAnimation(0.0));
        assert_eq!(mouchard.get(), 3.0, "temps absolu attendu après un premier pas de 3.0");
        sys.update(2.0, DtAnimation(0.0));
        assert_eq!(mouchard.get(), 5.0, "l'horloge doit cumuler, pas repartir de zéro");
    }

    // Un astre sans `decalage_futur` (donc sans orbite de Kepler connue —
    // typiquement une lune, pas encore couverte) rend `None`, pas une
    // position au hasard.
    #[test]
    fn position_future_de_est_absente_sans_orbite_connue() {
        let mut sys = Systeme::new();
        let etoile = CorpsEssai::poser(&mut sys, Categorie::Etoile, 0.0, None);
        let lune = CorpsEssai::poser(&mut sys, Categorie::Lune, 5.0, None);
        poser_foyer(&mut sys, lune, Foyer::Etoile(etoile));
        assert_eq!(sys.position_future_de(lune, 42.0), None);
    }

    // **`calculer_trajectoire` rend un vrai rendez-vous, pas une
    // approximation** : à l'heure d'arrivée qu'elle calcule, le vaisseau et
    // la cible sont au MÊME endroit — même si la cible a bougé pendant tout
    // le trajet. Cible synthétique dédiée (pas le `CorpsEssai` partagé) :
    // seul ce test a besoin d'une position qui dépend vraiment de `t`.
    #[test]
    fn calculer_trajectoire_rend_vraiment_rendez_vous_avec_une_cible_qui_bouge() {
        struct CibleMobile {
            base: CorpsBase,
        }
        impl Astre for CibleMobile {
            fn categorie(&self) -> Categorie {
                Categorie::Planete
            }
            fn corps(&self) -> &CorpsBase {
                &self.base
            }
            fn corps_mut(&mut self) -> &mut CorpsBase {
                &mut self.base
            }
            fn draw(&mut self, _cam: &CameraInfo) {}
            fn update(&mut self, _dt: f32) {}
            // Dérive à vitesse 1 le long de X : à l'instant `t`, elle est en (t, 0, 0).
            fn position_future(&self, _foyer_predit: Vec3, t: f64) -> Option<Vec3> {
                Some(vec3(t as f32, 0.0, 0.0))
            }
        }

        let mut sys = Systeme::new();
        let cible = sys.ajouter(Box::new(CibleMobile { base: CorpsBase::new(Vec3::ZERO, 1.0, 1.0) }));

        let depart = vec3(-100.0, 0.0, 0.0);
        let traj = sys.calculer_trajectoire(cible, depart, 0.0, 2.0, Vec3::Y).unwrap();

        let t_arrivee = traj.arrivee_a();
        let position_cible = sys.position_future_de(cible, t_arrivee).unwrap();
        let position_vaisseau = traj.position(t_arrivee);
        assert!(
            (position_vaisseau - position_cible).length() < 1e-2,
            "rendez-vous manqué : vaisseau à {position_vaisseau}, cible à {position_cible}"
        );
    }

    // Une destination sans position prédictible (pas encore couverte —
    // lune, engin, ceinture) fait échouer le calcul, pas une trajectoire
    // vers une position arbitraire.
    #[test]
    fn calculer_trajectoire_est_absente_sans_destination_predictible() {
        let mut sys = Systeme::new();
        let lune = CorpsEssai::poser(&mut sys, Categorie::Lune, 5.0, None);
        assert!(sys.calculer_trajectoire(lune, Vec3::ZERO, 0.0, 2.0, Vec3::Y).is_none());
    }

    // **`lancer` garde le même index et transforme vraiment l'astre
    // pilotable** — sert la décision 8 (`docs/conception/trajectoires.md`
    // §4) : ne doit décaler aucune référence (foyer, parent) tenue par un
    // AUTRE astre.
    #[test]
    fn lancer_garde_le_meme_index_et_transforme_lastre_pilotable() {
        struct VaisseauEssai {
            base: CorpsBase,
        }
        impl Astre for VaisseauEssai {
            fn categorie(&self) -> Categorie {
                Categorie::Engin
            }
            fn corps(&self) -> &CorpsBase {
                &self.base
            }
            fn corps_mut(&mut self) -> &mut CorpsBase {
                &mut self.base
            }
            fn draw(&mut self, _cam: &CameraInfo) {}
            fn update(&mut self, _dt: f32) {}
            fn pilotable(&self) -> bool {
                true
            }
            // Le résultat concret importe peu ici (`CorpsEssai` en guise de
            // « converti ») : ce test vérifie la PLOMBERIE (`Systeme::lancer`
            // au bon index), pas la conversion elle-même — déjà testée dans
            // `engin.rs`.
            fn tenter_lancer(&mut self, _trajectoire: trajectoire::Trajectoire) -> Option<Box<dyn Astre>> {
                Some(Box::new(CorpsEssai::neuf(Categorie::Asteroide, 1.0)))
            }
        }

        let mut sys = Systeme::new();
        let avant = sys.ajouter(Box::new(VaisseauEssai { base: CorpsBase::new(Vec3::ZERO, 1.0, 1.0) }));
        let apres_lui = CorpsEssai::poser(&mut sys, Categorie::Planete, 2.0, None);
        assert_eq!(sys.categorie_de(avant), Some(Categorie::Engin));

        let t = trajectoire::Trajectoire::nouvelle(Vec3::ZERO, Vec3::X, 0.0, 1.0, Vec3::Y);
        assert!(sys.lancer(avant, t));

        assert_eq!(sys.categorie_de(avant), Some(Categorie::Asteroide), "transformé au même index");
        assert_eq!(sys.categorie_de(apres_lui), Some(Categorie::Planete), "l'astre suivant n'a pas bougé");
        assert_eq!(sys.nb_astres(), 2, "aucun astre ajouté ni retiré");
    }

    // Un astre non pilotable (`CorpsEssai` par défaut) reste inchangé, et
    // `lancer` le signale en rendant `false` plutôt qu'en échouant en
    // silence.
    #[test]
    fn lancer_echoue_et_ne_change_rien_pour_un_astre_non_pilotable() {
        let mut sys = Systeme::new();
        let idx = CorpsEssai::poser(&mut sys, Categorie::Engin, 1.0, None);
        let t = trajectoire::Trajectoire::nouvelle(Vec3::ZERO, Vec3::X, 0.0, 1.0, Vec3::Y);
        assert!(!sys.lancer(idx, t));
        assert_eq!(sys.categorie_de(idx), Some(Categorie::Engin), "inchangé");
    }

    // Un index hors bornes ne panique pas — no-op silencieux, comme les
    // autres accesseurs `..._de` de `Systeme`.
    #[test]
    fn lancer_hors_bornes_ne_panique_pas() {
        let mut sys = Systeme::new();
        CorpsEssai::poser(&mut sys, Categorie::Engin, 1.0, None);
        let t = trajectoire::Trajectoire::nouvelle(Vec3::ZERO, Vec3::X, 0.0, 1.0, Vec3::Y);
        assert!(!sys.lancer(99, t));
        assert_eq!(sys.nb_astres(), 1);
    }

    // `pilotable_de`/`stats_vol_de` retombent sur les défauts du trait
    // (`false`/`None`) pour tout astre qui ne les redéfinit pas — un
    // `CorpsEssai` normal, ou un index invalide.
    #[test]
    fn pilotable_de_est_faux_par_defaut() {
        let mut sys = Systeme::new();
        let idx = CorpsEssai::poser(&mut sys, Categorie::Engin, 1.0, None);
        assert!(!sys.pilotable_de(idx));
        assert_eq!(sys.stats_vol_de(idx), None);
        assert!(!sys.pilotable_de(999), "index invalide : pas pilotable, pas de panique");
    }

    // Un astre qui redéfinit `pilotable`/`stats_vol` (comme `EnginOrbital`
    // pilotable) est bien lu à travers `Box<dyn Astre>` — c'est tout l'enjeu
    // de les avoir mis sur le trait plutôt que sur le type concret.
    #[test]
    fn pilotable_de_lit_vraiment_a_travers_le_trait() {
        struct VaisseauEssai {
            base: CorpsBase,
        }
        impl Astre for VaisseauEssai {
            fn categorie(&self) -> Categorie {
                Categorie::Engin
            }
            fn corps(&self) -> &CorpsBase {
                &self.base
            }
            fn corps_mut(&mut self) -> &mut CorpsBase {
                &mut self.base
            }
            fn draw(&mut self, _cam: &CameraInfo) {}
            fn update(&mut self, _dt: f32) {}
            fn pilotable(&self) -> bool {
                true
            }
            fn stats_vol(&self) -> Option<crate::vaisseau::vol::StatsVol> {
                Some(crate::vaisseau::vol::StatsVol { vitesse: 7.0, portee: 8.0, capacite_fret: 9.0 })
            }
        }

        let mut sys = Systeme::new();
        let idx = sys.ajouter(Box::new(VaisseauEssai { base: CorpsBase::new(Vec3::ZERO, 1.0, 1.0) }));
        assert!(sys.pilotable_de(idx));
        assert_eq!(
            sys.stats_vol_de(idx),
            Some(crate::vaisseau::vol::StatsVol { vitesse: 7.0, portee: 8.0, capacite_fret: 9.0 })
        );
    }

    /// **L'animation des astres ne suit pas l'horloge orbitale.**
    ///
    /// Le multiplicateur de vitesse existe pour faire tourner les orbites : une
    /// période de Kepler se compte en années. Les rythmes d'éruption solaire,
    /// eux, sont réglés à la main comme un spectacle — une tache vit 8 à 16 s,
    /// une boucle monte en ~2 s. Les passer au même multiplicateur les rend
    /// illisibles : à ×16 une tache naît et meurt en une demi-seconde.
    ///
    /// Le test tient la séparation : l'animation reçoit `dt_anim`, jamais `dt`.
    #[test]
    fn l_animation_des_astres_ignore_le_multiplicateur_orbital() {
        let mut sys = Systeme::new();
        let (_, mouchard) = CorpsEssai::poser_mouchard(&mut sys, Categorie::Planete, 10.0, None);

        // Une frame de 1/60 s, jouée à ×16 : l'orbite avance de 16 fois plus,
        // l'animation d'une seule.
        let dt = 1.0 / 60.0;
        sys.update(dt * 16.0, DtAnimation(dt));
        assert!(
            (mouchard.get() - dt).abs() < 1e-6,
            "l'animation a reçu {} au lieu de {dt} : elle suit l'horloge orbitale",
            mouchard.get()
        );

        // Et la pause fige l'animation comme le reste.
        mouchard.set(0.0);
        sys.update(0.0, DtAnimation(0.0));
        assert_eq!(mouchard.get(), 0.0, "en pause, l'animation continue");
    }

    // Les chiffres romains, sur les cas qui cassent une implementation naive :
    // les soustractifs (IV, IX) et les passages de dizaine.
    #[test]
    fn les_chiffres_romains_sont_justes() {
        let attendus = [
            (1, "I"), (2, "II"), (3, "III"), (4, "IV"), (5, "V"), (6, "VI"),
            (8, "VIII"), (9, "IX"), (10, "X"), (11, "XI"), (14, "XIV"),
            (19, "XIX"), (20, "XX"), (39, "XXXIX"),
        ];
        for (n, s) in attendus {
            assert_eq!(romain(n), s, "romain({n})");
        }
        // Hors table : la decimale, plus lisible que XXXX repete.
        assert_eq!(romain(40), "40");
        assert_eq!(romain(0), "0");
    }

    // **Un nom propre l'emporte sur tout le reste.** C'est la regle de base, et
    // ce que les presets attendent.
    #[test]
    fn un_nom_propre_lemporte_sur_la_numerotation() {
        let mut sys = Systeme::new();
        let a = CorpsEssai::poser(&mut sys, Categorie::Planete, 100.0, None);
        let b = CorpsEssai::poser(&mut sys, Categorie::Planete, 200.0, None);
        assert_eq!(sys.designation(a), "I");
        sys.nommer(a, "Terre");
        assert_eq!(sys.designation(a), "Terre");
        // Le voisin, lui, garde son rang — nommer l'un ne renumerote pas l'autre.
        assert_eq!(sys.designation(b), "II");
    }

    // **Le rang se compte par distance, pas par ordre d'ajout.** C'est ce qui
    // distingue une designation d'un simple index. Les corps sont ici declares
    // dans le desordre exprès : le plus loin en premier.
    #[test]
    fn le_rang_suit_la_distance_et_non_lordre_dajout() {
        let mut sys = Systeme::new();
        let loin = CorpsEssai::poser(&mut sys, Categorie::Planete, 900.0, None);
        let pres = CorpsEssai::poser(&mut sys, Categorie::Planete, 100.0, None);
        let milieu = CorpsEssai::poser(&mut sys, Categorie::Planete, 400.0, None);
        assert_eq!(sys.designation(pres), "I", "la plus proche doit etre I");
        assert_eq!(sys.designation(milieu), "II");
        assert_eq!(sys.designation(loin), "III", "la plus lointaine doit etre III");
    }

    // **Une lune se designe par son parent** — « II-1 » — et non dans la suite
    // des planetes. Sans ca, la premiere lune du systeme s'appellerait « III »
    // et se confondrait avec la troisieme planete.
    #[test]
    fn une_lune_se_designe_par_son_parent() {
        let mut sys = Systeme::new();
        let p1 = CorpsEssai::poser(&mut sys, Categorie::Planete, 100.0, None);
        let p2 = CorpsEssai::poser(&mut sys, Categorie::Planete, 500.0, None);
        // Deux lunes autour de la seconde planete, la plus proche declaree en
        // second — le rang des lunes suit lui aussi la distance.
        let externe = CorpsEssai::poser(&mut sys, Categorie::Lune, 560.0, Some(p2));
        let interne = CorpsEssai::poser(&mut sys, Categorie::Lune, 520.0, Some(p2));
        assert_eq!(sys.designation(p1), "I");
        assert_eq!(sys.designation(p2), "II");
        assert_eq!(sys.designation(interne), "II-1", "la lune interne est la premiere");
        assert_eq!(sys.designation(externe), "II-2");
        // Et si le parent est nomme, ses lunes suivent.
        sys.nommer(p2, "Jupiter");
        assert_eq!(sys.designation(interne), "Jupiter-1");
    }

    // **Deux astres ne peuvent pas porter la meme designation** : le selecteur
    // montrerait deux lignes identiques, et cliquer l'une ou l'autre serait
    // indiscernable. Le cas piege est celui de deux corps **co-orbitaux**
    // (Phobos et Deimos partagent un rayon dans le preset solaire) : a distance
    // egale, il faut quand meme les departager.
    #[test]
    fn deux_astres_coorbitaux_ne_se_confondent_pas() {
        let mut sys = Systeme::new();
        let p = CorpsEssai::poser(&mut sys, Categorie::Planete, 300.0, None);
        let a = CorpsEssai::poser(&mut sys, Categorie::Lune, 340.0, Some(p));
        let b = CorpsEssai::poser(&mut sys, Categorie::Lune, 340.0, Some(p));
        assert_ne!(sys.designation(a), sys.designation(b), "deux lunes a la meme distance");
        assert_eq!(sys.designation(a), "I-1");
        assert_eq!(sys.designation(b), "I-2");
    }

    // **`lunes_de` rend les enfants directs, et seulement eux.** Le filtre
    // porte sur le PARENT, pas sur la catégorie : c'est ce qui le rend
    // générique (une station ou un vaisseau posté autour d'une planète en est
    // aussi l'enfant). Ce sont les appelants qui filtrent par catégorie.
    #[test]
    fn lunes_de_rend_les_enfants_directs_et_rien_d_autre() {
        let mut sys = Systeme::new();
        let etoile = CorpsEssai::poser(&mut sys, Categorie::Etoile, 0.0, None);
        let terre = CorpsEssai::poser(&mut sys, Categorie::Planete, 100.0, None);
        let mars = CorpsEssai::poser(&mut sys, Categorie::Planete, 150.0, None);
        let lune = CorpsEssai::poser(&mut sys, Categorie::Lune, 101.0, Some(terre));
        let phobos = CorpsEssai::poser(&mut sys, Categorie::Lune, 151.0, Some(mars));
        let deimos = CorpsEssai::poser(&mut sys, Categorie::Lune, 152.0, Some(mars));
        // Un engin posté autour de Mars en est aussi l'enfant direct.
        let station = CorpsEssai::poser(&mut sys, Categorie::Engin, 153.0, Some(mars));

        assert_eq!(sys.lunes_de(terre), vec![lune]);
        assert_eq!(sys.lunes_de(mars), vec![phobos, deimos, station]);
        assert!(sys.lunes_de(etoile).is_empty(), "une planète tient à son étoile par son foyer, pas par parent");
        assert!(sys.lunes_de(lune).is_empty(), "une lune sans satellite");
        assert!(sys.lunes_de(999).is_empty(), "index invalide : liste vide, pas de panique");
    }

    // Chaque categorie a un repli, et **aucune ne rend une chaine vide** : une
    // designation vide donnerait une ligne blanche dans le selecteur.
    #[test]
    fn aucune_categorie_ne_rend_une_designation_vide() {
        let mut sys = Systeme::new();
        for cat in [Categorie::Etoile, Categorie::Planete, Categorie::Lune, Categorie::Asteroide, Categorie::Comete, Categorie::Engin] {
            let i = CorpsEssai::poser(&mut sys, cat, 100.0, None);
            assert!(!sys.designation(i).is_empty(), "{cat:?} rend une designation vide");
        }
        // Index hors bornes : chaine vide assumee, mais surtout pas de panique.
        assert_eq!(sys.designation(9999), "");
    }

    // L'etoile unique ne se numerote pas — « ETOILE », pas « ETOILE I ». Des
    // qu'il y en a deux, elles le sont.
    #[test]
    fn letoile_ne_se_numerote_que_sil_y_en_a_plusieurs() {
        let mut sys = Systeme::new();
        let a = CorpsEssai::poser(&mut sys, Categorie::Etoile, 0.0, None);
        assert_eq!(sys.designation(a), "ETOILE");
        let b = CorpsEssai::poser(&mut sys, Categorie::Etoile, 50.0, None);
        assert_eq!(sys.designation(a), "ETOILE I");
        assert_eq!(sys.designation(b), "ETOILE II");
    }
}
