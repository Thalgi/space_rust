mod astre;
mod camera;
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
mod ui;
mod vaisseau;

use ecran::{
    Parametres,
    Accueil, Categorie, Cible, Galerie, GalerieDisques, GalerieEtoiles, Objet, Skymap,
    SortieStarmap, Starmap, VueStation,
};
use macroquad::prelude::*;

fn window_conf() -> Conf {
    Conf {
        window_title: "Systeme solaire".to_owned(),
        window_width: 1000,
        window_height: 700,
        ..Default::default()
    }
}

/// Écran actif. Les vues lourdes sont boxées pour garder l'enum compact.
enum Etat {
    Accueil(Accueil),
    Starmap(Box<Starmap>),
    Skymap(Box<Skymap>),
    Objet(Box<Objet>),
    Galerie(Box<Galerie>),
    GalerieDisques(Box<GalerieDisques>),
    GalerieEtoiles(Box<GalerieEtoiles>),
    Station(Box<VueStation>),
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

    loop {
        match &mut etat {
            Etat::Accueil(a) => {
                if let Some(cible) = a.frame() {
                    etat = match cible {
                        Cible::Starmap => Etat::Starmap(Box::new(Starmap::new())),
                        Cible::Skymap => Etat::Skymap(Box::new(Skymap::new())),
                        Cible::Objet => Etat::Objet(Box::new(Objet::new())),
                        Cible::Galerie => Etat::Galerie(Box::new(Galerie::new(false))),
                        Cible::GalerieGaz => Etat::Galerie(Box::new(Galerie::new(true))),
                        Cible::GalerieDisques => {
                            Etat::GalerieDisques(Box::new(GalerieDisques::new()))
                        }
                        Cible::GalerieEtoiles => {
                            Etat::GalerieEtoiles(Box::new(GalerieEtoiles::new()))
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
        }
        next_frame().await;
    }
}
