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
    Accueil, Categorie, Cible, Galerie, GalerieCometes, GalerieDisques, GalerieEtoiles, Objet, Skymap,
    SkymapV2,
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
            swap_interval: std::env::var("PROFIL").is_ok().then_some(0),
            ..Default::default()
        },
        ..Default::default()
    }
}

/// Écran actif. Les vues lourdes sont boxées pour garder l'enum compact.
enum Etat {
    Accueil(Accueil),
    Starmap(Box<Starmap>),
    Skymap(Box<Skymap>),
    SkymapV2(Box<SkymapV2>),
    Objet(Box<Objet>),
    Galerie(Box<Galerie>),
    GalerieDisques(Box<GalerieDisques>),
    GalerieEtoiles(Box<GalerieEtoiles>),
    GalerieCometes(Box<GalerieCometes>),
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
    // `SYSTEME=<type d'étoile>` : la vue système sur un système de ce type
    // (Chantier-Opti-v2, avec `PROFIL`).
    if let Ok(nom) = std::env::var("SYSTEME") {
        if let Some(n) = crate::etoile::types_du_tirage().into_iter().find(|t| *t == nom) {
            etat = Etat::SkymapV2(Box::new(SkymapV2::sur_astre(n)));
        }
    }
    // `DEFILE=<rang>` : de même, sur un astre du défilé (touche 3).
    if let Some(rang) = std::env::var("DEFILE").ok().and_then(|r| r.parse().ok()) {
        etat = Etat::Objet(Box::new(Objet::sur_defile(rang)));
    }
    let mut frames_capture = 0u32;

    loop {
        match &mut etat {
            Etat::Accueil(a) => {
                if let Some(cible) = a.frame() {
                    etat = match cible {
                        Cible::Starmap => Etat::Starmap(Box::new(Starmap::new())),
                        Cible::Skymap => Etat::Skymap(Box::new(Skymap::new())),
                        Cible::SkymapV2 => Etat::SkymapV2(Box::new(SkymapV2::new())),
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

        next_frame().await;
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
