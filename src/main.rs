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

/// Police Minitel embarquée au binaire (assets/fonts/Minitel.ttf)
const MINITEL_FONT_BYTES: &[u8] = include_bytes!("assets/fonts/Minitel.ttf");

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
    // Chargement synchrone de la police depuis les bytes embarquées
    let minitel_font = load_ttf_font_from_bytes(MINITEL_FONT_BYTES)
        .expect("Erreur: assets/fonts/Minitel.ttf introuvable ou corrompu");

    let mut etat = Etat::Accueil(Accueil::new());

    loop {
        match &mut etat {
            Etat::Accueil(a) => {
                if let Some(cible) = a.frame() {
                    etat = match cible {
                        Cible::Skymap => Etat::Skymap(Box::new(Skymap::new())),
                        Cible::Objet => Etat::Objet(Box::new(Objet::new(minitel_font.clone()))),
                        // On passe la police SEULEMENT à la galerie planétaire
                        Cible::Galerie => {
                            let font = minitel_font.clone();
                            Etat::Galerie(Box::new(Galerie::new(false, font)))
                        }
                        Cible::GalerieGaz => {
                            let font = minitel_font.clone();
                            Etat::Galerie(Box::new(Galerie::new(true, font)))
                        }
                        // Les autres écrans ne touchent pas à la police pour l'instant
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