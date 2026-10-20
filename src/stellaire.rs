//! Arbre stellaire hiérarchique : décrit un système à plusieurs étoiles comme un
//! arbre de **paires** (chaque paire = deux sous-arbres orbitant leur barycentre
//! commun). Couvre toutes les topologies stables :
//!
//! - binaire        `(A·B)`
//! - trinaire        `((A·B)·C)`            (paire serrée + compagnon lointain)
//! - quadruple 2+2   `((A·B)·(C·D))`        (deux paires)
//! - quadruple 3+1   `(((A·B)·C)·D)`        (triple + compagnon lointain)
//!
//! L'arbre est **déployé** en une structure plate évaluable en O(n) chaque frame :
//! on descend racine → feuilles, en composant les orbites (une paire serrée dont le
//! barycentre tourne lui-même autour d'un compagnon lointain = deux mouvements
//! superposés, obtenus naturellement par la récursion).

use crate::astre::Astre;
use crate::orbite::Orbite;
use macroquad::prelude::*;

/// Variante d'une étoile-feuille (couronne / type particulier). Extensible aux
/// autres types d'astres à mesure qu'on en ajoute.
#[derive(Clone, Copy)]
#[allow(dead_code)] // certaines variantes ne sont pas encore utilisées par un preset
pub enum Variante {
    Normale,
    Jets,    // étoile à neutrons / protoétoile (jets bipolaires)
    Vent,    // Wolf-Rayet / supergéante bleue (vent stellaire)
    Pulsar,  // jets tournants + flash
    Magnetar,
    /// Trou noir stellaire : horizon noir, anneau de photons, disque
    /// d'accrétion. Seule variante qui n'éclaire pas par sa surface — sa
    /// lumière vient du disque.
    TrouNoir,
    /// Jeune étoile (T Tauri) : jets bipolaires **et** disque protosolaire.
    /// Distincte de [`Variante::Jets`], que porte aussi l'étoile à neutrons —
    /// laquelle n'a rien à faire d'un disque de matière primordiale.
    Proto,
}

impl Variante {
    /// **Le mode de couronne dit la variante. Source unique.**
    ///
    /// ⚠️ Un mode sans bras retombe sur `Normale` **en silence** : l'étoile
    /// perd ses jets, son horizon ou son disque, son rendu existe toujours,
    /// et rien ne le dit. C'est ce qui a tenu le trou noir hors du jeu.
    pub fn de_mode(mode: f32) -> Self {
        match mode as i32 {
            1 => Variante::Jets,
            2 => Variante::Vent,
            3 => Variante::Pulsar,
            4 => Variante::Magnetar,
            5 => Variante::TrouNoir,
            6 => Variante::Proto,
            _ => Variante::Normale,
        }
    }

    /// Nom lisible de la variante, pour l'affichage de debug. ⚠️ Ici, et pas
    /// dans la vue qui l'affiche : un `match` sur la variante recopié
    /// ailleurs est exactement ce que [`Self::appliquer`] existe pour
    /// éviter.
    pub fn nom(self) -> &'static str {
        match self {
            Variante::Normale => "normale",
            Variante::Jets => "jets",
            Variante::Vent => "vent",
            Variante::Pulsar => "pulsar",
            Variante::Magnetar => "magnetar",
            Variante::TrouNoir => "trou noir",
            Variante::Proto => "jets + disque proto",
        }
    }

    /// **Configure une étoile selon sa variante. Source unique aussi.**
    ///
    /// ⚠️ La galerie en tenait une **copie**, sous forme de chaîne de `if`
    /// sur le mode — et les deux avaient déjà divergé : son `mode > 4.5`
    /// attrapait aussi le mode 6, si bien qu'une jeune étoile y aurait été
    /// construite en trou noir, pendant que le T Tauri du catalogue restait
    /// au mode 1 et n'avait donc aucun disque. Deux endroits qui décident du
    /// même fait finissent toujours par ne plus dire la même chose.
    /// ⚠️ **Prend aussi le nom du type**, depuis le lot 7. La pulsation n'est
    /// pas un mode de couronne — une céphéide est au mode 0, comme une G
    /// ordinaire — donc elle n'aurait atteint aucune étoile par ce chemin, et il
    /// aurait fallu l'écrire dans les **cinq** endroits qui configurent une
    /// étoile d'après son type. C'est le défaut que cette fonction existe pour
    /// éviter, et la conception l'avait annoncé : « `Variante` ignore trou noir
    /// comme pulsation ; un exotique ajouté sans elle n'apparaîtra jamais dans
    /// un système multiple ».
    ///
    /// Un nom vide — les presets écrits à la main, qui décrivent de vraies
    /// étoiles — ne pulse pas.
    pub fn appliquer(self, s: crate::soleil::Soleil, nom: &str) -> crate::soleil::Soleil {
        let (amp, periode) = crate::etoile::pulsation(nom);
        let s = s
            .avec_pulsation(amp, periode)
            .avec_ebullition_du_type(nom)
            .avec_surface_du_type(nom)
            .avec_eclat_visible(crate::etoile::eclat_visible(nom));
        match self {
            Variante::Normale => s,
            Variante::Jets => s.avec_jets(),
            Variante::Vent => s.avec_vent(),
            Variante::Pulsar => s.avec_pulsar(),
            Variante::Magnetar => s.avec_magnetar(),
            Variante::TrouNoir => s.avec_trou_noir(),
            Variante::Proto => s.avec_jets().avec_disque_protosolaire(),
        }
    }
}

/// Étoile-feuille de l'arbre. La **masse** sert au calcul des barycentres.
#[derive(Clone, Copy)]
pub struct Feuille {
    pub rayon: f32,
    pub couleur: Vec3,
    pub luminosite: f32,
    pub masse: f32,
    pub variante: Variante,
    /// Rémanent compact (pulsar/magnétar/étoile à neutrons) : pas de zone habitable.
    /// Distinct de `variante` car l'étoile à neutrons partage `Jets` avec T Tauri.
    pub remnant: bool,
    /// **Nom du type**, pour ce qui s'en dérive et que le mode de couronne ne
    /// dit pas — la pulsation, aujourd'hui.
    ///
    /// ⚠️ Vide par défaut, et c'est voulu : les presets écrits à la main
    /// décrivent de **vraies** étoiles (le Soleil, Alpha du Centaure) et n'ont
    /// aucune raison de pulser. Seul `feuille_alea`, qui tire dans le catalogue,
    /// le renseigne.
    pub nom: &'static str,
}

impl Feuille {
    pub fn new(rayon: f32, couleur: Vec3, luminosite: f32, masse: f32) -> Self {
        Self {
            rayon,
            couleur,
            luminosite,
            masse,
            variante: Variante::Normale,
            remnant: false,
            nom: "",
        }
    }

    /// Renseigne le **type** de la feuille : ce qui s'en dérive au-delà du mode
    /// de couronne (la pulsation) s'appliquera alors comme en système simple.
    pub fn du_type(mut self, nom: &'static str) -> Self {
        self.nom = nom;
        self
    }
    /// Fixe une variante (jets, vent, pulsar…).
    pub fn variante(mut self, v: Variante) -> Self {
        self.variante = v;
        self
    }
    /// Marque (ou non) la feuille comme rémanent compact sans zone habitable.
    pub fn avec_remnant(mut self, remnant: bool) -> Self {
        self.remnant = remnant;
        self
    }
}

/// Nœud de l'arbre : une étoile, ou une paire de sous-arbres.
pub enum Noeud {
    Etoile(Feuille),
    Paire {
        a: Box<Noeud>,
        b: Box<Noeud>,
        sep: f32,   // demi-grand axe RELATIF a↔b (en UA)
        e: f32,     // excentricité
        incl: f32,  // inclinaison du plan orbital (rad)
        phase: f32, // anomalie moyenne de départ (rad)
    },
}

impl Noeud {
    pub fn etoile(f: Feuille) -> Noeud {
        Noeud::Etoile(f)
    }
    pub fn paire(a: Noeud, b: Noeud, sep: f32, e: f32, incl: f32, phase: f32) -> Noeud {
        Noeud::Paire { a: Box::new(a), b: Box::new(b), sep, e, incl, phase }
    }
    /// Masse totale du sous-arbre (somme des feuilles).
    pub fn masse(&self) -> f32 {
        match self {
            Noeud::Etoile(f) => f.masse,
            Noeud::Paire { a, b, .. } => a.masse() + b.masse(),
        }
    }
}

/// Nœud déployé : orbite autour du barycentre parent + éventuelle étoile associée.
struct NoeudDeploye {
    parent: Option<usize>,  // index dans `noeuds` (garanti < index courant)
    orbite: Option<Orbite>, // None = racine (barycentre à l'origine)
    astre: Option<usize>,   // index de l'étoile dans `Systeme.astres` (feuille)
}

/// Arbre déployé : liste plate en ordre topologique (parent avant enfants).
pub struct ArbreStellaire {
    noeuds: Vec<NoeudDeploye>,
}

impl ArbreStellaire {
    pub fn new() -> Self {
        Self { noeuds: Vec::new() }
    }

    /// Ajoute un nœud déployé, renvoie son index. `parent` doit déjà exister.
    pub fn ajouter(&mut self, parent: Option<usize>, orbite: Option<Orbite>, astre: Option<usize>) -> usize {
        self.noeuds.push(NoeudDeploye { parent, orbite, astre });
        self.noeuds.len() - 1
    }

    /// Évalue les positions à l'instant `t` (racine → feuilles) et les écrit dans
    /// les étoiles correspondantes. O(n).
    pub fn evaluer(&self, t: f64, astres: &mut [Box<dyn Astre>]) {
        let mut pos = vec![Vec3::ZERO; self.noeuds.len()];
        for (i, nd) in self.noeuds.iter().enumerate() {
            let base = match nd.parent {
                Some(p) => pos[p], // déjà calculé (ordre topologique)
                None => Vec3::ZERO,
            };
            let local = match &nd.orbite {
                Some(o) => o.position(t),
                None => Vec3::ZERO,
            };
            pos[i] = base + local;
            if let Some(ai) = nd.astre {
                if let Some(a) = astres.get_mut(ai) {
                    a.corps_mut().position = pos[i];
                }
            }
        }
    }

    /// Polylignes (en coordonnées monde) des orbites de chaque étoile-feuille autour
    /// de son barycentre parent, à l'instant `t`. Pour tracer les orbites stellaires.
    pub fn orbites_etoiles(&self, t: f64) -> Vec<Vec<Vec3>> {
        let mut pos = vec![Vec3::ZERO; self.noeuds.len()];
        let mut sorties = Vec::new();
        for (i, nd) in self.noeuds.iter().enumerate() {
            let base = match nd.parent {
                Some(p) => pos[p],
                None => Vec3::ZERO,
            };
            let local = match &nd.orbite {
                Some(o) => o.position(t),
                None => Vec3::ZERO,
            };
            pos[i] = base + local;
            // Étoile (feuille) avec une orbite -> ellipse centrée sur le barycentre parent.
            if nd.astre.is_some() {
                if let (Some(p), Some(o)) = (nd.parent, nd.orbite.as_ref()) {
                    let centre = pos[p];
                    sorties.push(o.polyligne(96).into_iter().map(|pt| centre + pt).collect());
                }
            }
        }
        sorties
    }
}

#[cfg(test)]
mod tests_variante {
    /// Les fichiers qui traduisent un **mode de couronne** en étoile
    /// configurée. Tous doivent passer par [`super::Variante::appliquer`].
    const CONSOMMATEURS: [(&str, &str); 3] = [
        ("genese/mod.rs", include_str!("genese/mod.rs")),
        ("ecran/galerie_etoiles.rs", include_str!("ecran/galerie_etoiles.rs")),
        ("ecran/objet.rs", include_str!("ecran/objet.rs")),
    ];

    /// **Personne ne retraduit un mode de couronne dans son coin.**
    ///
    /// ⚠️ Ce défaut s'est produit **trois fois** sur le chantier astres, et
    /// chaque fois il est passé inaperçu jusqu'à ce qu'on le voie à l'écran :
    /// la genèse, la galerie des étoiles et la vue isolée tenaient chacune
    /// leur propre chaîne `mode → constructeur`, arrêtée aux bras qui
    /// existaient le jour où elle a été écrite. Un mode plus récent y
    /// retombait sur `Normale` **en silence** — le trou noir s'affichait en
    /// étoile ordinaire, la jeune étoile perdait son disque.
    ///
    /// La garde est indirecte, faute de mieux : construire un `Soleil` exige
    /// le contexte graphique, donc rien ne peut vérifier le résultat. Mais
    /// une chaîne recopiée mentionne forcément les variantes du **milieu**
    /// (`pulsar`, `magnetar`) — celles que personne ne construit
    /// délibérément, à la différence du trou noir ou des jets, qu'une
    /// galerie peut légitimement demander nommément.
    #[test]
    fn personne_ne_retraduit_un_mode_dans_son_coin() {
        for (nom, src) in CONSOMMATEURS {
            for builder in ["avec_pulsar(", "avec_magnetar(", "avec_pulsation("] {
                let recopie = src
                    .lines()
                    .filter(|l| !l.trim_start().starts_with("//"))
                    .any(|l| l.contains(builder));
                assert!(
                    !recopie,
                    "{nom} construit `{builder}` lui-même : c'est une chaîne \
                     mode → constructeur recopiée. Passer par `Variante::appliquer`."
                );
            }
        }
    }

    /// **Et la source unique, elle, les construit bien toutes.** Sans quoi
    /// le test ci-dessus serait satisfait par un `appliquer` qui aurait
    /// perdu des bras en route.
    ///
    /// ⚠️ Ne cherche que dans la partie **production** du fichier. La liste
    /// ci-dessous contient elle-même les noms qu'elle cherche : fouiller tout
    /// le fichier rendait le test **toujours vrai**, quoi qu'on retire de
    /// `appliquer`. Il se satisfaisait lui-même, et le sabotage l'a montré.
    #[test]
    fn la_source_unique_construit_toutes_les_variantes() {
        let ici = include_str!("stellaire.rs");
        let ici = ici.split("#[cfg(test)]").next().unwrap_or(ici);
        for builder in [
            "avec_jets(",
            "avec_vent(",
            "avec_pulsar(",
            "avec_magnetar(",
            "avec_trou_noir(",
            "avec_disque_protosolaire(",
            "avec_pulsation(",
            "avec_ebullition_du_type(",
            "avec_eclat_visible(",
            "avec_surface_du_type(",
        ] {
            assert!(ici.contains(builder), "`Variante::appliquer` ne sait pas faire `{builder}`");
        }
    }
}
