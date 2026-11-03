use crate::vaisseau::montage::cuire;
use crate::vaisseau::{
    Assembleur, Composant, EtatStation, Profil, Repere, VarianteModule,
    rayon_pour_anneau, VarianteMiroir,
};
use macroquad::prelude::*;
use std::f32::consts::{PI, TAU};
use super::elysium::{ELYSIUM_BRAS, ELYSIUM_HABITAT, ELYSIUM_PROFIL, HABITAT_PROFIL, STANFORD_BRAS, STANFORD_ECHELLE, STANFORD_N_BRAS, TUNNEL_MODULE};
use super::demos::{poser_grappe_habitat, HAB_LONG};

/// **Tore de Stanford** : bras courts, ceinturés d'un tore cylindrique.
///
/// Son centre est bâti **séparément** de celui d'Elysium, et non plus par une
/// ossature commune paramétrée. Les deux mégastructures partagent aujourd'hui
/// leur moyeu et leurs bras, mais elles ne sont pas destinées à se ressembler :
/// factoriser un centre unique aurait rendu chaque divergence à venir plus
/// coûteuse que la duplication qu'elle évite. Ce qui reste partagé
/// (`moyeu_hexagonal`, `bras_et_tunnels`) l'est parce que c'est **encore**
/// identique, pas parce que ça doit le rester.
#[cfg(test)]
pub fn preset_stanford() -> EtatStation {
    // Vue d'un seul tenant : les deux moitiés versées dans le même assemblage.
    // Sert au test de silhouette et à qui veut l'objet entier sans animation.
    let mut asm = Assembleur::new();
    verser(&mut asm, preset_stanford_moyeu());
    verser(&mut asm, preset_stanford_anneau());
    asm.terminer()
}

#[cfg(test)]
pub(crate) fn verser(dest: &mut Assembleur, source: EtatStation) {
    super::commun::verser_a_echelle(dest, source, 1.0);
}

/// **La moitié qui NE tourne PAS** : les deux gros cœurs d'habitat axiaux, et
/// le miroir.
///
/// Un tore de Stanford ne tourne pas d'un bloc. L'anneau tourne — c'est lui qui
/// fait la gravité —, et avec lui toute son ossature ; mais la **colonne
/// d'habitat sur l'axe** reste immobile, pour que l'amarrage et les paliers se
/// fassent sur une pièce qui ne tourne pas.
///
/// ⚠️ **Le miroir reste de ce côté-ci**, et ce n'est pas négociable : il doit
/// garder le soleil dans le même axe. Solidaire de l'anneau, il ne renverrait la
/// lumière qu'une fraction du temps — ce qui rendrait vaine l'inclinaison à 30°
/// réglée juste avant.
pub fn preset_stanford_moyeu() -> EtatStation {
    let mut asm = Assembleur::new();
    // **Les deux gros cœurs axiaux, et eux seuls.** C'est la section
    // désolidarisée : la colonne d'habitat sur l'axe reste immobile pendant que
    // tout le reste — ossature centrale comprise — tourne autour d'elle. C'est
    // ainsi qu'un axe « despun » fonctionne : le manège tourne autour du moyeu,
    // pas l'inverse.
    //
    // Bâtis à part, puis reversés **réduits** : la mise à l'échelle porte sur la
    // géométrie, donc sur le diamètre autant que sur la longueur.
    let mut hab = Assembleur::new();
    habitats_accouples(&mut hab, ELYSIUM_HABITAT);
    super::commun::verser_a_echelle(&mut asm, hab.terminer(), STANFORD_ECHELLE);
    // Coiffes et flèches avec eux : **toute** la colonne axiale est immobile.
    // Elles tournaient un temps — sans effet visible, ce sont des corps de
    // révolution posés sur l'axe — mais les mettre du côté fixe dit ce qu'elles
    // sont : la partie de la station où l'on ne pèse rien.
    pilier_polaire(&mut asm);

    // **Les deux miroirs primaires**, inclinés vers le soleil.
    //
    // Un de chaque côté de l'anneau : celui du haut lui renvoie la lumière vers
    // le bas, celui du bas vers le haut. Un seul laissait une moitié du tore
    // éclairée et l'autre non — or l'anneau est vitré sur tout son pourtour.
    //
    // ⚠️ **Les deux regardent le MÊME soleil**, et c'est tout l'enjeu du calcul
    // ci-dessous. Chacun part de la normale qui fait face à l'anneau (−Y en
    // haut, +Y en bas) puis s'incline **vers `SOLEIL`** — donc en sens opposés
    // l'un de l'autre. Les incliner du même signe les ferait regarder deux
    // directions différentes, et l'un des deux ne renverrait plus rien.
    let r_tore = stanford_rayon_tore();
    let miroir = Composant::Miroir {
        profil: ELYSIUM_PROFIL,
        variante: VarianteMiroir::Anneau,
        rayon: rayon_pour_anneau(r_tore),
        creux: 0.05,
    };
    for sens in [1.0_f32, -1.0] {
        // La normale visée : d'abord **vers l'anneau** (−Y en haut, +Y en bas),
        // puis penchée de `MIROIR_INCLINAISON` **vers le soleil**.
        //
        // L'axe du penchement se **déduit** de `SOLEIL` (`n × soleil`) au lieu
        // d'être écrit en dur. Ce n'est pas un raffinement : la première version
        // fixait un `rotation_z` de signe choisi à la main, et se trompait — le
        // miroir du haut regardait à l'opposé du soleil. Un axe calculé ne peut
        // pas se tromper de côté, quel que soit le pôle et quelle que soit la
        // direction qu'on donne au soleil.
        let vers_anneau_n = -sens * Vec3::Y;
        let vers_anneau = Quat::from_rotation_arc(Vec3::Z, vers_anneau_n);
        let axe = vers_anneau_n.cross(SOLEIL).normalize();
        let penche = Quat::from_axis_angle(axe, MIROIR_INCLINAISON);
        asm.ajouter(cuire(
            Repere::new(vec3(0.0, sens * r_tore * MIROIR_HAUTEUR, 0.0), penche * vers_anneau),
            &miroir,
        ));
    }
    asm.terminer()
}

/// **La moitié qui tourne** : ossature centrale, bras, coiffes de pôle et
/// anneau — tout sauf la colonne d'habitat axiale et le miroir.
pub fn preset_stanford_anneau() -> EtatStation {
    let mut asm = Assembleur::new();
    // L'**ossature centrale** tourne avec l'anneau : c'est elle qui le porte,
    // et une charpente qui resterait immobile pendant que sa jante tourne
    // n'aurait rien à quoi se raccrocher.
    moyeu_hexagonal(&mut asm);
    bras_et_tunnels(&mut asm, STANFORD_BRAS, STANFORD_N_BRAS);

    // Le tore boucle **au bout** des bras : son rayon majeur est la distance de
    // l'axe à leur pointe. Sa section vaut la hauteur du moyeu (donc son
    // diamètre, pas son rayon) — le prisme et le tube se lisent alors au même
    // gabarit.
    let tore = Composant::Tore {
        rayon_majeur: stanford_rayon_tore(),
        rayon_mineur: hauteur_moyeu() * 0.5,
        segments: 96,
        anneaux: 16,
        // Trois jonctions, **calées sur les bras** : mêmes angles, une seule
        // source. Un décalage entre les deux mettrait le vitrage là où arrive
        // l'effort — exactement ce qu'on cherche à éviter.
        jonctions: 3,
        phase: PI / 6.0,
    };
    asm.ajouter(cuire(Repere::IDENTITE, &tore));
    asm.terminer()
}

/// **Pilier d'habitat polaire** : une colonne le long de l'axe, qui traverse le
/// moyeu et sort **des deux côtés**, coiffée d'un cœur d'habitat à chaque pôle.
///
/// C'est la partie du tore où la gravité est nulle : docking, observation,
/// industrie. Elle ne tourne pas — d'où sa place dans la moitié fixe.
///
/// ⚠️ **Le fût du pilier, c'est l'habitat axial du moyeu**, pas des segments à
/// part. Une première version empilait trois modules de 3 par côté ; une sonde a
/// montré qu'ils étaient **entièrement enfouis** dans l'habitat accouplé, qui
/// couvre exactement la même tranche (y de 0 à 9) avec un rayon deux fois
/// supérieur. Trois pièces par côté dessinées pour rien, et invisibles — c'est
/// le genre de doublon qu'on ne voit qu'en mesurant.
///
/// Le pilier ne pose donc que ses **coiffes**, au-delà du fût existant.
///
/// Symétrique par construction : la boucle sur `[1, -1]` pose les deux bouts
/// avec le même code, plutôt qu'un haut et un bas écrits séparément qui
/// finiraient par diverger.
pub(crate) fn pilier_polaire(asm: &mut Assembleur) {
    // Bout du fût : l'habitat axial est centré sur l'origine et déployé de part
    // et d'autre, à l'échelle de Stanford. Lire sa cote ici plutôt que d'écrire
    // un nombre — le jour où l'échelle change, la coiffe suit.
    let fut = ELYSIUM_HABITAT * STANFORD_ECHELLE;
    for sens in [1.0_f32, -1.0] {
        // **La coiffe de pôle** : le cœur d'habitat d'Elysium — un fût nu, sans
        // hublot ni coupole. Ni nœud à six sorties ni à quatre : les deux ont
        // été essayés et écartés à l'œil. Un nœud est une **croix**, et une
        // croix au sommet d'une colonne se lit comme un embranchement, alors
        // qu'il n'y a rien à embrancher là-haut. Le tube prolonge le pilier au
        // lieu de le contredire — et c'est la pièce même qu'emploie le complexe
        // Elysium pour ses habitats, donc un seul vocabulaire dans la famille.
        asm.ajouter(cuire(
            Repere::new(
                vec3(0.0, sens * (fut + POLE_COEUR * 0.5), 0.0),
                Quat::from_rotation_x(-sens * PI / 2.0),
            ),
            &Composant::ModuleAxial {
                profil: HABITAT_PROFIL,
                variante: VarianteModule::CoeurGris,
                longueur: POLE_COEUR,
            },
        ));

        // **La flèche polaire** : un second cœur, étiré cinq fois, posé bout à
        // bout au-delà de la coiffe — et **deux fois plus fine**.
        //
        // `FLECHE_PROFIL` est deux crans sous celui de l'habitat : P0 contre
        // P2, soit le quart du diamètre. La colonne s'effile donc franchement —
        // gros cœur, coiffe, puis **tige** — au lieu de filer d'un seul gabarit
        // sur toute sa longueur, ce qui la ferait lire comme un tube unique et
        // non comme un mât.
        let long = POLE_COEUR * POLE_ETIREMENT;
        asm.ajouter(cuire(
            Repere::new(
                vec3(0.0, sens * (fut + POLE_COEUR + long * 0.5), 0.0),
                Quat::from_rotation_x(-sens * PI / 2.0),
            ),
            &Composant::ModuleAxial {
                profil: FLECHE_PROFIL,
                variante: VarianteModule::CoeurGris,
                longueur: long,
            },
        ));
    }
    charge_polaire_basse(asm, fut + POLE_COEUR + POLE_COEUR * POLE_ETIREMENT);
}

/// **Charge utile du mât bas** : la grappe d'habitat de l'ISV, puis les cuves,
/// enfilées sur la flèche du pôle sud.
///
/// ⚠️ **C'est la seule chose qui rompt la symétrie des deux pôles**, et c'est
/// voulu : le haut porte l'amarrage et l'observation, le bas porte ce qui se
/// consomme. Une station symétrique jusqu'au bout n'aurait pas de haut ni de
/// bas ; c'est le déséquilibre qui donne son sens à la colonne.
///
/// Les deux charges partagent la **même figure** — trois volumes en couronne
/// autour d'un axe vide, où passe le mât. Ce n'est pas une coïncidence dont on
/// profite : c'est ce qui fait qu'elles se lisent comme deux étages d'une même
/// pile et non comme deux pièces sans rapport.
///
/// Tout est calé sur le **bout de la flèche** et remonte de là. Ancrer sur le
/// haut aurait laissé la dernière cuve dépasser dans le vide dès qu'une cote
/// change — or ce qui doit rester vrai, c'est que la pile **finit sur le mât**.
pub(crate) fn charge_polaire_basse(asm: &mut Assembleur, bout: f32) {
    let orient = Quat::from_rotation_arc(Vec3::Z, Vec3::Y);

    // Les cuves d'abord : leur pied tombe au bout du mât. Elles se déploient
    // vers +Z depuis leur origine, qu'on retourne donc vers le bas.
    let haut_cuves = bout - crate::vaisseau::grappe_hauteur(POLE_CUVE, POLE_RANGEES);
    asm.ajouter(cuire(
        Repere::new(vec3(0.0, -haut_cuves, 0.0), Quat::from_rotation_x(PI / 2.0)),
        &Composant::GrappeReservoirs {
            profil: FLECHE_PROFIL,
            rayon_cuve: POLE_CUVE,
            rangees: POLE_RANGEES,
        },
    ));

    // L'habitat au-dessus, séparé par une longueur de mât nu. L'écart n'est pas
    // du décor : accolées, les deux couronnes se lisent comme un seul bloc à six
    // volumes, et l'on ne voit plus qu'il y a des gens d'un côté et des ergols
    // de l'autre.
    let centre_hab = haut_cuves - POLE_ECART - HAB_LONG * 0.5;
    poser_grappe_habitat(asm, -centre_hab, orient, FLECHE_PROFIL.rayon());
}

/// Rayon d'une cuve du mât bas, et nombre de rangées.
///
/// Calé pour que la grappe reste **plus étroite** que la couronne d'habitat
/// au-dessus : la pile s'affine en descendant, comme le mât lui-même s'affine
/// sous les cœurs. L'inverse donnerait une massue.
pub(crate) const POLE_CUVE: f32 = 1.6;
pub(crate) const POLE_RANGEES: usize = 3;

/// Longueur de mât nu entre l'habitat et les cuves.
pub(crate) const POLE_ECART: f32 = 6.0;

/// Écart des miroirs au plan de l'anneau, en fraction de son rayon.
///
/// Parti de 0,75, porté au triple (2,25) parce que de près le miroir coiffait le
/// tore comme un couvercle ; ramené d'un tiers (1,5) une fois la paire en place,
/// deux disques aussi lointains ne se lisant plus comme appartenant au même
/// objet.
///
/// ⚠️ **Conséquence à connaître** : les miroirs pèsent sur le rayon englobant de
/// la station, donc sur le recul de la caméra au cadrage automatique.
pub(crate) const MIROIR_HAUTEUR: f32 = 1.5;

/// Direction conventionnelle du soleil.
///
/// L'éclairage de la vue suit la caméra : il n'y a pas de vrai soleil à viser.
/// Cette constante n'existe donc que pour que les **deux** miroirs regardent le
/// même point — c'est leur accord qui compte, pas l'azimut choisi.
pub(crate) const SOLEIL: Vec3 = Vec3::X;

/// Inclinaison des miroirs, **30°** — réglé à l'œil.
///
/// La géométrie pure demanderait 45° : la lumière arrive perpendiculairement à
/// l'axe de rotation, et c'est l'angle qui la renvoie exactement *le long* de
/// cet axe. À plat (0°), le miroir la réfléchirait d'où elle vient — c'était le
/// défaut d'origine.
///
/// 30° est un **arbitrage de lecture**, pas d'optique : à 45° le disque se
/// présente trop de profil et sa surface disparaît sous les angles de caméra
/// usuels. Moins incliné, il montre son reflet tout en restant manifestement
/// de biais.
pub(crate) const MIROIR_INCLINAISON: f32 = PI / 6.0;

/// Longueur du cœur d'habitat qui coiffe chaque pôle.
///
/// **Pas d'antenne au-delà** : elle a été essayée et retirée. Le pilier se
/// termine par son habitat, et une parabole en bout ajoutait un détail
/// d'échelle incompatible avec une pièce longue de plusieurs unités.
pub(crate) const POLE_COEUR: f32 = 5.0;

/// Étirement de la **flèche polaire** — le second cœur, au-delà de la coiffe —
/// en multiples de [`POLE_COEUR`].
pub(crate) const POLE_ETIREMENT: f32 = 10.0;

/// Profil de la flèche : **deux crans** sous `HABITAT_PROFIL`, soit le quart de
/// son diamètre (P0 = 0,5 contre P2 = 2,0).
///
/// Halvée une première fois (P1), puis encore : à P1 la flèche restait un fût,
/// et ce qu'il faut ici est une **tige**. La grille des profils fournit les deux
/// moitiés successives sans qu'aucun facteur soit écrit à la main — P0 est le
/// plus fin qu'elle porte, donc le bout de la course.
pub(crate) const FLECHE_PROFIL: Profil = Profil::P0;

/// Hauteur du moyeu, **et** côté de son hexagone : les faces latérales sont
/// carrées. Lue par le tore, qui accorde son diamètre dessus.
pub(crate) fn hauteur_moyeu() -> f32 {
    2.0 * (ELYSIUM_PROFIL.rayon() * 0.5)
}

/// Distance de l'axe à la **pointe** d'un bras court — le rayon majeur du tore.
pub(crate) fn stanford_rayon_tore() -> f32 {
    pied_de_bras() + STANFORD_BRAS
}

/// Distance de l'axe à la **pointe** d'un bras long — le rayon de la jante
/// d'Elysium, même construction que [`stanford_rayon_tore`] : elle touche le
/// bout des bras, jamais un nombre écrit à part.
pub(crate) fn elysium_rayon_jante() -> f32 {
    pied_de_bras() + ELYSIUM_BRAS
}

/// Distance de l'axe à la **base** d'un bras : apothème du moyeu plus la
/// demi-épaisseur de ses membrures.
pub(crate) fn pied_de_bras() -> f32 {
    let sg = ELYSIUM_PROFIL.rayon() * 0.5;
    let cote = 2.0 * sg;
    cote * 3.0_f32.sqrt() * 0.5 + sg * 0.5
}

/// **Moyeu hexagonal** : deux hexagones dans le plan X‑Z, écartés le long de la
/// normale (+Y) par les montants du premier. Centré sur l'origine, donc l'axe
/// de rotation de la station est Y.
pub(crate) fn moyeu_hexagonal(asm: &mut Assembleur) {
    let haut = hauteur_moyeu();
    let bas_c = Composant::TreillisHexagone { profil: ELYSIUM_PROFIL, liaison: haut };
    let haut_c = Composant::TreillisHexagone { profil: ELYSIUM_PROFIL, liaison: 0.0 };
    asm.ajouter(cuire(Repere::new(vec3(0.0, -haut * 0.5, 0.0), Quat::IDENTITY), &bas_c));
    asm.ajouter(cuire(Repere::new(vec3(0.0, haut * 0.5, 0.0), Quat::IDENTITY), &haut_c));
}

/// **Moyeu d'Elysium** : trois hexagones, pas deux — une **jante médiane** entre
/// les deux plateaux, exactement à la hauteur (Y = 0) où les six bras
/// traversent le moyeu.
///
/// Même hauteur totale que [`moyeu_hexagonal`] (les deux plateaux restent aux
/// mêmes cotes, donc les faces latérales restent carrées et les bras s'y
/// posent sans rattrapage) — seuls les montants changent : deux jeux de six,
/// courts, au lieu d'un seul jeu de six qui traverse tout. Une ceinture de plus
/// à l'endroit où l'ossature encaisse le plus (six bras au lieu de trois), pas
/// une pièce ajoutée pour « expliquer » — la leçon inverse retenue sur le
/// miroir et la grappe de cuves (`docs/suivi/stations.md`, clôture du
/// 2026-08-10) est de ne pas gréer une pièce pour la faire lire comme robuste ;
/// ici c'est la même primitive répétée, pas une addition hétérogène.
pub(crate) fn moyeu_elysium(asm: &mut Assembleur) {
    let demi = hauteur_moyeu() * 0.5;
    let bas_c = Composant::TreillisHexagone { profil: ELYSIUM_PROFIL, liaison: demi };
    let mid_c = Composant::TreillisHexagone { profil: ELYSIUM_PROFIL, liaison: demi };
    let haut_c = Composant::TreillisHexagone { profil: ELYSIUM_PROFIL, liaison: 0.0 };
    asm.ajouter(cuire(Repere::new(vec3(0.0, -demi, 0.0), Quat::IDENTITY), &bas_c));
    asm.ajouter(cuire(Repere::new(Vec3::ZERO, Quat::IDENTITY), &mid_c));
    asm.ajouter(cuire(Repere::new(vec3(0.0, demi, 0.0), Quat::IDENTITY), &haut_c));
}

/// **`n` bras** sur les faces du moyeu, chacun garni de son tunnel.
///
/// Les sommets de l'hexagone sont à 0°, 60°… ; les **milieux de face** sont
/// donc à 30°, 90°, 150°… — six en tout. `n = 3` (Stanford) en prend une sur
/// deux (30°, 150°, 270°) ; `n = 6` (Elysium) les prend **toutes**, sans qu'une
/// seule face du moyeu reste nue.
pub(crate) fn bras_et_tunnels(asm: &mut Assembleur, bras: f32, n: usize) {
    let poutre = Composant::Charpente {
        grand: ELYSIUM_PROFIL,
        petit: Profil::P1,
        longueur: bras,
        courbure: 2.6,
        aiguille: false, // le moyeu remplace le pied : pas de second hexagone
    };
    // Tunnel de circulation : une file de modules **dans** le bras, du moyeu
    // vers la jante. C'est par là qu'on passe de l'anneau au centre — un bras
    // en treillis nu ne mène nulle part.
    //
    // Profil P0 : le bras s'affine jusqu'à `petit` (P1, demi-section 0,5), donc
    // 1 unité de large au bout. Un module P0 fait exactement ce diamètre — il
    // s'y loge sans dépasser, ce qu'aucun profil plus gros ne ferait.
    let tunnel = Composant::ModuleAxial {
        profil: Profil::P0,
        variante: VarianteModule::Standard,
        longueur: TUNNEL_MODULE,
    };
    let n_tunnel = (bras / TUNNEL_MODULE).floor().max(1.0) as usize;
    let pied = pied_de_bras();

    for k in 0..n {
        let a = PI / 6.0 + TAU * k as f32 / n as f32;
        let radial = vec3(a.cos(), 0.0, a.sin());
        let rot = Quat::from_rotation_arc(Vec3::Z, radial);
        asm.ajouter(cuire(Repere::new(radial * (pied + bras * 0.5), rot), &poutre));
        for m in 0..n_tunnel {
            let d = pied + (m as f32 + 0.5) * TUNNEL_MODULE;
            asm.ajouter(cuire(Repere::new(radial * d, rot), &tunnel));
        }
    }
}

/// **Deux habitats accouplés tambour contre tambour** sur l'axe Y.
///
/// Le `Coeur` est une pièce **dissymétrique** : tambour large en −Z, col de
/// docking étroit en +Z. Les joindre par leur grande face donne un corps renflé
/// au milieu et effilé aux deux bouts, dont les **petites** faces regardent
/// l'extérieur du plan de l'anneau. C'est l'accouplement que l'ISV fait déjà
/// entre ses Cœurs 1 et 2 (« tambour contre tambour »).
pub(crate) fn habitats_accouples(asm: &mut Assembleur, longueur: f32) {
    let habitat = Composant::ModuleAxial {
        profil: HABITAT_PROFIL,
        variante: VarianteModule::CoeurGris,
        longueur,
    };
    // Chacun s'éloigne du plan médian : son +Z local (le col) pointe dehors,
    // son −Z (le tambour) vers le centre, où les deux se touchent.
    for sens in [Vec3::Y, Vec3::NEG_Y] {
        let centre = sens * (longueur * 0.5);
        asm.ajouter(cuire(Repere::new(centre, Quat::from_rotation_arc(Vec3::Z, sens)), &habitat));
    }
}
