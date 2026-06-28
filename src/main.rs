mod astre;
mod camera;
mod ceinture;
mod ecran;
mod etoile;
mod fond;
mod genese;
mod impostor;
mod menu;
mod planete;
mod rendu;
mod soleil;
mod systeme;
mod ui;

use ecran::{Accueil, Cible, Galerie, GalerieEtoiles, Objet, Skymap};
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
    Skymap(Box<Skymap>),
    Objet(Box<Objet>),
    Galerie(Box<Galerie>),
    GalerieEtoiles(Box<GalerieEtoiles>),
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut etat = Etat::Accueil(Accueil::new());

    loop {
        match &mut etat {
            Etat::Accueil(a) => {
                if let Some(cible) = a.frame() {
                    etat = match cible {
                        Cible::Skymap => Etat::Skymap(Box::new(Skymap::new())),
                        Cible::Objet => Etat::Objet(Box::new(Objet::new())),
                        Cible::Galerie => Etat::Galerie(Box::new(Galerie::new(false))),
                        Cible::GalerieGaz => Etat::Galerie(Box::new(Galerie::new(true))),
                        Cible::GalerieEtoiles => {
                            Etat::GalerieEtoiles(Box::new(GalerieEtoiles::new()))
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
            Etat::GalerieEtoiles(g) => {
                if g.frame() {
                    etat = Etat::Accueil(Accueil::new());
                }
            }
        }
        next_frame().await;
    }
}
