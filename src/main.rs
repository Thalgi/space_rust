mod astre;
mod camera;
mod fenetre;
mod comete;
mod disque;
mod ecran;
mod engin;
mod etoile;
mod fond;
mod genese;
mod genese_v2;
mod impostor;
mod menu;
mod orbite;
mod palette;
mod planete;
mod police;
mod reglages;
mod rendu;
mod stabilite;
mod starmap;
mod stellaire;
mod soleil;
mod sprites;
mod systeme;
mod transit;
mod ui;
mod vaisseau;

use ecran::{
    Parametres,
    Accueil, Categorie, Cible, Galerie, GalerieCometes, GalerieDisques, GalerieEtoiles, Objet, Skymap, Tableur,
    SkymapV2, SkymapV3,
    SortieStarmap, Starmap, VueAssembleur, VueStation,
};
use macroquad::prelude::*;

fn window_conf() -> Conf {
    // Fenêtre agrandie sous `CAPTURE` : une icône de sélecteur fait 20 à
    // 50 px à l'écran à la taille normale — illisible pour juger un contour
    // d'un pixel ou un débordement d'anneau au bord d'une case. Sans effet
    // sur le jeu normal.
    let capture = std::env::var("CAPTURE").is_ok();
    Conf {
        window_title: "Systeme solaire".to_owned(),
        window_width: if capture { 2000 } else { 1000 },
        window_height: if capture { 1400 } else { 700 },
        // `assets/logo/model.ico`, embarquée ; l'icône par défaut si illisible.
        icon: fenetre::icone(),
        // Sous `PROFIL`, la synchro verticale coupée : sinon toute mesure
        // plafonne à la fréquence de l'écran, et rien au-dessus ne se
        // distingue (Chantier-Opti-v2). Un indice pour le pilote, pas une
        // garantie.
        platform: miniquad::conf::Platform {
            swap_interval: (std::env::var("PROFIL").is_ok() || std::env::var("DIAG").is_ok()).then_some(0),
            ..Default::default()
        },
        ..Default::default()
    }
}

/// `SKYMAP=v3` : les outils de mesure (`SYSTEME`, `FOCUS`, `DIAG=tour`,
/// `DIAG=systemes`) visent la Skymap V3 au lieu de la V2 (Genèse V2, G2.2).
fn mesure_v3() -> bool {
    std::env::var("SKYMAP").is_ok_and(|s| s == "v3")
}

/// La vue système mesurée, sur un système de ce type d'étoile.
fn vue_sur_astre(t: &'static str) -> Etat {
    if mesure_v3() {
        Etat::SkymapV3(Box::new(SkymapV3::sur_astre(t)))
    } else {
        Etat::SkymapV2(Box::new(SkymapV2::sur_astre(t)))
    }
}

/// **Les graines du tour des systèmes** (`DIAG=systemes`) : réparties, et non
/// 1, 2, 3… En V1 le genre vient de `graine % 100`, et les graines 1 à 54 sont
/// toutes à une étoile : une suite ne mesurerait jamais un multiple.
fn graines_du_tour() -> Vec<u64> {
    (0..60).map(|i| 1 + i * 7919).collect()
}

/// **Moyenne `(image, CPU, erreur type de l'image)` sur les graines d'un tour.**
/// Seules les lignes « graine » comptent : les lignes « systeme X » partagent
/// toutes la graine 1, donc presque le même tirage de planètes. Elles ne sont
/// pas des échantillons, et la V3 ressortait 18 % plus rapide à leur seule faveur
/// (mesuré le 2026-11-05, G2.2).
fn moyenne_des_graines(resultats: &[(String, f32, f32)]) -> (f32, f32, f32) {
    let v: Vec<_> = resultats.iter().filter(|(nom, ..)| nom.starts_with("graine ")).collect();
    let n = v.len().max(1) as f32;
    let (i, c) = (v.iter().map(|r| r.1).sum::<f32>() / n, v.iter().map(|r| r.2).sum::<f32>() / n);
    let variance = v.iter().map(|r| (r.1 - i).powi(2)).sum::<f32>() / (n - 1.0).max(1.0);
    (i, c, (variance / n).sqrt())
}

/// Écran actif. Les vues lourdes sont boxées pour garder l'enum compact.
enum Etat {
    Accueil(Accueil),
    Starmap(Box<Starmap>),
    Skymap(Box<Skymap>),
    SkymapV2(Box<SkymapV2>),
    SkymapV3(Box<SkymapV3>),
    Objet(Box<Objet>),
    Galerie(Box<Galerie>),
    GalerieDisques(Box<GalerieDisques>),
    GalerieEtoiles(Box<GalerieEtoiles>),
    GalerieCometes(Box<GalerieCometes>),
    Tableur(Box<Tableur>),
    Station(Box<VueStation>),
    Assembleur(Box<VueAssembleur>),
    Parametres(Parametres),
}

#[macroquad::main(window_conf)]
async fn main() {
    police::charger().await;
    sprites::charger().await;

    // Les reglages survivent aux changements d'ecran : ils appartiennent au
    // jeu, pas a une vue.
    // Relus du disque : sans ça, chaque lancement repartait en fenêtré 1280x720
    // et rendu net, et il fallait tout reconfigurer avant de pouvoir juger quoi
    // que ce soit à l'écran.
    let mut reglages = reglages::Reglages::charger();
    reglages.appliquer();
    let mut etat = Etat::Accueil(Accueil::new());

    // Harnais de profilage (facultatif) : `PROFIL=<libellé exact>` saute le
    // menu et va droit sur l'item nommé dans les Mégastructures, pour mesurer
    // le fps **réel** plutôt que de l'estimer en triangles retirés — voir
    // `docs/suivi/stations.md`. Dix secondes d'échantillonnage (la première
    // seconde, chargement et premier cadrage compris, est ignorée), puis la
    // fenêtre se ferme d'elle-même et imprime le résultat sur la sortie
    // standard — aucune navigation manuelle, aucun œil requis pour lire le
    // chiffre.
    let profil = std::env::var("PROFIL").ok();
    if let Some(libelle) = &profil {
        let mut v = VueStation::new(Categorie::Megastructures);
        v.choisir(libelle);
        etat = Etat::Station(Box::new(v));
    }
    let mut echantillons: Vec<f32> = Vec::new();
    let debut = get_time();
    // `DIAG` (Opti-v2) : la décomposition du coût sur la machine cible, en
    // un lancement — voir `Diagnostic`.
    // `QUALITE=basse` : avant tout le reste, car la genèse V2 décide de ses
    // particules à la construction d'un système (G2.2).
    if std::env::var("QUALITE").is_ok_and(|q| q == "basse") {
        reglages::regler_qualite(reglages::Qualite::Basse);
    }
    let mode_diag = std::env::var("DIAG").ok();
    let mut tour = match mode_diag.as_deref() {
        Some("tour") => Some(Tour::new(false)),
        // Les systèmes seuls : ce que la genèse fait varier (G2.2).
        Some("systemes") => Some(Tour::new(true)),
        _ => None,
    };
    let mut diag = (mode_diag.is_some() && tour.is_none()).then(Diagnostic::new);
    if let Some(t) = &tour {
        etat = t.etat();
    }

    // Capture d'écran ponctuelle (facultative) : `CAPTURE=<chemin.png>` ouvre
    // la Skymap (système solaire) et exporte l'écran après quelques frames de
    // chauffe (icônes du sélecteur comprises), pour une vérification visuelle
    // sans main sur la souris.
    let capture = std::env::var("CAPTURE").ok();
    if capture.is_some() {
        etat = Etat::Skymap(Box::new(Skymap::new()));
    }
    // `PROTO=<rang>` : la vue isolée, directement sur ce prototype (touche 4).
    // Combiné à `CAPTURE`, de quoi juger un essai sans main sur la souris.
    if let Some(rang) = std::env::var("PROTO").ok().and_then(|r| r.parse().ok()) {
        etat = Etat::Objet(Box::new(Objet::sur_prototype(rang)));
    }
    // `DISQUES=<case>` : la galerie « Ceintures & disques », sur cette case.
    if let Some(case) = std::env::var("DISQUES").ok().and_then(|r| r.parse().ok()) {
        etat = Etat::GalerieDisques(Box::new(GalerieDisques::sur_case(case)));
    }
    // `BANC=<n>` : les deux genèses comparées sur n graines, puis on quitte
    // (Genèse V2, G2.1).
    if let Some(n) = std::env::var("BANC").ok().and_then(|n| n.parse().ok()) {
        genese_v2::banc::lancer(n);
        return;
    }
    // `SYSTEME=<type d'étoile>` : la vue système sur un système de ce type
    // (Chantier-Opti-v2, avec `PROFIL`) ; `SYSTEME=solaire` : le système solaire.
    if let Ok(nom) = std::env::var("SYSTEME") {
        if nom == "solaire" {
            // `FOCUS=<astre>` : cadré de près (`FOCUS_RAYONS`, 4 par défaut).
            let rayons = || std::env::var("FOCUS_RAYONS").ok().and_then(|r| r.parse().ok()).unwrap_or(4.0);
            let focus = std::env::var("FOCUS").ok();
            etat = if mesure_v3() {
                let mut v = SkymapV3::new();
                if let Some(f) = &focus {
                    v.cadrer(f, rayons());
                }
                Etat::SkymapV3(Box::new(v))
            } else {
                let mut v = SkymapV2::new();
                if let Some(f) = &focus {
                    v.cadrer(f, rayons());
                }
                Etat::SkymapV2(Box::new(v))
            };
        } else if let Some(n) =crate::etoile::types_du_tirage().into_iter().find(|t| *t == nom) {
            etat = vue_sur_astre(n);
        }
    }
    // `TABLEUR` : le tableur d'habitabilité (captures).
    if std::env::var("TABLEUR").is_ok() {
        etat = Etat::Tableur(Box::new(Tableur::new()));
    }
    // `GALERIE` : la galerie des planètes (captures).
    if std::env::var("GALERIE").is_ok() {
        etat = Etat::Galerie(Box::new(Galerie::new(false)));
    }
    // `DEFILE=<rang>` : de même, sur un astre du défilé (touche 3).
    if let Some(rang) = std::env::var("DEFILE").ok().and_then(|r| r.parse().ok()) {
        etat = Etat::Objet(Box::new(Objet::sur_defile(rang)));
    }
    // `NUAGES=0` : sans nuages ni bras ; `RENDU=net` : sans filtre — pour
    // juger un voile seul à la capture (Opti-v2 O3).
    if std::env::var("NUAGES").is_ok_and(|v| v == "0") {
        disque::set_nuages(false);
    }
    if std::env::var("RENDU").is_ok_and(|v| v == "net") {
        reglages::regler_rendu(reglages::ModeRendu::Net);
    }
    let mut frames_capture = 0u32;
    let mut echeance = get_time();
    let sans_plafond = std::env::var("PROFIL").is_ok() || std::env::var("DIAG").is_ok();

    loop {
        let debut_image = get_time();
        match &mut etat {
            Etat::Accueil(a) => {
                if let Some(cible) = a.frame() {
                    etat = match cible {
                        Cible::Starmap => Etat::Starmap(Box::new(Starmap::new())),
                        Cible::Skymap => Etat::Skymap(Box::new(Skymap::new())),
                        Cible::SkymapV2 => Etat::SkymapV2(Box::new(SkymapV2::new())),
                        Cible::SkymapV3 => Etat::SkymapV3(Box::new(SkymapV3::new())),
                        Cible::Objet => Etat::Objet(Box::new(Objet::new())),
                        Cible::Galerie => Etat::Galerie(Box::new(Galerie::new(false))),
                        Cible::GalerieGaz => Etat::Galerie(Box::new(Galerie::new(true))),
                        Cible::GalerieDisques => {
                            Etat::GalerieDisques(Box::new(GalerieDisques::new()))
                        }
                        Cible::GalerieEtoiles => {
                            Etat::GalerieEtoiles(Box::new(GalerieEtoiles::new()))
                        }
                        Cible::GalerieCometes => {
                            Etat::GalerieCometes(Box::new(GalerieCometes::new()))
                        }
                        Cible::Tableur => Etat::Tableur(Box::new(Tableur::new())),
                        Cible::Parametres => Etat::Parametres(Parametres::new()),
                        // On sort de la boucle de jeu plutôt que d'appeler
                        // `process::exit` : miniquad ferme alors sa fenêtre
                        // proprement.
                        Cible::Quitter => return,
                        Cible::Briques => {
                            Etat::Station(Box::new(VueStation::new(Categorie::Briques)))
                        }
                        Cible::PetitesStations => {
                            Etat::Station(Box::new(VueStation::new(Categorie::PetitesStations)))
                        }
                        Cible::Generateur => {
                            Etat::Station(Box::new(VueStation::new(Categorie::Generateur)))
                        }
                        Cible::Megastructures => {
                            Etat::Station(Box::new(VueStation::new(Categorie::Megastructures)))
                        }
                        Cible::Vaisseaux => {
                            Etat::Station(Box::new(VueStation::new(Categorie::Vaisseaux)))
                        }
                        // ⚠️ Échap détruit la vue, donc le vaisseau en cours
                        // (`conception/assembleur.md` §10.7 b, arbitrage encore
                        // ouvert). Sans conséquence au Lot 4.1 : rien ne se pose
                        // encore. À trancher avant L4.3.
                        Cible::Assembleur => Etat::Assembleur(Box::new(VueAssembleur::new())),
                    };
                }
            }
            Etat::Starmap(s) => {
                if let Some(sortie) = s.frame() {
                    etat = match sortie {
                        SortieStarmap::Accueil => Etat::Accueil(Accueil::new()),
                        SortieStarmap::Systeme(dest) => {
                            Etat::Skymap(Box::new(Skymap::depuis_destination(dest)))
                        }
                    };
                }
            }
            Etat::Skymap(s) => {
                if s.frame() {
                    etat = Etat::Accueil(Accueil::new());
                }
            }
            Etat::SkymapV2(s) => {
                if s.frame() {
                    etat = Etat::Accueil(Accueil::new());
                }
            }
            Etat::SkymapV3(s) => {
                if s.frame() {
                    etat = Etat::Accueil(Accueil::new());
                }
            }
            Etat::Objet(o) => {
                if o.frame() {
                    etat = Etat::Accueil(Accueil::new());
                }
            }
            Etat::Galerie(g) => {
                if g.frame() {
                    etat = Etat::Accueil(Accueil::new());
                }
            }
            Etat::GalerieDisques(g) => {
                if g.frame() {
                    etat = Etat::Accueil(Accueil::new());
                }
            }
            Etat::GalerieEtoiles(g) => {
                if g.frame() {
                    etat = Etat::Accueil(Accueil::new());
                }
            }
            Etat::GalerieCometes(g) => {
                if g.frame() {
                    etat = Etat::Accueil(Accueil::new());
                }
            }
            Etat::Tableur(t) => {
                if t.frame() {
                    etat = Etat::Accueil(Accueil::new());
                }
            }
            Etat::Parametres(p) => {
                if p.frame(&mut reglages) {
                    etat = Etat::Accueil(Accueil::new());
                }
            }
            Etat::Station(s) => {
                if s.frame() {
                    etat = Etat::Accueil(Accueil::new());
                }
            }
            Etat::Assembleur(a) => {
                if a.frame() {
                    etat = Etat::Accueil(Accueil::new());
                }
            }
        }

        if let Some(d) = &mut diag {
            if d.image(get_frame_time(), (get_time() - debut_image) as f32) {
                return;
            }
        }
        if let Some(t) = &mut tour {
            match t.image(get_frame_time(), (get_time() - debut_image) as f32) {
                Suite::Rester => {}
                Suite::Changer(e) => etat = e,
                Suite::Fin => return,
            }
        }

        if profil.is_some() {
            let ecoule = get_time() - debut;
            if ecoule > 1.0 {
                echantillons.push(get_frame_time());
            }
            if ecoule > 10.0 {
                imprimer_profil(profil.as_deref().unwrap_or(""), &mut echantillons);
                return;
            }
        }

        if let Some(chemin) = &capture {
            frames_capture += 1;
            // Quelques frames de chauffe : le premier rendu d'une icône
            // (cache vide) prend une frame de plus que les suivantes.
            // **Quatre images** (`_0` à `_3`), consécutives par défaut : un
            // clignotement d'une frame à l'autre ne se voit pas sur une image
            // seule. `CAPTURE_PAS=<frames>` les espace, pour suivre ce qui
            // varie sur plusieurs secondes (un battement, une pulsation).
            let pas = std::env::var("CAPTURE_PAS").ok().and_then(|p| p.parse::<u32>().ok()).unwrap_or(1).max(1);
            if frames_capture >= 300 && (frames_capture - 300).is_multiple_of(pas) && (frames_capture - 300) / pas < 4 {
                let n = (frames_capture - 300) / pas;
                let fichier = chemin.replace(".png", &format!("_{n}.png"));
                get_screen_data().export_png(&fichier);
                println!("CAPTURE {fichier}");
                if n == 3 {
                    return;
                }
            }
        }

        // **Le plafond d'images** (Opti-v2 O5) : attendre l'échéance.
        // Pas sous `PROFIL` ni `DIAG` : ils mesurent ce que la machine tient.
        let plafond = if sans_plafond { reglages::Plafond::Libre } else { reglages::plafond() };
        let (attente, suivante) = reglages::attente_plafond(plafond, echeance, get_time());
        echeance = suivante;
        if attente > 0.0 {
            std::thread::sleep(std::time::Duration::from_secs_f64(attente));
        }
        next_frame().await;
    }
}

/// **Le diagnostic de performance sur la machine cible** (Opti-v2, `DIAG=1`,
/// avec `DEFILE` ou `SYSTEME`, synchro coupée) : quelques phases de 5 s, une
/// configuration chacune — tout, sans voile, sans nuages, sans les deux, en
/// rendu net —, puis un tableau sur la sortie standard et la fenêtre se
/// ferme. Pour chaque phase : l'image moyenne, et le temps **CPU** (de la
/// boucle jusqu'à `next_frame`) ; une image bien plus longue que son CPU dit
/// que le GPU est le verrou.
struct Diagnostic {
    phase: usize,
    debut: f64,
    images: Vec<f32>,
    cpu: Vec<f32>,
    /// Remplissage des nuages par image, en écrans (`disque::remplissage_nuages`).
    remplis: Vec<f32>,
    lignes: Vec<String>,
    mode_initial: reglages::ModeRendu,
    qualite_initiale: reglages::Qualite,
    echelle_initiale: reglages::Echelle,
}

impl Diagnostic {
    /// (nom, voile, nuages, rendu imposé, qualité basse). Le rendu Pixel
    /// sépare la cible basse résolution de la quantification de palette ; sans
    /// voile, il dit si l'écart Palette/Pixel tient au voile. « tout (bis) »
    /// répète la première phase : l'écart entre les deux est la dérive de la
    /// machine pendant la mesure (chauffe, fréquence). « voile seul, rendu
    /// Net » : le voile à pleine résolution, sans les nuages qui dominent le
    /// CPU — la phase qui juge O3 sur la machine de dev. Dernier champ : la
    /// résolution de rendu à 50 % (O5, rendu Net).
    #[allow(clippy::type_complexity)]
    const PHASES: [(&'static str, bool, bool, Option<reglages::ModeRendu>, bool, bool); 11] = [
        ("tout", true, true, None, false, false),
        ("sans voile", false, true, None, false, false),
        ("sans nuages ni bras", true, false, None, false, false),
        ("ni voile ni nuages", false, false, None, false, false),
        ("tout, rendu Net", true, true, Some(reglages::ModeRendu::Net), false, false),
        ("tout, rendu Pixel", true, true, Some(reglages::ModeRendu::Pixel), false, false),
        ("sans voile, rendu Pixel", false, true, Some(reglages::ModeRendu::Pixel), false, false),
        ("voile seul, rendu Net", true, false, Some(reglages::ModeRendu::Net), false, false),
        ("tout, qualite basse", true, true, None, true, false),
        ("tout, Net a 50 %", true, true, Some(reglages::ModeRendu::Net), false, true),
        ("tout (bis)", true, true, None, false, false),
    ];

    fn new() -> Self {
        let d = Self {
            phase: 0,
            debut: get_time(),
            images: Vec::new(),
            cpu: Vec::new(),
            remplis: Vec::new(),
            lignes: Vec::new(),
            mode_initial: reglages::mode_rendu(),
            qualite_initiale: reglages::qualite(),
            echelle_initiale: reglages::echelle(),
        };
        disque::mesurer_remplissage(true);
        d.appliquer();
        d
    }

    fn appliquer(&self) {
        let (_, voile, nuages, rendu, basse, moitie) = Self::PHASES[self.phase];
        // Une phase qui impose le rendu Net le prend à 100 %, sauf « à 50 % ».
        reglages::regler_echelle(match (moitie, rendu) {
            (true, _) => reglages::Echelle::Moitie,
            (false, Some(reglages::ModeRendu::Net)) => reglages::Echelle::Pleine,
            _ => self.echelle_initiale,
        });
        disque::set_voiles(voile);
        disque::set_nuages(nuages);
        reglages::regler_qualite(if basse { reglages::Qualite::Basse } else { self.qualite_initiale });
        reglages::regler_rendu(rendu.unwrap_or(self.mode_initial));
    }

    /// Compte une image ; `true` quand tout est mesuré.
    fn image(&mut self, image: f32, cpu: f32) -> bool {
        let t = get_time() - self.debut;
        // La première phase chauffe plus longtemps : sur la cible, à 1 s,
        // « tout » sortait ~6 ms plus lent que sa répétition en fin de DIAG.
        let chauffe = if self.phase == 0 { 4.0 } else { 1.0 };
        let remplis = disque::remplissage_nuages();
        if t > chauffe {
            self.images.push(image);
            self.cpu.push(cpu);
            self.remplis.push(remplis);
        }
        if t < chauffe + 4.0 {
            return false;
        }
        let moy = |v: &[f32]| v.iter().sum::<f32>() / v.len().max(1) as f32 * 1000.0;
        let (i, c) = (moy(&self.images), moy(&self.cpu));
        self.lignes.push(format!(
            "  {:<24} {:>7.1} fps   image {:>6.1} ms   CPU {:>6.1} ms   nuages {:>6.1} écrans",
            Self::PHASES[self.phase].0,
            1000.0 / i.max(1e-3),
            i,
            c,
            moy(&self.remplis) / 1000.0
        ));
        self.phase += 1;
        self.images.clear();
        self.cpu.clear();
        self.remplis.clear();
        self.debut = get_time();
        if self.phase == Self::PHASES.len() {
            println!("DIAG ({} x {}, rendu de départ {:?})", screen_width(), screen_height(), self.mode_initial);
            for l in &self.lignes {
                println!("{l}");
            }
            return true;
        }
        self.appliquer();
        false
    }
}

/// Un cas du tour de diagnostic.
enum Cas {
    Defile(usize),
    Systeme(&'static str),
    Graine(u64),
    Solaire,
    Galerie,
    Starmap,
}

/// Ce que le tour demande à la boucle.
enum Suite {
    Rester,
    Changer(Etat),
    Fin,
}

/// **Le tour de diagnostic** (Opti-v2, `DIAG=tour`) : un lancement passe en
/// revue tout ce qui se dessine — chaque astre du défilé en vue isolée, chaque
/// type d'étoile en vue système, des systèmes engendrés à graine fixe (les
/// mêmes chez l'utilisateur et sur la machine de dev), le système solaire,
/// la galerie des planètes et la starmap —, 3,5 s chacun (1 s de chauffe
/// écartée), dans la qualité et le rendu du joueur. Chaque cas s'imprime à
/// sa fin, puis le tout, du pire au meilleur.
struct Tour {
    cas: Vec<(String, Cas)>,
    i: usize,
    debut: f64,
    images: Vec<f32>,
    cpu: Vec<f32>,
    resultats: Vec<(String, f32, f32)>,
}

impl Tour {
    const CHAUFFE: f64 = 1.0;
    const DUREE: f64 = 3.5;

    /// `seulement_systemes` : ni vues isolées ni galerie ni starmap, que la
    /// genèse ne touche pas (`DIAG=systemes`).
    fn new(seulement_systemes: bool) -> Self {
        let cas = Self::cas(seulement_systemes);
        let skymap = if mesure_v3() { "V3" } else { "V2" };
        println!("TOUR : {} cas, ~{:.0} s, skymap {skymap}, qualite {:?}", cas.len(), cas.len() as f64 * Self::DUREE, reglages::qualite());
        Self { cas, i: 0, debut: get_time(), images: Vec::new(), cpu: Vec::new(), resultats: Vec::new() }
    }

    fn cas(seulement_systemes: bool) -> Vec<(String, Cas)> {
        let mut cas: Vec<(String, Cas)> = if seulement_systemes {
            Vec::new()
        } else {
            Objet::noms_du_defile()
                .into_iter()
                .enumerate()
                .map(|(i, nom)| (format!("isolee  {nom}"), Cas::Defile(i)))
                .collect()
        };
        if seulement_systemes {
            cas.extend(graines_du_tour().into_iter().map(|g| (format!("graine  {g}"), Cas::Graine(g))));
            cas.push(("systeme solaire".into(), Cas::Solaire));
            return cas;
        }
        for t in etoile::types_du_tirage() {
            cas.push((format!("systeme {t}"), Cas::Systeme(t)));
        }
        for g in 1..=8 {
            cas.push((format!("graine  {g}"), Cas::Graine(g)));
        }
        cas.push(("systeme solaire".into(), Cas::Solaire));
        cas.push(("galerie des planetes".into(), Cas::Galerie));
        cas.push(("starmap".into(), Cas::Starmap));
        cas
    }

    /// L'écran du cas courant.
    fn etat(&self) -> Etat {
        match self.cas[self.i].1 {
            Cas::Defile(r) => Etat::Objet(Box::new(Objet::sur_defile(r))),
            Cas::Systeme(t) => vue_sur_astre(t),
            Cas::Graine(g) => {
                if mesure_v3() {
                    Etat::SkymapV3(Box::new(SkymapV3::sur_graine(g)))
                } else {
                    Etat::SkymapV2(Box::new(SkymapV2::sur_graine(g)))
                }
            }
            Cas::Solaire => {
                if mesure_v3() {
                    Etat::SkymapV3(Box::new(SkymapV3::new()))
                } else {
                    Etat::SkymapV2(Box::new(SkymapV2::new()))
                }
            }
            Cas::Galerie => Etat::Galerie(Box::new(Galerie::new(false))),
            Cas::Starmap => Etat::Starmap(Box::new(Starmap::new())),
        }
    }

    fn image(&mut self, image: f32, cpu: f32) -> Suite {
        let t = get_time() - self.debut;
        if t > Self::CHAUFFE {
            self.images.push(image);
            self.cpu.push(cpu);
        }
        if t < Self::DUREE {
            return Suite::Rester;
        }
        let moy = |v: &[f32]| v.iter().sum::<f32>() / v.len().max(1) as f32 * 1000.0;
        let (i, c) = (moy(&self.images), moy(&self.cpu));
        let nom = self.cas[self.i].0.clone();
        println!("TOUR {:>3}/{}  {:<44} {:>7.1} fps  image {:>6.1} ms  CPU {:>6.1} ms", self.i + 1, self.cas.len(), nom, 1000.0 / i.max(1e-3), i, c);
        self.resultats.push((nom, i, c));
        self.images.clear();
        self.cpu.clear();
        self.i += 1;
        self.debut = get_time();
        if self.i == self.cas.len() {
            self.resultats.sort_by(|a, b| b.1.total_cmp(&a.1));
            println!("TOUR, du pire au meilleur ({} x {}, rendu {:?}, qualite {:?}) :", screen_width(), screen_height(), reglages::mode_rendu(), reglages::qualite());
            for (nom, i, c) in &self.resultats {
                println!("  {:<44} {:>7.1} fps  image {:>6.1} ms  CPU {:>6.1} ms", nom, 1000.0 / i.max(1e-3), i, c);
            }
            let (i, c, e) = moyenne_des_graines(&self.resultats);
            println!("TOUR, moyenne des graines : image {i:.3} ms (erreur type {e:.3})  CPU {c:.3} ms");
            return Suite::Fin;
        }
        Suite::Changer(self.etat())
    }
}

/// Résume un profilage fps : moyenne, pire frame, et le « 1 % bas »
/// (percentile 99 du temps de frame — la métrique qui dit si l'expérience
/// **bafouille**, là où la moyenne peut la masquer).
fn imprimer_profil(libelle: &str, echantillons: &mut [f32]) {
    if echantillons.is_empty() {
        println!("PROFIL {libelle} : aucun échantillon (item introuvable ?)");
        return;
    }
    echantillons.sort_by(f32::total_cmp);
    let n = echantillons.len();
    let moyenne = echantillons.iter().sum::<f32>() / n as f32;
    let p99 = echantillons[(((n as f32) * 0.99) as usize).min(n - 1)];
    let pire = *echantillons.last().unwrap();
    println!(
        "PROFIL {libelle} : {n} frames | moyenne {:.1} fps | 1% bas {:.1} fps | pire frame {:.1} fps",
        1.0 / moyenne,
        1.0 / p99,
        1.0 / pire
    );
}

#[cfg(test)]
mod tests_tour {
    use super::*;

    /// `DIAG=systemes` ne garde que les systèmes : les graines réparties, et le
    /// système solaire pour repère.
    #[test]
    fn le_tour_des_systemes_ne_garde_que_les_systemes() {
        let noms = |seuls| Tour::cas(seuls).into_iter().map(|(n, _)| n).collect::<Vec<_>>();
        let (seuls, tous) = (noms(true), noms(false));
        assert_eq!(seuls.len(), graines_du_tour().len() + 1);
        for n in &seuls {
            assert!(n.starts_with("graine ") || n == "systeme solaire", "« {n} » n'a pas sa place");
        }
        assert!(tous.iter().any(|n| n.starts_with("isolee")) && tous.iter().any(|n| n == "starmap"));
    }

    /// **Les graines couvrent les genres de la V1**, tirés de `graine % 100`
    /// (`genese::construire_systeme`) : à une, deux, trois et quatre étoiles,
    /// dans des parts proches de 55 / 28 / 11 / 6 %.
    #[test]
    fn les_graines_du_tour_couvrent_les_genres_de_la_v1() {
        let g = graines_du_tour();
        let part = |lo: u64, hi: u64| g.iter().filter(|&&x| (lo..=hi).contains(&(x % 100))).count() as f32 / g.len() as f32;
        let parts = [part(0, 54), part(55, 82), part(83, 93), part(94, 99)];
        for (p, attendu) in parts.iter().zip([0.55, 0.28, 0.11, 0.06]) {
            assert!((p - attendu).abs() < 0.1, "parts des genres : {parts:?}");
        }
        assert!(parts[3] > 0.0, "aucun quadruple : {parts:?}");
    }

    /// La moyenne ne compte que les graines : les lignes « systeme X » partagent
    /// une seule graine, et les vues sans genèse n'ont rien à y faire.
    #[test]
    fn la_moyenne_ne_compte_que_les_graines() {
        let r = |n: &str, i, c| (n.to_string(), i, c);
        let v = [r("graine  1", 10.0, 1.0), r("graine  2", 20.0, 3.0), r("systeme G", 900.0, 900.0), r("systeme solaire", 900.0, 900.0), r("starmap", 900.0, 900.0)];
        let (i, c, e) = moyenne_des_graines(&v);
        assert_eq!((i, c), (15.0, 2.0));
        // Écart-type 7,07 sur deux échantillons : erreur type 5.
        assert!((e - 5.0).abs() < 1e-4, "erreur type {e}");
    }
}
