//! Générateur procédural de stations, posé sur le [`Chantier`]
//! (`docs/stations_procedurales.md`, §6–7). Une **grammaire** pilote le
//! constructeur : choix d'une ossature, puis habillage des ports libres, le tout
//! borné par un budget et rendu déterministe par une graine.

use super::chantier::Chantier;
use super::montage::{cuire, port_monde, poser};
use super::{
    Assembleur, Composant, EtatStation, GenrePort, Profil, Repere, Sorties, StyleTreillis,
    VarianteAntenne, VarianteModule, VariantePanneau, VarianteRadiateur,
};
use macroquad::prelude::*;

// ---------------------------------------------------------------------------
// RNG déterministe (splitmix64) — pas de dépendance externe.
// ---------------------------------------------------------------------------

pub struct Rng {
    etat: u64,
}

impl Rng {
    pub fn new(graine: u64) -> Self {
        Self { etat: graine ^ 0x9E37_79B9_7F4A_7C15 }
    }

    fn suivant(&mut self) -> u64 {
        self.etat = self.etat.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.etat;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Flottant dans [0, 1).
    fn unite(&mut self) -> f32 {
        (self.suivant() >> 40) as f32 / (1u64 << 24) as f32
    }

    fn entre(&mut self, a: f32, b: f32) -> f32 {
        a + self.unite() * (b - a)
    }

    fn chance(&mut self, p: f32) -> bool {
        self.unite() < p
    }

    fn choix<T: Copy>(&mut self, v: &[T]) -> T {
        v[(self.suivant() as usize) % v.len()]
    }
}

// ---------------------------------------------------------------------------
// Style : contraint les variantes tirées pour une station cohérente.
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Style {
    /// Argent + ambre, façon ISS.
    Historique,
    /// Or + bleu, façon Mir / segment russe.
    Russe,
    /// Métal + cyan, futuriste.
    Futuriste,
}

impl Style {
    pub const TOUS: [Style; 3] = [Style::Historique, Style::Russe, Style::Futuriste];

    pub fn nom(self) -> &'static str {
        match self {
            Style::Historique => "HISTORIQUE",
            Style::Russe => "RUSSE",
            Style::Futuriste => "FUTURISTE",
        }
    }

    fn module(self, rng: &mut Rng) -> VarianteModule {
        use VarianteModule::*;
        rng.choix(match self {
            Style::Historique => &[Standard, Hublots, Labo][..],
            Style::Russe => &[Dore, Hublots][..],
            Style::Futuriste => &[Coupole, Gonflable, Labo][..],
        })
    }

    fn panneau(self, rng: &mut Rng) -> VariantePanneau {
        use VariantePanneau::*;
        rng.choix(match self {
            Style::Historique => &[RigideUS, RollOut][..],
            Style::Russe => &[RusseBleu][..],
            Style::Futuriste => &[Futuriste, Hexagonal][..],
        })
    }

    fn radiateur(self, rng: &mut Rng) -> VarianteRadiateur {
        use VarianteRadiateur::*;
        rng.choix(match self {
            Style::Historique => &[PanneauSimple, AccordeonATCS, Caloducs][..],
            Style::Russe => &[PanneauSimple, PivotantTRRJ][..],
            Style::Futuriste => &[Gouttelettes, Deroulable, Corps][..],
        })
    }

    fn antenne(self, rng: &mut Rng) -> VarianteAntenne {
        use VarianteAntenne::*;
        rng.choix(match self {
            Style::Historique => &[ParaboleGG, Cornets][..],
            Style::Russe => &[Fouet, ParaboleGG][..],
            Style::Futuriste => &[ReseauPhase, Helice][..],
        })
    }
}

// ---------------------------------------------------------------------------
// Paramètres + point d'entrée.
// ---------------------------------------------------------------------------

/// Famille d'ossature.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Ossature {
    /// Poutre-épine type ISS.
    Iss,
    /// Enfilade de modules type Mir.
    Mir,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct ParamsStation {
    pub graine: u64,
    /// Complexité **1..4** : pilote le budget, la longueur et le nombre de branches.
    pub complexite: u8,
    pub style: Style,
    /// `None` = tirée à la graine ; `Some` = forcée (presets).
    pub ossature: Option<Ossature>,
}

impl Default for ParamsStation {
    fn default() -> Self {
        Self { graine: 0, complexite: 2, style: Style::Historique, ossature: None }
    }
}

/// Génère une station complète (déterministe pour une graine donnée).
pub fn generer(p: &ParamsStation) -> EtatStation {
    let c = p.complexite.clamp(1, 4);
    let mut rng = Rng::new(p.graine);
    let budget = 40.0 + c as f32 * 55.0;
    let mut ch = Chantier::avec_budget(budget);

    let iss = match p.ossature {
        Some(o) => o == Ossature::Iss,
        None => rng.chance(0.5),
    };
    if iss {
        ossature_iss(&mut ch, &mut rng, p.style, c);
    } else {
        enfilade_mir(&mut ch, &mut rng, p.style, c);
    }
    // Branchement : modules + **armatures en treillis** (arms, incl. verticales
    // via nœuds Six) sur plusieurs passes, puis habillage proportionnel.
    for _ in 0..c {
        brancher(&mut ch, &mut rng, p.style, c);
    }
    habiller_surface(&mut ch, &mut rng, p.style);
    ch.terminer()
}

// Indices des ports d'un nœud Six : 0=+Z, 1=−Z, 2=+X, 3=−X, 4=+Y, 5=−Y. Le boom
// Z1 et la poutre visent ces faces par index ; le reste du preset docke par
// **direction monde** (`porter_vers`), robuste au basculement des nœuds.
const HUB_X_PLUS: usize = 2;
const HUB_X_MOINS: usize = 3;
const HUB_Y_PLUS: usize = 4;

fn hab(v: VarianteModule) -> Composant {
    Composant::ModuleAxial { profil: Profil::P1, variante: v, longueur: 3.0 }
}
fn hub6() -> Composant {
    Composant::Noeud { profil: Profil::P1, sorties: Sorties::Six }
}

/// Pose `enfant` (par son port `montage`) sur le port `idx` d'un hôte placé en
/// `hote_monde`, l'ajoute à l'assemblage et renvoie son repère monde.
fn poser_sur(
    asm: &mut Assembleur,
    hote_monde: Repere,
    hote: Composant,
    idx: usize,
    enfant: Composant,
    montage: usize,
) -> Repere {
    let m = poser(port_monde(hote_monde, hote, idx), enfant, montage);
    asm.ajouter(cuire(m, enfant));
    m
}

/// Petits arrays russes (bleus) sur les flancs ±X d'un module.
fn arrays_russes(asm: &mut Assembleur, corps: Repere, comp: Composant) {
    for p in comp.ports() {
        if p.genre != GenrePort::Surface || p.repere.avant().x.abs() < 0.9 {
            continue;
        }
        let pan = Composant::PanneauSolaire {
            profil: Profil::P0,
            variante: VariantePanneau::RusseBleu,
            longueur: 3.0,
            largeur: 1.1,
        };
        asm.ajouter(cuire(poser(corps.compose(p.repere), pan, 0), pan));
    }
}

/// Pose `app` sur le port `Surface` d'un module situé du côté `dir` (monde), le
/// premier trouvé. Sert à placer des radiateurs/arrays sur une face précise
/// (ex. nadir −Y) sans dépendre de l'index du port.
fn appendice_sur_module(asm: &mut Assembleur, corps: Repere, comp: Composant, dir: Vec3, app: Composant) {
    for p in comp.ports() {
        if p.genre != GenrePort::Surface {
            continue;
        }
        let w = corps.compose(p.repere);
        if (w.pos - corps.pos).normalize_or_zero().dot(dir) > 0.7 {
            asm.ajouter(cuire(poser(w, app, 0), app));
            return;
        }
    }
}

/// Docke `enfant` (par son port `montage`) sur le port **structurel** de `hote`
/// dont l'**avant monde** pointe le plus vers `dir`. On vise une direction monde
/// plutôt qu'un index de port : les nœuds basculent (demi-tour) à l'accouplement,
/// donc « le port −Z » ne pointe pas forcément vers −Z monde. Renvoie le repère
/// monde de l'enfant.
fn porter_vers(asm: &mut Assembleur, hote_monde: Repere, hote: Composant, dir: Vec3, enfant: Composant, montage: usize) -> Repere {
    let mut best = 0usize;
    let mut best_dot = f32::NEG_INFINITY;
    for (i, p) in hote.ports().iter().enumerate() {
        if p.genre == GenrePort::Surface {
            continue; // on chaîne sur les ports structurels (axiaux/radiaux)
        }
        let avant = (hote_monde.rot * p.repere.avant()).normalize_or_zero();
        let d = avant.dot(dir);
        if d > best_dot {
            best_dot = d;
            best = i;
        }
    }
    let hote_port = hote_monde.compose(hote.ports()[best].repere);
    let w = poser(hote_port, enfant, montage);
    asm.ajouter(cuire(w, enfant));
    w
}

/// Preset : **reproduction de l'ISS**, assemblée à la main (référence pour juger
/// ce qu'il manque au générateur, cf. `docs/iss_reference.md`). Poutre déportée
/// au **zénith** par un boom Z1 (elle ne traverse plus le cœur) ; segment US
/// habité (Destiny→Harmony + Columbus/Kibo + grappe Node3/Cupola + nez PMA) et
/// segment russe (FGB→SM + petits arrays + nœud MRM) fore/aft ; Sas Quest, PMM
/// et radiateurs au nadir sur le cœur.
pub fn preset_iss() -> EtatStation {
    let mut asm = Assembleur::new();

    // Unity (Node 1) — cœur.
    let hub = hub6();
    let hub_m = Repere::IDENTITE;
    asm.ajouter(cuire(hub_m, hub));

    // ===== Poutre au zénith via boom Z1 : Unity(+Y) → Z1 → nœud S0 → poutre ±X.
    // Arrays sur la moitié externe (z < 0), radiateurs sur la moitié interne. =====
    let z1 = Composant::Treillis { profil: Profil::P1, longueur: 3.0, style: StyleTreillis::Carre };
    let z1m = poser_sur(&mut asm, hub_m, hub, HUB_Y_PLUS, z1, 0);
    let s0 = hub6();
    let s0m = poser_sur(&mut asm, z1m, z1, 1, s0, 1);
    for axe in [HUB_X_PLUS, HUB_X_MOINS] {
        let truss = Composant::Treillis { profil: Profil::P2, longueur: 15.0, style: StyleTreillis::Carre };
        let tm = poser_sur(&mut asm, s0m, s0, axe, truss, 0);
        for p in truss.ports() {
            if p.genre != GenrePort::Surface {
                continue;
            }
            let z = p.repere.pos.z; // vers S0 (inboard) ou vers l'extérieur
            let app = if z < -3.5 {
                Composant::PanneauSolaire { profil: Profil::P0, variante: VariantePanneau::RigideUS, longueur: 6.5, largeur: 2.0 }
            } else if z > 3.5 {
                Composant::Radiateur { profil: Profil::P0, variante: VarianteRadiateur::AccordeonATCS, longueur: 3.5, largeur: 1.5 }
            } else {
                continue;
            };
            asm.ajouter(cuire(poser(tm.compose(p.repere), app, 0), app));
        }
    }

    // ===== Segment US (aft, −Z) : Node1 → Destiny → Harmony, ramifié. =====
    let node1 = hub6();
    let n1 = porter_vers(&mut asm, hub_m, hub, Vec3::NEG_Z, node1, 1);
    let lab = hab(VarianteModule::Labo);
    let labm = porter_vers(&mut asm, n1, node1, Vec3::NEG_Z, lab, 1);
    let node2 = hub6();
    let n2 = porter_vers(&mut asm, labm, lab, Vec3::NEG_Z, node2, 1);
    // Columbus (tribord) et Kibō (bâbord) latéraux sur Harmony.
    porter_vers(&mut asm, n2, node2, Vec3::X, hab(VarianteModule::Labo), 1);
    porter_vers(&mut asm, n2, node2, Vec3::NEG_X, hab(VarianteModule::Hublots), 1);
    // Module avant + nez de docking PMA/IDA (adaptateur conique P1→P0).
    let av = hab(VarianteModule::Hublots);
    let avm = porter_vers(&mut asm, n2, node2, Vec3::NEG_Z, av, 1);
    let nez = Composant::Adaptateur { grand: Profil::P1, petit: Profil::P0, longueur: 1.2 };
    porter_vers(&mut asm, avm, av, Vec3::NEG_Z, nez, 0);

    // Grappe Tranquility (Node3) sous Node1 : Cupola (nadir), BEAM (tribord),
    // PMM/Leonardo (bâbord).
    let node3 = hub6();
    let n3 = porter_vers(&mut asm, n1, node1, Vec3::NEG_Y, node3, 1);
    porter_vers(&mut asm, n3, node3, Vec3::NEG_Y, hab(VarianteModule::Coupole), 1);
    porter_vers(&mut asm, n3, node3, Vec3::X, hab(VarianteModule::Gonflable), 1);
    porter_vers(&mut asm, n3, node3, Vec3::NEG_X, hab(VarianteModule::Standard), 1);

    // ===== Segment russe (fore, +Z) : Zarya → Zvezda (arrays) → nœud + MRM. =====
    let fgb = hab(VarianteModule::Dore);
    let fgbm = porter_vers(&mut asm, hub_m, hub, Vec3::Z, fgb, 1);
    arrays_russes(&mut asm, fgbm, fgb);
    let sm = hab(VarianteModule::Dore);
    let smm = porter_vers(&mut asm, fgbm, fgb, Vec3::Z, sm, 1);
    arrays_russes(&mut asm, smm, sm);
    let rn = hub6();
    let rnm = porter_vers(&mut asm, smm, sm, Vec3::Z, rn, 1);
    for dir in [Vec3::Y, Vec3::NEG_Y, Vec3::Z] {
        porter_vers(&mut asm, rnm, rn, dir, hab(VarianteModule::Dore), 1);
    }

    // ===== Sur le cœur : Sas Quest (tribord) + radiateurs nadir sur modules. =====
    porter_vers(&mut asm, hub_m, hub, Vec3::X, hab(VarianteModule::Sas), 1);
    let radia = Composant::Radiateur { profil: Profil::P0, variante: VarianteRadiateur::PanneauSimple, longueur: 2.6, largeur: 1.2 };
    appendice_sur_module(&mut asm, labm, lab, Vec3::NEG_Y, radia);
    appendice_sur_module(&mut asm, smm, sm, Vec3::NEG_Y, radia);

    asm.terminer()
}

/// Preset : une station reconnaissable **type Mir** (modules + nœud), style russe.
pub fn preset_mir() -> EtatStation {
    generer(&ParamsStation { graine: 2, complexite: 2, style: Style::Russe, ossature: Some(Ossature::Mir) })
}

// ---------------------------------------------------------------------------
// Grammaire.
// ---------------------------------------------------------------------------

fn module(style: Style, rng: &mut Rng, longueur: f32) -> Composant {
    Composant::ModuleAxial { profil: Profil::P1, variante: style.module(rng), longueur }
}

/// Nœud connecteur : surtout **Six** (ports ±X et ±Y) pour permettre des branches
/// verticales, parfois Quatre (croix plane).
fn noeud(rng: &mut Rng) -> Composant {
    let sorties = if rng.chance(0.7) { Sorties::Six } else { Sorties::Quatre };
    Composant::Noeud { profil: Profil::P1, sorties }
}

/// Poutre-épine type ISS : treillis + un connecteur (branches) ou un module à
/// chaque bout axial.
fn ossature_iss(ch: &mut Chantier, rng: &mut Rng, style: Style, c: u8) {
    let long = 6.0 + c as f32 * 3.5;
    let st = rng.choix(&[StyleTreillis::Carre, StyleTreillis::Triangulaire]);
    ch.racine(Composant::Treillis { profil: Profil::P1, longueur: long, style: st });

    let bouts: Vec<Vec3> = ch.libres().iter().filter(|p| p.genre == GenrePort::ModuleAxial).map(|p| p.repere.pos).collect();
    for pos in bouts {
        if let Some(i) = ch.libres().iter().position(|p| p.genre == GenrePort::ModuleAxial && p.repere.pos.distance(pos) < 1e-3) {
            if rng.chance(0.6) {
                let n = noeud(rng);
                ch.poser(i, n, 1);
            } else {
                let longueur = rng.entre(2.0, 3.5);
                let m = module(style, rng, longueur);
                ch.poser(i, m, 1);
            }
        }
    }
}

/// Enfilade type Mir : chaîne de modules ponctuée de connecteurs.
fn enfilade_mir(ch: &mut Chantier, rng: &mut Rng, style: Style, c: u8) {
    let longueur = rng.entre(2.0, 3.0);
    ch.racine(module(style, rng, longueur));

    let n = 2 + c as usize;
    for _ in 0..n {
        let Some(i) = ch.libres().iter().position(|p| p.genre == GenrePort::ModuleAxial) else {
            break;
        };
        if rng.chance(0.5) {
            let nd = noeud(rng);
            ch.poser(i, nd, 1);
        } else {
            let longueur = rng.entre(2.0, 3.0);
            let m = module(style, rng, longueur);
            ch.poser(i, m, 1);
        }
    }
}

/// Une passe de branchement : sur les ports radiaux libres, on pose surtout des
/// **armatures en treillis** (qui porteront panneaux/radiateurs, façon ISS) et
/// parfois un module ; sur quelques bouts axiaux, un nœud pour ramifier plus (±Y
/// des nœuds Six → branches **verticales**).
fn brancher(ch: &mut Chantier, rng: &mut Rng, style: Style, c: u8) {
    let radiaux: Vec<Vec3> = ch.libres().iter().filter(|p| p.genre == GenrePort::ModuleRadial).map(|p| p.repere.pos).collect();
    for pos in radiaux {
        if let Some(i) = ch.libres().iter().position(|p| p.genre == GenrePort::ModuleRadial && p.repere.pos.distance(pos) < 1e-3) {
            let r = rng.unite();
            if r < 0.55 {
                // Armature porteuse : un treillis (montage = bout axial, port 0).
                let long = 4.0 + c as f32 * 1.2;
                let st = rng.choix(&[StyleTreillis::Carre, StyleTreillis::Triangulaire]);
                ch.poser(i, Composant::Treillis { profil: Profil::P1, longueur: long, style: st }, 0);
            } else if r < 0.85 {
                let longueur = rng.entre(1.5, 2.5);
                let m = module(style, rng, longueur);
                ch.poser(i, m, 1);
            }
        }
    }

    let axiaux: Vec<Vec3> = ch.libres().iter().filter(|p| p.genre == GenrePort::ModuleAxial).map(|p| p.repere.pos).collect();
    for pos in axiaux {
        if rng.chance(0.2) {
            if let Some(i) = ch.libres().iter().position(|p| p.genre == GenrePort::ModuleAxial && p.repere.pos.distance(pos) < 1e-3) {
                let nd = noeud(rng);
                ch.poser(i, nd, 1);
            }
        }
    }
}

fn fabrique_appendice(style: Style, rng: &mut Rng, categorie: u8) -> Composant {
    match categorie {
        1 => Composant::Radiateur {
            profil: Profil::P0,
            variante: style.radiateur(rng),
            longueur: rng.entre(2.0, 3.0),
            largeur: 1.2,
        },
        2 => Composant::Antenne {
            profil: Profil::P0,
            variante: style.antenne(rng),
            taille: rng.entre(0.9, 1.4),
        },
        _ => Composant::PanneauSolaire {
            profil: Profil::P0,
            variante: style.panneau(rng),
            longueur: rng.entre(2.5, 4.0),
            largeur: 1.4,
        },
    }
}

/// Clé de regroupement d'un port `Surface` : `(axe, perp1, perp2, catégorie)`.
/// L'axe de sortie décide de la **catégorie** — panneaux sur ±X (ailes
/// horizontales, jamais 4 côtés), radiateurs sur ±Y, antennes sur ±Z — et les
/// coordonnées perpendiculaires regroupent les paires opposées (même clé) pour
/// un rendu symétrique.
fn cle_surface(pos: Vec3, avant: Vec3) -> (u8, i64, i64, u8) {
    let q = |v: f32| (v * 4.0).round() as i64;
    if avant.x.abs() > 0.9 {
        // ±X : arrays le long de l'armature — surtout panneaux, ~1/3 radiateurs,
        // par bande axiale (les deux côtés d'une bande partagent la catégorie).
        let bande = q(pos.z);
        let cat = if bande.rem_euclid(3) == 1 { 1 } else { 0 };
        (0, q(pos.y), bande, cat)
    } else if avant.y.abs() > 0.9 {
        (1, q(pos.x), q(pos.z), 1) // ±Y → radiateur
    } else {
        (2, q(pos.x), q(pos.y), 2) // ±Z → antenne
    }
}

/// Habille les ports hôtes `Surface` : un appendice par **groupe symétrique**
/// (même axe + mêmes coordonnées perpendiculaires), la catégorie venant de l'axe.
fn habiller_surface(ch: &mut Chantier, rng: &mut Rng, style: Style) {
    let ports: Vec<(Vec3, Vec3)> = ch
        .libres()
        .iter()
        .filter(|p| p.genre == GenrePort::Surface)
        .map(|p| (p.repere.pos, p.repere.avant()))
        .collect();

    let mut cles: Vec<(u8, i64, i64, u8)> = ports.iter().map(|(p, a)| cle_surface(*p, *a)).collect();
    cles.sort_unstable();
    cles.dedup();

    for cle in cles {
        let app = fabrique_appendice(style, rng, cle.3); // un seul pour tout le groupe
        for (pos, avant) in ports.iter().filter(|(p, a)| cle_surface(*p, *a) == cle) {
            if let Some(i) = ch.libres().iter().position(|q| {
                q.genre == GenrePort::Surface
                    && q.repere.pos.distance(*pos) < 1e-3
                    && (q.repere.avant() - *avant).length() < 1e-3
            }) {
                ch.poser(i, app, 0);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nb(etat: &EtatStation) -> usize {
        etat.doit_dessiner().map(|s| s.nb_pieces()).unwrap_or(0)
    }

    #[test]
    fn generer_produit_une_station() {
        let etat = generer(&ParamsStation { graine: 1, complexite: 2, style: Style::Historique, ossature: None });
        assert!(matches!(etat, EtatStation::Prete(_)));
        assert!(nb(&etat) >= 3, "au moins une ossature garnie");
    }

    #[test]
    fn generer_est_deterministe() {
        let p = ParamsStation { graine: 42, complexite: 3, style: Style::Russe, ossature: None };
        assert_eq!(nb(&generer(&p)), nb(&generer(&p)));
    }

    #[test]
    fn complexite_influe_sur_le_nombre_de_pieces() {
        // Sur une même graine/style, une station complexe a plus de pièces.
        let petite = nb(&generer(&ParamsStation { graine: 7, complexite: 1, style: Style::Futuriste, ossature: None }));
        let grande = nb(&generer(&ParamsStation { graine: 7, complexite: 4, style: Style::Futuriste, ossature: None }));
        assert!(grande >= petite);
    }

    #[test]
    fn ossature_forcee_est_respectee_et_finie() {
        for oss in [Ossature::Iss, Ossature::Mir] {
            for g in 0..10u64 {
                let etat = generer(&ParamsStation { graine: g, complexite: 3, style: Style::Historique, ossature: Some(oss) });
                assert!(matches!(etat, EtatStation::Prete(_)), "{oss:?} graine {g}");
            }
        }
    }

    #[test]
    fn presets_iss_et_mir_produisent_des_stations() {
        assert!(nb(&preset_iss()) >= 5);
        assert!(nb(&preset_mir()) >= 4);
    }

    #[test]
    fn toutes_graines_donnent_une_station_finie() {
        for g in 0..30u64 {
            for style in Style::TOUS {
                let etat = generer(&ParamsStation { graine: g, complexite: 3, style, ossature: None });
                assert!(matches!(etat, EtatStation::Prete(_)), "graine {g} {style:?}");
            }
        }
    }
}
