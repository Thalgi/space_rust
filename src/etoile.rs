use macroquad::prelude::*;
use macroquad::rand::gen_range;

/// Profil physique d'une étoile.
pub struct ProfilEtoile {
    pub temperature: f32, // Kelvin
    pub couleur: Vec3,    // couleur de corps noir
    pub rayon: f32,       // rayon visuel (unités du jeu)
    pub luminosite: f32,  // relative (éclaire les planètes)
    pub nom: &'static str,
    pub couronne: f32,    // mode de couronne : 0 halo, 1 jets, 2 vent, 3 pulsar, 4 magnétar
}

/// Un gabarit d'étoile : `(t_min, t_max, rayon, luminosité, nom, couronne)`.
pub(crate) type Gabarit = (f32, f32, f32, f32, &'static str, f32);

/// Part du tirage réservée aux étoiles **particulières**.
const PART_PARTICULIERES: f32 = 0.05;

/// **Quelle étoile particulière**, pour un tirage `p` dans `[0 ; 1]`.
///
/// Extrait de [`ProfilEtoile::aleatoire`] pour être **testable** :
/// `macroquad::rand::gen_range` exige le contexte graphique, donc le tirage
/// lui-même ne se teste pas — mais ce qu'on en fait, si. C'est le seul
/// endroit du dépôt où se décide quels objets exotiques existent.
pub(crate) fn particuliere(p: f32) -> Gabarit {
    // ⚠️ Le trou noir tient **un cinquième** des particulières, soit 1 % de
    // tous les tirages. C'est bien plus que sa part réelle parmi les objets
    // stellaires — mais le but est qu'on puisse en **rencontrer un en
    // explorant**, pas de simuler un recensement. Même esprit que les 5 %
    // de particulières, déjà très au-dessus du réel.
    if p < 0.2 {
        (5500.0, 7500.0, 0.62, 0.4, "Trou noir", 5.0)
    } else if p < 0.36 {
        (35000.0, 45000.0, 0.42, 0.25, "Pulsar", 3.0)
    } else if p < 0.52 {
        (35000.0, 45000.0, 0.42, 0.2, "Etoile a neutrons", 1.0)
    } else if p < 0.64 {
        (35000.0, 45000.0, 0.45, 0.3, "Magnetar", 4.0)
    } else if p < 0.8 {
        (38000.0, 45000.0, 1.1, 5.0, "Wolf-Rayet", 2.0)
    } else if p < 0.92 {
        (18000.0, 24000.0, 1.4, 6.0, "Supergeante bleue", 2.0)
    } else {
        (4000.0, 4600.0, 0.9, 1.2, "T Tauri (jeune)", 6.0)
    }
}

/// **Quelle étoile ordinaire** — tout ce qui n'est pas une particulière — pour
/// un tirage `p` dans `[0 ; 1]`, `q` donnant la place dans la plage de rayon et
/// de luminosité de son type.
///
/// ⚠️ **Extrait d'`aleatoire` pour la même raison que [`particuliere`]** :
/// `gen_range` exige le contexte graphique, donc le tirage ne se teste pas,
/// mais la **table**, si. Les trois types du lot 6 auraient vécu dans la chaîne
/// de `if` d'`aleatoire`, hors d'atteinte de tout test — c'est exactement
/// ainsi que le trou noir est resté hors du jeu jusqu'au lot 4.
///
/// `q` est ce qui évite un second type de piège : une table de constantes
/// figées ferait de toutes les géantes rouges des jumelles, et le lot 1 a été
/// fait pour le contraire.
pub(crate) fn ordinaire(p: f32, q: f32) -> Gabarit {
    let entre = |a: f32, b: f32| a + (b - a) * q.clamp(0.0, 1.0);
    if p < 0.08 {
        // Géante rouge : énorme, froide, très lumineuse.
        (3000.0, 3900.0, entre(4.5, 7.0), entre(3.0, 6.0), "Geante rouge", 0.0)
    } else if p < 0.15 {
        // Naine blanche : petite, très chaude, peu lumineuse.
        (12000.0, 28000.0, entre(0.5, 0.8), entre(0.15, 0.4), "Naine blanche", 0.0)
    } else if p < 0.18 {
        // **Supergéante rouge** (lot 6). Se distingue de la géante par la
        // taille et l'éclat, pas par la teinte : 150 K d'écart ne se voient
        // pas. ⚠️ C'est le plus gros astre que le tirage sache produire, et
        // `genese::premiere_orbite` en dépend pour ne pas poser un monde
        // dedans.
        (3200.0, 4000.0, entre(8.5, 12.0), entre(8.0, 14.0), "Supergeante rouge", 0.0)
    } else if p < 0.21 {
        // **Sous-naine chaude sdB** (lot 6). ⚠️ Sa couleur est celle d'une
        // naine blanche — mesuré : (0,62 / 0,74 / 1,00) contre
        // (0,67 / 0,78 / 1,00). Ce qui la distingue est son **éclat**, cinq à
        // vingt fois supérieur : cœur d'hélium en fusion contre braise qui
        // refroidit. Ça se lit au halo, et la zone habitable s'en va au loin.
        (28000.0, 36000.0, entre(0.55, 0.75), entre(1.6, 3.2), "Sous-naine (sdB)", 0.0)
    } else if p < 0.24 {
        // **Étoile carbonée** (lot 6) : une géante froide dont la teinte ne
        // vient pas du corps noir mais de [`filtre_teinte`]. Sans ce filtre
        // elle n'est pas un type, seulement une étiquette.
        (2600.0, 3200.0, entre(4.0, 6.5), entre(2.0, 4.0), "Etoile carbonee (C)", 0.0)
    } else if p < 0.27 {
        // **Naine brune** (lot 6.2). ⚠️ Le seul type du tirage qui n'est **pas
        // une photosphère** : elle ne fusionne pas l'hydrogène, et le dépôt la
        // rend depuis toujours en **géante gazeuse** — bandes, vortex, émission
        // thermique — ce qui est bien meilleur que ce qu'une étoile en ferait.
        // Voir [`Self::est_naine_brune`], que chaque bâtisseur de corps doit
        // consulter.
        //
        // Rayon : celui d'une géante gazeuse (~11 R⊕ compressés), soit ~2
        // unités.
        //
        // ⚠️ Luminosité **volontairement non physique**. Le réel est de l'ordre
        // de 10⁻⁵ L☉ : ses mondes seraient noirs et sa fiche annoncerait 0 K.
        // Posée au niveau de la naine blanche (0,15–0,4), elle reste l'astre le
        // plus terne du catalogue tout en éclairant ce qui l'entoure. L'échelle
        // de luminosité du jeu est de toute façon compressée — une O bleue y
        // vaut 4 pour 10⁵ dans le réel.
        //
        // La température descend à 450 K pour que la classe **Y** soit
        // atteignable (`genese::preset_naine_brune`).
        (450.0, 2400.0, entre(1.7, 2.1), entre(0.15, 0.30), "Naine brune", 0.0)
    } else if p < 0.29 {
        // **Céphéide** (lot 7) : supergéante jaune. Se distingue d'une
        // supergéante rouge par sa température, et de tout le reste par sa
        // **pulsation** — voir [`pulsation`].
        (5000.0, 6500.0, entre(5.0, 7.5), entre(6.0, 11.0), "Cepheide", 0.0)
    } else if p < 0.31 {
        // **Mira** (lot 7) : géante rouge de la branche asymptotique. ⚠️ Elle
        // recouvre la géante rouge en teinte comme en taille, et c'est
        // **assumé** : ce qui en fait un type est sa pulsation, la plus ample
        // du catalogue. Le §3 bis l'autorise — une représentation distincte à
        // l'écran, pas forcément une teinte distincte.
        (2500.0, 3200.0, entre(6.0, 9.0), entre(4.0, 8.0), "Mira", 0.0)
    } else if p < 0.33 {
        // **Naine blanche pulsante (ZZ Ceti)** (lot 7). ⚠️ Gabarit
        // **identique** à celui d'une naine blanche : même température, même
        // taille, même éclat. Sa pulsation est sa seule différence, et c'est
        // exactement ce que la fiche du §3 annonçait.
        (11000.0, 12500.0, entre(0.5, 0.8), entre(0.15, 0.4), "Naine blanche pulsante (ZZ Ceti)", 0.0)
    } else {
        // Séquence principale (poids biaisés pour la variété).
        let r = (p - 0.33) / 0.67;
        if r < 0.36 {
            (2400.0, 3700.0, 1.3, 0.4, "M (naine rouge)", 0.0)
        } else if r < 0.58 {
            (3700.0, 5200.0, 1.6, 0.7, "K (orange)", 0.0)
        } else if r < 0.74 {
            (5200.0, 6000.0, 2.0, 1.0, "G (jaune)", 0.0)
        } else if r < 0.85 {
            (6000.0, 7500.0, 2.3, 1.4, "F (blanc-jaune)", 0.0)
        } else if r < 0.93 {
            (7500.0, 11000.0, 2.6, 2.0, "A (blanche)", 0.0)
        } else if r < 0.98 {
            (11000.0, 30000.0, 3.2, 3.0, "B (bleu-blanc)", 0.0)
        } else {
            (30000.0, 45000.0, 4.0, 4.0, "O (bleue)", 0.0)
        }
    }
}

/// **Pas de balayage des tables** : 1/1000. La bande la plus étroite (O bleue,
/// 1,3 % du tirage) y tient treize échantillons.
const PAS_BALAYAGE: usize = 1000;

/// **Les types que le tirage sait produire**, dans l'ordre des tables :
/// particulières d'abord, puis ordinaires.
///
/// ⚠️ **Balayés, pas recopiés.** Une liste écrite à la main oublierait le
/// prochain type ajouté à [`ordinaire`] ou à [`particuliere`] — c'est ainsi que
/// le trou noir est resté hors du jeu jusqu'au lot 4.
pub fn types_du_tirage() -> Vec<&'static str> {
    let mut noms: Vec<&'static str> = Vec::new();
    let pas = |k: usize| k as f32 / PAS_BALAYAGE as f32;
    let tables = (0..PAS_BALAYAGE)
        .map(|k| particuliere(pas(k)))
        .chain((0..PAS_BALAYAGE).map(|k| ordinaire(pas(k), 0.5)));
    for g in tables {
        if !noms.contains(&g.4) {
            noms.push(g.4);
        }
    }
    noms
}

/// **Gabarit d'un type nommé**, `q` donnant sa place dans la plage de rayon et
/// de luminosité (voir [`ordinaire`]). `None` si le tirage ne le connaît pas.
pub(crate) fn gabarit_du_type(nom: &str, q: f32) -> Option<Gabarit> {
    (0..PAS_BALAYAGE)
        .map(|k| k as f32 / PAS_BALAYAGE as f32)
        .flat_map(|p| [particuliere(p), ordinaire(p, q)])
        .find(|g| g.4 == nom)
}

impl ProfilEtoile {
    /// Tire une étoile au hasard : une particulière dans 5 % des cas, sinon une
    /// ordinaire ([`ordinaire`], où vit toute la table).
    pub fn aleatoire() -> Self {
        let v: f32 = gen_range(0.0, 1.0);
        let g = if v < PART_PARTICULIERES {
            particuliere(gen_range(0.0, 1.0))
        } else {
            ordinaire(gen_range(0.0, 1.0), gen_range(0.0, 1.0))
        };
        Self::depuis_gabarit(g)
    }

    /// **Une étoile d'un type choisi** — le menu « choisir astre » de la vue
    /// système, outil de mise au point. Seuls la température et la place dans
    /// la plage sont tirées. `None` si le tirage ne connaît pas ce type.
    pub fn du_type(nom: &str) -> Option<Self> {
        gabarit_du_type(nom, gen_range(0.0, 1.0)).map(Self::depuis_gabarit)
    }

    fn depuis_gabarit((tmin, tmax, rayon, lumi, nom, couronne): Gabarit) -> Self {
        let temperature = gen_range(tmin, tmax);
        Self {
            temperature,
            couleur: couleur_etoile(nom, temperature),
            rayon,
            luminosite: lumi,
            nom,
            couronne,
        }
    }

    pub fn nom(&self) -> &'static str {
        self.nom
    }

    /// Rémanent stellaire compact (pulsar, magnétar, étoile à neutrons). Ces objets
    /// n'ont **pas de zone habitable** pertinente (leur éclat visible ne trace pas
    /// une zone d'eau liquide) et leurs planètes rescapées orbitent serré
    /// (cf. PSR B1257+12 : 0,19–0,46 UA). Sert à adapter distances et affichage.
    pub fn est_remnant(&self) -> bool {
        matches!(self.nom, "Pulsar" | "Etoile a neutrons" | "Magnetar" | "Trou noir")
    }

    /// **Trou noir.** Il se distingue des autres rémanents sur deux points :
    /// aucune surface ne s'y allume (le moteur d'éruptions s'arrête net, voir
    /// `eruptions::maj`), et sa **masse ne se déduit pas de sa luminosité**
    /// (`genese::MASSE_TROU_NOIR`).
    pub fn est_trou_noir(&self) -> bool {
        self.nom == "Trou noir"
    }

    /// **Naine brune** : le seul type du tirage qui ne produit pas un `Soleil`.
    ///
    /// Elle ne fusionne pas l'hydrogène, n'a pas de photosphère stellaire, et le
    /// dépôt la rend depuis toujours en **géante gazeuse** — bandes, vortex,
    /// émission thermique côté nuit. La rendre en étoile serait un recul.
    ///
    /// ⚠️ **Tout bâtisseur de corps doit consulter ce prédicat.** Trois le
    /// font : la genèse d'un système simple (elle en fait le centre), le tirage
    /// des feuilles d'un système multiple (il l'écarte, faute de savoir en faire
    /// une compagne), et la vue isolée (elle en fait une planète). Un quatrième
    /// qui l'oublierait la peindrait en étoile, et ce serait invisible au test :
    /// c'est le défaut le plus tenace de ce chantier.
    ///
    /// Elle partage deux traits avec les rémanents — orbites serrées, pas de
    /// zone habitable traçable — sans en être une : `est_remnant` reste faux.
    pub fn est_naine_brune(&self) -> bool {
        est_naine_brune(self.nom)
    }
}

/// Unités du monde par UA (unité astronomique). Les distances orbitales sont
/// raisonnées en UA puis converties à l'échelle de rendu via cette constante.
pub const UA: f32 = 48.0;

/// Température d'équilibre approximative d'une planète (Kelvin), d'après la
/// luminosité de l'étoile et la distance **en UA** : T = 278 · L^¼ / √(d_UA).
/// Calée sur le réel : L=1, d=1 UA → 278 K (la Terre).
pub fn temp_equilibre(luminosite: f32, distance_ua: f32) -> f32 {
    278.0 * luminosite.powf(0.25) / distance_ua.max(0.01).sqrt()
}

/// Bornes (interne, externe) de la zone habitable **en UA** : eau liquide
/// (~320 K → ~260 K). Se déplace avec la luminosité de l'étoile.
pub fn zone_habitable(luminosite: f32) -> (f32, f32) {
    let k = 278.0 * luminosite.powf(0.25);
    let interne = (k / 320.0).powi(2);
    let externe = (k / 260.0).powi(2);
    (interne, externe)
}

/// **Taux d'activité éruptive propre au type d'étoile**, dérivé de son gabarit
/// (rayon, luminosité) — les deux seuls traits que connaît déjà `Soleil::new`,
/// pas besoin d'un troisième canal pour ce qui n'est qu'un facteur d'échelle.
///
/// Une naine M (petite, peu lumineuse) est active ; une géante (grande, très
/// lumineuse) l'est peu. `gabarit = rayon × √luminosité` croît avec les deux,
/// et l'activité en décroît — borné à `[0,15 ; 1,0]` : jamais nulle (une
/// géante flambe rarement, pas jamais), jamais démesurée.
pub fn activite_type(rayon: f32, luminosite: f32) -> f32 {
    let gabarit = rayon.max(0.01) * luminosite.max(0.0).sqrt();
    (1.6 / (1.0 + gabarit)).clamp(0.15, 1.0)
}

/// **Pulsation d'un type** : `(amplitude relative du rayon, période en
/// secondes)`. `(0, 0)` pour tout ce qui ne pulse pas — c'est-à-dire presque
/// tout.
///
/// Une seule pièce couvre les **trois** types variables, parce qu'ils ne
/// diffèrent que par ces deux nombres. La luminosité n'a pas de réglage à
/// elle : à température constante `L ∝ R²`, donc elle suit le carré du facteur.
/// C'est la physique qui lie les deux, pas une commodité — et ça évite deux
/// réglages pour un seul fait.
///
/// ⚠️ **Réponse à la question laissée ouverte au §9** de la conception
/// (« jusqu'où va la pulsation ? »). Les **amplitudes de rayon sont réelles** :
/// δ Cephei varie d'environ 10 %, Mira d'environ 20 %. Seule ZZ Ceti est
/// exagérée — son rayon ne bouge en vrai que de quelques millièmes, et sa
/// variation de 0,1 à 3 % de brillance serait invisible ici ; c'est de la
/// triche assumée, du même ordre que `PX_MIN`.
///
/// Les **périodes**, elles, sont toutes compressées et ne peuvent pas ne pas
/// l'être : 5,4 jours pour δ Cephei, 332 jours pour Mira, quelques minutes pour
/// ZZ Ceti. Elles sont calées sur l'horloge du spectacle, celle où une tache vit
/// huit à seize secondes. Le garde-fou est qu'aucune ne descende au clignotement.
pub fn pulsation(nom: &str) -> (f32, f32) {
    match nom {
        "Cepheide" => (0.10, 7.0),
        // Allégée de 20 % (réel) à 15 % le 2026-10-19 : elle bouillonne aussi
        // désormais (`ebullition`), et les deux mouvements s'additionnent.
        "Mira" => (0.15, 16.0),
        "Naine blanche pulsante (ZZ Ceti)" => (0.04, 1.4),
        _ => (0.0, 0.0),
    }
}

/// **Ébullition de surface d'un type** : `(échelle, style)`, ce que reçoit
/// `Soleil::avec_ebullition`. `(0, plat)` : la granulation fine ordinaire.
///
/// Jugé à l'écran le 2026-10-19 : la géante et la supergéante rouges ont un
/// aspect **stable** (grosses cellules peintes à plat) et un aspect
/// **instable** — le contour qui se déforme pour la géante, le relief pour la
/// supergéante. Qui est instable se dérive de la graine (`soleil::est_instable`).
/// Mira l'est toujours : géante de la branche asymptotique, elle pulse en plus.
pub fn ebullition(nom: &str, instable: bool) -> (f32, f32) {
    use crate::soleil::{EBUL_CONTOUR, EBUL_PLAT, EBUL_RELIEF};
    match (nom, instable) {
        ("Geante rouge", false) => (0.5, EBUL_PLAT),
        ("Geante rouge", true) | ("Mira", _) => (1.0, EBUL_CONTOUR),
        ("Supergeante rouge", false) => (1.0, EBUL_PLAT),
        ("Supergeante rouge", true) => (1.0, EBUL_RELIEF),
        _ => (0.0, EBUL_PLAT),
    }
}

/// **Naine brune ?**, d'après le seul nom du type.
///
/// ⚠️ La forme dont a besoin un bâtisseur qui n'a qu'un **gabarit** sous la
/// main, sans `ProfilEtoile` complet — la vue isolée, par exemple.
/// [`ProfilEtoile::est_naine_brune`] passe par ici : le nom du type n'est écrit
/// qu'une fois.
pub fn est_naine_brune(nom: &str) -> bool {
    nom == "Naine brune"
}

/// **Filtre d'absorption moléculaire**, en multiplicateur de la couleur de
/// corps noir. Neutre pour presque tous les types.
///
/// ⚠️ **Sans lui, l'étoile carbonée n'existe pas.** Sa fiche de conception la
/// donne « distincte par la seule teinte » (§3) — mais dérivée du corps noir
/// comme les autres, sa teinte la rend **moins rouge** que les naines M les
/// plus froides : mesuré, (1,00 / 0,68 / 0,40) à 2900 K contre
/// (1,00 / 0,61 / 0,24) pour une naine M à 2400 K. Ce serait donc une
/// étiquette et non un type, ce que le filtre de la §3 bis interdit
/// explicitement.
///
/// **Et la physique est justement un filtre** : une étoile carbonée est une
/// photosphère froide vue **à travers** ses propres bandes de C₂ et CN, qui
/// mangent le bleu puis le vert. D'où un multiplicateur plutôt qu'une couleur
/// posée à la main — et l'ordre R > G > B s'en trouve renforcé, jamais inversé.
pub fn filtre_teinte(nom: &str) -> Vec3 {
    match nom {
        "Etoile carbonee (C)" => vec3(1.0, 0.55, 0.20),
        _ => Vec3::ONE,
    }
}

/// **La couleur d'une étoile, d'après son type et sa température. Source
/// unique.**
///
/// ⚠️ Trois endroits la calculaient — le tirage, la galerie des étoiles, la vue
/// isolée — tous les trois par `couleur_corps_noir` seul. Le filtre posé dans
/// un seul aurait donné une carbonée rouge en jeu et orange en vitrine.
///
/// La carte du ciel (`starmap`) reste sur `couleur_corps_noir` et c'est
/// volontaire : elle ne connaît que des températures, pas des types.
pub fn couleur_etoile(nom: &str, temp: f32) -> Vec3 {
    couleur_corps_noir(temp) * filtre_teinte(nom)
}

/// Couleur approximative d'un corps noir à `temp` Kelvin (approx. de Tanner Helland).
pub fn couleur_corps_noir(temp: f32) -> Vec3 {
    let t = temp.clamp(1000.0, 40000.0) / 100.0;

    let r = if t <= 66.0 {
        255.0
    } else {
        (329.698_73 * (t - 60.0).powf(-0.133_204_76)).clamp(0.0, 255.0)
    };
    let g = if t <= 66.0 {
        (99.470_8 * t.ln() - 161.119_57).clamp(0.0, 255.0)
    } else {
        (288.122_2 * (t - 60.0).powf(-0.075_514_85)).clamp(0.0, 255.0)
    };
    let b = if t >= 66.0 {
        255.0
    } else if t <= 19.0 {
        0.0
    } else {
        (138.517_73 * (t - 10.0).ln() - 305.044_8).clamp(0.0, 255.0)
    };

    vec3(r / 255.0, g / 255.0, b / 255.0)
}

#[cfg(test)]
mod tests_particulieres {
    use super::*;

    /// Les noms que le tirage des particulières doit savoir produire.
    const ATTENDUS: [&str; 7] = [
        "Trou noir",
        "Pulsar",
        "Etoile a neutrons",
        "Magnetar",
        "Wolf-Rayet",
        "Supergeante bleue",
        "T Tauri (jeune)",
    ];

    fn balayage() -> Vec<&'static str> {
        (0..1000).map(|k| particuliere(k as f32 / 1000.0).4).collect()
    }

    /// **Chaque type particulier est atteignable.** Une borne mal posée en
    /// écarte un **en silence** : son rendu continue d'exister, personne ne le
    /// rencontre jamais. C'est exactement ce qui est arrivé au trou noir.
    #[test]
    fn chaque_type_particulier_est_atteignable() {
        let vus = balayage();
        for nom in ATTENDUS {
            assert!(vus.contains(&nom), "{nom} est injoignable au tirage");
        }
        // Et rien d'autre ne sort : un nom inconnu ne serait rattaché à aucun
        // rendu, ni à `est_remnant`.
        for nom in &vus {
            assert!(ATTENDUS.contains(nom), "type inattendu au tirage : {nom}");
        }
    }

    /// **On peut rencontrer un trou noir en explorant** — c'est tout l'objet
    /// du lot. Un cinquième des particulières, donc environ 1 % des étoiles.
    #[test]
    fn un_trou_noir_est_rencontrable() {
        let part = balayage().iter().filter(|n| **n == "Trou noir").count() as f32 / 1000.0;
        assert!(part > 0.1, "le trou noir ne tient que {part} des particulières : introuvable");
        assert!(part < 0.35, "il en tient {part} : ce n'est plus une rencontre, c'est la règle");
        // Sur l'ensemble du tirage, il reste une rareté.
        let global = part * PART_PARTICULIERES;
        assert!((0.002..0.02).contains(&global), "{global} de tous les tirages");
    }

    /// **Un trou noir porte le mode de couronne 5** — celui que
    /// `Soleil::avec_trou_noir` attend, et sur lequel `eruptions::maj`
    /// s'arrête (pas de taches sur un horizon).
    #[test]
    fn un_trou_noir_porte_son_mode_de_couronne() {
        let (_, _, _, lumi, nom, couronne) = particuliere(0.0);
        assert_eq!(nom, "Trou noir");
        assert_eq!(couronne, 5.0, "le rendu de trou noir ne se déclenchera pas");
        assert!(lumi > 0.0, "un trou noir tout noir n'éclairerait plus rien de son disque");
    }

    /// **Et il n'a pas de zone habitable.** Elle se déduit de la luminosité,
    /// qui ici ne décrit pas une photosphère mais un disque d'accrétion : la
    /// tracer donnerait un anneau d'eau liquide autour d'un trou noir.
    #[test]
    fn un_trou_noir_n_a_pas_de_zone_habitable() {
        let tn = ProfilEtoile {
            temperature: 6500.0,
            couleur: Vec3::ONE,
            rayon: 0.62,
            luminosite: 0.4,
            nom: "Trou noir",
            couronne: 5.0,
        };
        assert!(tn.est_trou_noir());
        assert!(tn.est_remnant(), "une zone habitable serait tracée autour de l'horizon");
    }
}

#[cfg(test)]
mod tests_ordinaires {
    use super::*;

    /// Les noms que la table des ordinaires doit savoir produire.
    /// Les classes de la **séquence principale**, séparées du reste : c'est
    /// d'elles que le tirage doit rester majoritairement fait.
    const SEQUENCE: [&str; 7] = [
        "M (naine rouge)",
        "K (orange)",
        "G (jaune)",
        "F (blanc-jaune)",
        "A (blanche)",
        "B (bleu-blanc)",
        "O (bleue)",
    ];

    const ATTENDUS: [&str; 16] = [
        "Geante rouge",
        "Naine blanche",
        "Supergeante rouge",
        "Sous-naine (sdB)",
        "Etoile carbonee (C)",
        "Naine brune",
        "Cepheide",
        "Mira",
        "Naine blanche pulsante (ZZ Ceti)",
        "M (naine rouge)",
        "K (orange)",
        "G (jaune)",
        "F (blanc-jaune)",
        "A (blanche)",
        "B (bleu-blanc)",
        "O (bleue)",
    ];

    fn balayage() -> Vec<&'static str> {
        (0..2000).map(|k| ordinaire(k as f32 / 2000.0, 0.5).4).collect()
    }

    /// **Chaque type ordinaire est atteignable.**
    ///
    /// ⚠️ La garde qui manquait, et c'est *elle* le lot 6 autant que les types
    /// eux-mêmes : la table vivait dans la chaîne de `if` d'`aleatoire`, donc
    /// aucun test ne pouvait dire qu'un type en était sorti. Une borne mal
    /// posée en écarte un **en silence** — son rendu continue d'exister,
    /// personne ne le rencontre jamais. C'est arrivé au trou noir, et les
    /// particulières ont reçu cette garde au lot 4 ; les ordinaires ne
    /// l'avaient pas.
    #[test]
    fn chaque_type_ordinaire_est_atteignable() {
        let vus = balayage();
        for nom in ATTENDUS {
            assert!(vus.contains(&nom), "{nom} est injoignable au tirage");
        }
        for nom in &vus {
            assert!(ATTENDUS.contains(nom), "type inattendu au tirage : {nom}");
        }
    }

    /// **Les trois types du lot 6 sortent assez souvent pour qu'on les
    /// rencontre**, et restent moins courants que la séquence principale.
    #[test]
    fn les_types_du_lot_6_se_rencontrent_sans_devenir_la_regle() {
        let vus = balayage();
        let part = |nom: &str| vus.iter().filter(|n| **n == nom).count() as f32 / vus.len() as f32;
        for nom in ["Supergeante rouge", "Sous-naine (sdB)", "Etoile carbonee (C)"] {
            let p = part(nom);
            assert!(p > 0.01, "{nom} ne tient que {p} du tirage : introuvable");
            assert!(p < 0.10, "{nom} en tient {p} : ce n'est plus une variété, c'est la norme");
        }
        // **La séquence principale reste l'ordinaire de l'ordinaire.**
        //
        // ⚠️ Mesurée sur ses **sept** classes, et non trois. Elle n'en comptait
        // que M, K et G : à chaque type exotique ajouté la somme baissait
        // mécaniquement, et le test s'est mis au rouge au lot 7 sans que la
        // séquence ait cessé d'être majoritaire. Un seuil qu'un ajout légitime
        // fait tomber ne garde pas ce qu'il prétend garder.
        let sp: f32 = SEQUENCE.iter().map(|n| part(n)).sum();
        assert!(sp > 0.6, "la séquence principale ne tient plus que {sp} du tirage");
    }

    /// **`q` fait vraiment varier les types à plage**, sinon deux géantes
    /// rouges sont jumelles — le défaut que le lot 1 a été fait pour corriger,
    /// et qu'une table de constantes figées ramènerait.
    #[test]
    fn le_second_tirage_ecarte_les_jumelles() {
        for p in [0.04_f32, 0.11, 0.16, 0.19, 0.22] {
            let (_, _, r0, l0, nom, _) = ordinaire(p, 0.0);
            let (_, _, r1, l1, _, _) = ordinaire(p, 1.0);
            assert!(r1 > r0, "{nom} : rayon figé à {r0}");
            assert!(l1 > l0, "{nom} : luminosité figée à {l0}");
        }
        // Et `q` reste borné : hors de [0 ; 1] on ne sort pas de la plage.
        let (_, _, r_bas, _, _, _) = ordinaire(0.04, -5.0);
        let (_, _, r_haut, _, _, _) = ordinaire(0.04, 9.0);
        assert_eq!(r_bas, ordinaire(0.04, 0.0).2);
        assert_eq!(r_haut, ordinaire(0.04, 1.0).2);
    }

    /// **L'étoile carbonée est la plus rouge du catalogue — c'est sa seule
    /// raison d'être.**
    ///
    /// ⚠️ Sans le filtre moléculaire elle serait *moins* rouge que les naines M
    /// les plus froides, malgré une température inférieure : le corps noir seul
    /// ne descend pas assez bas dans le vert. Ce test compare donc au **pire
    /// cas** de la concurrence, pas à une valeur écrite à la main.
    #[test]
    fn la_carbonee_est_la_plus_rouge_du_catalogue() {
        // Le plus rouge de chaque type : sa borne froide.
        let plus_rouge = |nom: &str, t: f32| {
            let c = couleur_etoile(nom, t);
            (c.y + c.z) / c.x.max(1e-3) // plus petit = plus rouge
        };
        let carbonee = plus_rouge("Etoile carbonee (C)", 3200.0); // sa borne CHAUDE
        for (nom, t) in [
            ("M (naine rouge)", 2400.0),
            ("Geante rouge", 3000.0),
            ("Supergeante rouge", 3200.0),
            ("K (orange)", 3700.0),
        ] {
            let autre = plus_rouge(nom, t);
            assert!(
                carbonee < autre * 0.8,
                "carbonée ({carbonee:.3}) pas franchement plus rouge que {nom} ({autre:.3})"
            );
        }
        // Et le filtre ne casse pas l'ordre des canaux : une teinte qui
        // remonterait le bleu au-dessus du rouge saturerait vers le cyan.
        let c = couleur_etoile("Etoile carbonee (C)", 2900.0);
        assert!(c.x >= c.y && c.y >= c.z, "teinte mal ordonnée : {c:?}");
    }

    /// **La supergéante rouge se distingue par sa taille et son éclat**, pas
    /// par sa teinte : 150 K d'écart avec une géante ne se voient pas.
    #[test]
    fn la_supergeante_depasse_la_geante_en_taille_et_en_eclat() {
        // Le plus petit exemplaire de supergéante contre le plus grand de
        // géante : c'est le cas limite qui décide, pas les médianes.
        let (_, _, r_sup, l_sup, nom, _) = ordinaire(0.16, 0.0);
        assert_eq!(nom, "Supergeante rouge");
        let (_, _, r_geante, l_geante, _, _) = ordinaire(0.04, 1.0);
        assert!(r_sup > r_geante * 1.2, "supergéante {r_sup} contre géante {r_geante}");
        assert!(l_sup > l_geante * 1.2, "éclat {l_sup} contre {l_geante}");
    }

    /// **Les trois types variables pulsent, et eux seuls.**
    ///
    /// ⚠️ C'est leur **seule** raison d'être pour deux d'entre eux : Mira
    /// recouvre la géante rouge en teinte comme en taille, et ZZ Ceti a le
    /// gabarit d'une naine blanche au chiffre près. Une pulsation nulle en
    /// ferait donc des doublons silencieux — des étiquettes, ce que le §3 bis
    /// interdit. Le test compare chacun à son jumeau non pulsant.
    #[test]
    fn les_trois_variables_pulsent_et_elles_seules() {
        for nom in ["Cepheide", "Mira", "Naine blanche pulsante (ZZ Ceti)"] {
            let (amp, periode) = pulsation(nom);
            assert!(amp > 0.0, "{nom} ne pulse pas : c'est un doublon");
            assert!(periode > 0.0, "{nom} : période nulle, la pulsation serait inerte");
            // ⚠️ **Pas épileptique** (§8 de la conception). Une seconde de
            // période est déjà un battement soutenu ; en deçà on quitte la
            // pulsation pour le clignotement.
            assert!(periode >= 1.0, "{nom} : {periode} s, c'est un clignotement");
            // ⚠️ Et l'amplitude reste sous la marge orbitale. `premiere_orbite`
            // pose la planète interne à 1,5 rayon : au-delà de +50 %, une étoile
            // gonflée l'avalerait.
            assert!(amp < 0.5, "{nom} : {amp} d'amplitude, elle toucherait sa planète interne");
        }
        // Leurs jumeaux, eux, ne pulsent pas — sinon il n'y aurait plus de
        // jumeau, et la comparaison ne dirait rien.
        for nom in ["Geante rouge", "Naine blanche", "Supergeante rouge", "G (jaune)"] {
            assert_eq!(pulsation(nom), (0.0, 0.0), "{nom} s'est mise à pulser");
        }
        // Mira est la plus ample, ZZ Ceti la plus rapide : c'est ce qui les
        // distingue l'une de l'autre.
        assert!(pulsation("Mira").0 > pulsation("Cepheide").0);
        assert!(pulsation("Naine blanche pulsante (ZZ Ceti)").1 < pulsation("Cepheide").1);
    }

    /// **ZZ Ceti et la naine blanche ont le même gabarit** — et c'est voulu.
    ///
    /// Si un jour elles se distinguaient autrement que par la pulsation, ce
    /// commentaire serait faux et ce test doit le dire : on croirait garder une
    /// équivalence qui n'existe plus.
    #[test]
    fn zz_ceti_ne_se_distingue_d_une_naine_blanche_que_par_sa_pulsation() {
        let trouve = |cible: &str| {
            (0..3000)
                .map(|k| ordinaire(k as f32 / 3000.0, 0.5))
                .find(|g| g.4 == cible)
                .expect("type absent du tirage")
        };
        let (bmin, bmax, br, bl, ..) = trouve("Naine blanche");
        let (zmin, zmax, zr, zl, ..) = trouve("Naine blanche pulsante (ZZ Ceti)");
        assert!((zr - br).abs() < 0.05, "rayons écartés : {zr} contre {br}");
        assert!((zl - bl).abs() < 0.05, "éclats écartés : {zl} contre {bl}");
        // ⚠️ La température, elle, n'est **pas** identique, et c'est voulu : la
        // bande d'instabilité des DAV est à 10 500–12 500 K, soit l'extrémité
        // **froide** de la famille. ZZ Ceti est une naine blanche qui a
        // refroidi jusqu'à se mettre à battre — pas n'importe quelle naine
        // blanche. Elle vit donc au bas de la plage, voire juste en dessous, et
        // jamais dans son haut.
        assert!(zmax <= bmax, "ZZ Ceti monte au-delà des naines blanches les plus chaudes");
        assert!(
            zmax < bmin + 1000.0,
            "ZZ Ceti à {zmax} K : elle a quitté la bande d'instabilité pour le gros de la famille"
        );
        assert!(zmin > bmin * 0.8, "ZZ Ceti à {zmin} K : trop froide pour une naine blanche");
        assert_eq!(pulsation("Naine blanche").0, 0.0);
        assert!(pulsation("Naine blanche pulsante (ZZ Ceti)").0 > 0.0);
    }

    /// **La naine brune n'est pas une photosphère, et n'est pas un rémanent.**
    ///
    /// ⚠️ Le prédicat est ce qui décide de **quel corps** on bâtit : trois
    /// endroits le consultent (genèse simple, feuilles d'un multiple, vue
    /// isolée). Elle partage deux traits avec les rémanents — orbites serrées,
    /// pas de zone habitable traçable — et les confondre lui collerait une
    /// couronne de pulsar.
    #[test]
    fn la_naine_brune_n_est_ni_une_photosphere_ni_un_remnant() {
        let (_, _, rayon, lumi, nom, couronne) = ordinaire(0.25, 0.5);
        assert_eq!(nom, "Naine brune");
        assert!(est_naine_brune(nom), "le prédicat ne la reconnaît pas");
        let p = ProfilEtoile {
            temperature: 1200.0,
            couleur: Vec3::ONE,
            rayon,
            luminosite: lumi,
            nom,
            couronne,
        };
        assert!(p.est_naine_brune());
        assert!(!p.est_remnant(), "prise pour un rémanent : elle en prendrait la couronne");
        assert!(!p.est_trou_noir());
        // ⚠️ Assez lumineuse pour éclairer ses mondes. Sa valeur réelle
        // (~10⁻⁵ L☉) les laisserait noirs et sa fiche annoncerait 0 K.
        assert!(lumi > 0.1, "luminosité {lumi} : ses planètes seront noires");
        // Et elle reste la plus terne du catalogue, hors rémanents.
        for autre in ["Geante rouge", "Supergeante rouge", "Sous-naine (sdB)", "Etoile carbonee (C)"] {
            let l = (0..2000)
                .map(|k| ordinaire(k as f32 / 2000.0, 0.0))
                .find(|g| g.4 == autre)
                .expect("type absent")
                .3;
            assert!(lumi < l, "la naine brune ({lumi}) brille plus que {autre} ({l})");
        }
        // Aucun filtre de teinte : son aspect vient de son apparence gazeuse,
        // pas d'une couleur de corps noir corrigée.
        assert_eq!(filtre_teinte(nom), Vec3::ONE);
    }

    /// **La sous-naine se distingue par son éclat, et par lui seul.**
    ///
    /// ⚠️ Sa couleur est celle d'une naine blanche — mesuré, (0,62 / 0,74 /
    /// 1,00) contre (0,67 / 0,78 / 1,00). Un test sur la teinte passerait au
    /// vert sans rien garantir. Ce qui la rend visible est son halo : cœur
    /// d'hélium en fusion contre braise qui refroidit.
    #[test]
    fn la_sous_naine_se_distingue_de_la_naine_blanche_par_son_eclat() {
        let (_, _, _, l_sdb, nom, _) = ordinaire(0.19, 0.0);
        assert_eq!(nom, "Sous-naine (sdB)");
        let (_, _, _, l_blanche, _, _) = ordinaire(0.11, 1.0); // la plus brillante
        assert!(l_sdb > l_blanche * 3.0, "sdB {l_sdb} contre naine blanche {l_blanche}");
        // Et la teinte, elle, ne les sépare PAS : si un jour elle le faisait,
        // ce commentaire serait faux et ce test doit le dire.
        let (a, b) = (couleur_etoile("Sous-naine (sdB)", 32000.0), couleur_etoile("Naine blanche", 20000.0));
        assert!((a - b).length() < 0.15, "les teintes se sont écartées : {a:?} vs {b:?}");
    }
}

#[cfg(test)]
mod tests_activite {
    use super::activite_type;

    /// **La hiérarchie qu'on cherche : une naine M dépasse une géante.**
    /// Gabarits tirés de `ProfilEtoile::aleatoire` (`etoile.rs:46` et `:38`).
    #[test]
    fn une_naine_m_est_plus_active_qu_une_geante_rouge() {
        let naine_m = activite_type(1.3, 0.4);
        let geante = activite_type(5.5, 4.0);
        assert!(naine_m > geante, "naine {naine_m} <= géante {geante}");
    }

    /// Une jeune étoile (T Tauri, petite et modérément lumineuse) reste active,
    /// pas assommée par sa luminosité un peu plus haute qu'une naine M.
    #[test]
    fn une_jeune_etoile_reste_active() {
        let t_tauri = activite_type(0.9, 1.2);
        let geante = activite_type(5.5, 4.0);
        assert!(t_tauri > geante, "T Tauri {t_tauri} <= géante {geante}");
    }

    /// **Monotone et borné**, tel que réclamé : grossir le rayon (à luminosité
    /// fixe) ne peut que faire baisser ou stagner l'activité, jamais grimper ;
    /// et le résultat reste dans `[0,15 ; 1,0]` sur toute la plage tirée par
    /// `ProfilEtoile::aleatoire`.
    #[test]
    fn monotone_et_borne() {
        let mut precedent = f32::INFINITY;
        for rayon in [0.4_f32, 0.9, 1.3, 2.0, 3.2, 4.0, 5.5, 7.0] {
            let a = activite_type(rayon, 1.0);
            assert!(a <= precedent + 1e-6, "activité remonte à rayon {rayon} : {a} > {precedent}");
            assert!((0.15..=1.0).contains(&a), "rayon {rayon} -> {a}, hors bornes");
            precedent = a;
        }
    }
}

#[cfg(test)]
mod tests_types {
    use super::*;

    /// **Chaque type du tirage est listé une fois, et se retrouve par son nom.**
    ///
    /// C'est ce que le menu « choisir astre » promet : cliquer un nom construit
    /// un système autour de **ce** type. Un nom listé sans gabarit retomberait
    /// sur un tirage quelconque, et le menu mentirait sans que rien ne casse.
    #[test]
    fn chaque_type_du_tirage_se_retrouve_par_son_nom() {
        let types = types_du_tirage();
        for (i, nom) in types.iter().enumerate() {
            assert!(!types[..i].contains(nom), "« {nom} » listé deux fois");
            for q in [0.0, 0.5, 1.0] {
                let g = gabarit_du_type(nom, q).unwrap_or_else(|| panic!("« {nom} » sans gabarit"));
                assert_eq!(g.4, *nom);
            }
        }
        // Les deux tables au complet : 7 particulières, 16 ordinaires. Si ce
        // compte bouge, un type est arrivé ou parti — le menu suit tout seul,
        // ce test dit seulement qu'on l'a remarqué.
        assert_eq!(types.len(), 23, "{types:?}");
        for attendu in ["Trou noir", "T Tauri (jeune)", "Naine brune", "O (bleue)", "Mira"] {
            assert!(types.contains(&attendu), "« {attendu} » absent : {types:?}");
        }
        assert!(gabarit_du_type("Licorne", 0.5).is_none());
    }
}
