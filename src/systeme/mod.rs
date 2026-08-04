mod gravite;
mod rendu;

use crate::astre::{Astre, CameraInfo, Categorie, Foyer};
use crate::stellaire::ArbreStellaire;
use macroquad::prelude::*;

pub const G: f32 = 1.0; // constante gravitationnelle
const SOUS_PAS: usize = 4; // sous-pas de physique par frame (stabilité)

/// Mode de propagation des **planètes**.
/// - `SurRails` (défaut) : orbites de Kepler analytiques — stable, déterministe.
/// - `NCorps` : intégration gravitationnelle — dynamique émergente « bac à sable ».
/// Les étoiles et les lunes restent toujours analytiques, quel que soit le mode.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ModePhysique {
    SurRails,
    NCorps,
}

/// Le système : un ensemble d'astres soumis à la gravité mutuelle.
pub struct Systeme {
    astres: Vec<Box<dyn Astre>>,
    /// Lumière de secours (pos, couleur) utilisée quand il n'y a pas d'étoile
    /// (ex. vue d'une planète isolée). Ignorée dès qu'une étoile est présente.
    lumiere_manuelle: Option<(Vec3, Vec3)>,
    /// Temps de simulation cumulé (secondes) : sert à l'évaluation analytique `f(t)`.
    temps: f64,
    /// Mode de propagation des planètes (voir `ModePhysique`).
    mode: ModePhysique,
    /// Arbre stellaire hiérarchique (systèmes multiples). Absent = étoile unique fixe.
    arbre: Option<ArbreStellaire>,
    /// Vue par défaut suggérée : (astre à focaliser, distance caméra). Utilisé pour
    /// cadrer d'emblée la zone planétaire d'un système type S (trop étalé sinon).
    vue: Option<(usize, f32)>,
}

impl Systeme {
    pub fn new() -> Self {
        Self {
            astres: Vec::new(),
            lumiere_manuelle: None,
            temps: 0.0,
            mode: ModePhysique::SurRails,
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

    /// Mode de propagation courant.
    pub fn mode(&self) -> ModePhysique {
        self.mode
    }

    /// Change le mode. Le passage vers N-corps amorce les vitesses des planètes
    /// depuis leur orbite analytique (hand-off) ; le retour sur rails resnappe
    /// naturellement à la frame suivante. Idempotent (no-op si mode identique).
    pub fn regler_mode(&mut self, m: ModePhysique) {
        if m == self.mode {
            return;
        }
        if m == ModePhysique::NCorps {
            // Amorce chaque planète depuis son foyer (étoile hôte ou barycentre).
            let pos: Vec<Vec3> = self.astres.iter().map(|a| a.corps().position).collect();
            let t = self.temps;
            for a in &mut self.astres {
                let f = match a.foyer() {
                    Some(Foyer::Etoile(i)) => pos.get(i).copied().unwrap_or(Vec3::ZERO),
                    _ => Vec3::ZERO, // Barycentre / pas de foyer
                };
                a.amorcer_ncorps(f, Vec3::ZERO, t); // vitesse du foyer approximée à 0 (sandbox)
            }
        }
        self.mode = m;
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

    /// Teinte propre de l'astre `idx`, tirée de son apparence. `None` s'il n'en
    /// a pas (ceinture) ou si l'index est invalide.
    pub fn teinte_de(&self, idx: usize) -> Option<Vec3> {
        self.astres.get(idx).and_then(|a| a.teinte())
    }

    /// Rayon de l'astre `idx`, `None` si l'index est invalide.
    pub fn rayon_de(&self, idx: usize) -> Option<f32> {
        self.astres.get(idx).map(|a| a.corps().rayon)
    }

    /// **Luminosité cumulée** de toutes les étoiles du système.
    ///
    /// C'est la grandeur dont dépend la zone habitable, y compris circumbinaire
    /// — `systeme/rendu.rs` somme déjà de la même façon pour la tracer. Une
    /// seule source : la fiche d'astre et le tracé ne peuvent pas diverger.
    pub fn luminosite_totale(&self) -> f32 {
        self.astres.iter().filter_map(|a| a.luminosite()).sum()
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
            let rayon = a.corps().rayon.max(0.3) * 1.4; // marge pour cliquer facilement
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

    pub fn update(&mut self, dt: f32) {
        self.temps += dt as f64;
        let t = self.temps;

        // Étoiles : positions issues de l'arbre stellaire hiérarchique (barycentres
        // composés). TOUJOURS analytique, quel que soit le mode -> binaires/multiples
        // stables. Étoile unique = pas d'arbre -> reste à sa position (origine).
        if let Some(arbre) = self.arbre.take() {
            arbre.evaluer(t, &mut self.astres);
            self.arbre = Some(arbre);
        }

        match self.mode {
            // Planètes « sur rails » : chacune orbite son foyer (étoile hôte S-type
            // ou barycentre P-type). Étoiles déjà repositionnées juste au-dessus.
            ModePhysique::SurRails => {
                let pos: Vec<Vec3> = self.astres.iter().map(|a| a.corps().position).collect();
                for a in &mut self.astres {
                    if a.categorie() == Categorie::Planete {
                        let f = match a.foyer() {
                            Some(Foyer::Etoile(i)) => pos.get(i).copied().unwrap_or(Vec3::ZERO),
                            _ => Vec3::ZERO, // Barycentre
                        };
                        a.maj_rail(f, t);
                    }
                }
            }
            // Planètes en N-corps : intégration (elles ressentent les étoiles mobiles).
            ModePhysique::NCorps => {
                let h = dt / SOUS_PAS as f32;
                for _ in 0..SOUS_PAS {
                    self.gravite(h);
                }
            }
        }

        // Animation propre de chaque astre (éruptions du soleil, etc.).
        for a in &mut self.astres {
            a.update(dt);
        }

        // Lunes : orbite analytique autour de leur parent (positions courantes).
        let pos: Vec<Vec3> = self.astres.iter().map(|a| a.corps().position).collect();
        for a in &mut self.astres {
            if let Some(p) = a.parent() {
                a.orbiter_autour(pos[p], dt);
            }
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
    }

    impl CorpsEssai {
        fn poser(sys: &mut Systeme, cat: Categorie, x: f32, parent: Option<usize>) -> usize {
            sys.ajouter(Box::new(CorpsEssai {
                base: CorpsBase::new(vec3(x, 0.0, 0.0), 1.0, 1.0),
                cat,
                parent,
            }))
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
        fn update(&mut self, _dt: f32) {}
        fn draw(&mut self, _cam: &CameraInfo) {}
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
