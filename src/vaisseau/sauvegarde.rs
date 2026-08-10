//! Sauvegarde et chargement d'un chantier — `docs/conception/assembleur.md`
//! §6.4, Lot 5.1.
//!
//! **Ce qui part sur le disque est une [`Recette`]**, pas une géométrie : la
//! liste ordonnée des poses, chaque hôte désigné par une *position* (indice de
//! pièce dans la séquence, indice local de port) et jamais par un
//! `PortLibre::id`, qui n'a de sens que dans la session qui l'a distribué (§6.1).
//!
//! Aucune `Mat4` cuite n'est écrite : rejouer la liste les reconstruit. Ce qui
//! n'est vrai que parce que la construction est **déterministe** — d'où le fait
//! que le déterminisme n'était pas un raffinement mais un **prérequis** de la
//! sauvegarde (§6.4).

use super::{Chantier, Recette};
use std::path::{Path, PathBuf};

/// Dossier des vaisseaux sauvegardés, à côté de `reglages.json`.
pub const DOSSIER: &str = "vaisseaux";

/// Extension des fichiers de recette.
pub const EXTENSION: &str = "json";

/// Chemin du fichier pour un nom de vaisseau donné.
///
/// Le nom est **assaini** : seuls lettres, chiffres, tiret et souligné passent,
/// le reste devient `_`. Un nom de vaisseau est une saisie libre, et il ne doit
/// pas pouvoir désigner un chemin — ni `..`, ni séparateur, ni caractère que
/// Windows refuse dans un nom de fichier.
pub fn chemin(nom: &str) -> PathBuf {
    let sain: String = nom
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect();
    let sain = if sain.trim_matches('_').is_empty() { "vaisseau".to_string() } else { sain };
    Path::new(DOSSIER).join(format!("{sain}.{EXTENSION}"))
}

/// Écrit la recette du chantier. Crée le dossier au besoin.
///
/// Rend l'erreur d'entrée-sortie telle quelle : contrairement aux réglages —
/// qui retombent silencieusement sur leurs défauts parce qu'un réglage perdu
/// n'est rien — **une sauvegarde qui échoue doit se dire**. C'est le travail de
/// l'utilisateur.
pub fn enregistrer(chantier: &Chantier, nom: &str) -> std::io::Result<PathBuf> {
    let chemin = chemin(nom);
    if let Some(dossier) = chemin.parent() {
        std::fs::create_dir_all(dossier)?;
    }
    let texte = serde_json::to_string_pretty(&chantier.recette())
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(&chemin, texte)?;
    Ok(chemin)
}

/// Relit un vaisseau. `None` si le fichier est absent, illisible, mal formé, ou
/// si la recette ne se rejoue pas (`depuis_recette`).
///
/// **Le chantier rendu porte l'historique** : on charge pour continuer à
/// construire, et un vaisseau rechargé sur lequel Annuler ne répond pas serait
/// une surprise désagréable.
pub fn charger(nom: &str) -> Option<Chantier> {
    let texte = std::fs::read_to_string(chemin(nom)).ok()?;
    let recette: Recette = serde_json::from_str(&texte).ok()?;
    Chantier::depuis_recette(&recette).map(|c| c.avec_historique())
}

/// Noms des vaisseaux sauvegardés, triés. Vide si le dossier n'existe pas —
/// c'est le cas au premier lancement, et ce n'est pas une erreur.
pub fn liste() -> Vec<String> {
    let Ok(entrees) = std::fs::read_dir(DOSSIER) else { return Vec::new() };
    let mut noms: Vec<String> = entrees
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let chemin = e.path();
            (chemin.extension()?.to_str()? == EXTENSION)
                .then(|| chemin.file_stem()?.to_str().map(str::to_string))?
        })
        .collect();
    noms.sort();
    noms
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vaisseau::{Composant, Profil, VarianteModule};

    fn module() -> Composant {
        Composant::ModuleAxial { profil: Profil::P1, variante: VarianteModule::Standard, longueur: 2.0 }
    }

    /// Un chantier à trois pièces, dont une branche.
    fn chantier() -> Chantier {
        let mut ch = Chantier::new();
        ch.racine(module());
        let p0 = ch.libres()[0].id;
        ch.poser(p0, module(), 1);
        let b = ch.derniere_piece().unwrap();
        let sur_b = ch.libres().iter().find(|p| p.origine == b).map(|p| p.id).unwrap();
        ch.poser(sur_b, module(), 1);
        ch
    }

    /// **L'aller-retour par le disque reproduit la géométrie cuite** (§6.4).
    ///
    /// C'est LA propriété de la sauvegarde, et elle passe par le vrai fichier :
    /// sérialiser en mémoire prouverait que serde fonctionne, pas que le format
    /// écrit se relit.
    #[test]
    fn un_vaisseau_ecrit_puis_relu_est_le_meme() {
        let ch = chantier();
        let nom = "_essai_aller_retour";
        let chemin = enregistrer(&ch, nom).expect("écriture");
        let relu = charger(nom).expect("relecture");
        let _ = std::fs::remove_file(&chemin);

        assert_eq!(relu.nb_pieces(), ch.nb_pieces(), "le nombre de pièces a changé");
        assert_eq!(relu.libres().len(), ch.libres().len(), "le nombre de ports libres a changé");
        // Géométrie : chaque pièce, dans l'ordre de pose, au même endroit.
        let poses = |c: &Chantier| c.pieces().map(|(_, p)| p.centre()).collect::<Vec<_>>();
        let (a, b) = (poses(&ch), poses(&relu));
        assert_eq!(a.len(), b.len());
        for (i, (av, ap)) in a.iter().zip(&b).enumerate() {
            assert!(av.distance(*ap) < 1e-4, "pièce {i} : {av:?} devenue {ap:?}");
        }
    }

    /// **Un fichier absent ou abîmé ne panique pas.** Le dossier est éditable à
    /// la main, et un JSON d'une version antérieure doit se refuser proprement.
    #[test]
    fn un_fichier_absent_ou_casse_rend_none() {
        assert!(charger("_ce_vaisseau_n_existe_pas_du_tout").is_none());

        let nom = "_essai_casse";
        let chemin = chemin(nom);
        std::fs::create_dir_all(chemin.parent().unwrap()).unwrap();
        std::fs::write(&chemin, "{ pas du json }").unwrap();
        let lu = charger(nom);
        let _ = std::fs::remove_file(&chemin);
        assert!(lu.is_none(), "un JSON cassé a produit un chantier");
    }

    /// **Un nom de vaisseau ne peut pas désigner un chemin.** C'est une saisie
    /// libre : sans assainissement, `../reglages` écrirait hors du dossier.
    #[test]
    fn un_nom_ne_sort_jamais_du_dossier() {
        for hostile in ["../reglages", "..\\..\\systeme", "a/b/c", "nul:", "  "] {
            let c = chemin(hostile);
            assert_eq!(
                c.parent().map(Path::to_path_buf),
                Some(PathBuf::from(DOSSIER)),
                "{hostile} : sorti du dossier ({c:?})"
            );
            let tige = c.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            assert!(
                tige.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_'),
                "{hostile} : caractère non assaini dans « {tige} »"
            );
        }
    }

    /// Le chantier rechargé sait **annuler** : on charge pour continuer à
    /// construire, pas seulement pour regarder.
    #[test]
    fn un_vaisseau_recharge_garde_son_historique() {
        let nom = "_essai_historique";
        let chemin = enregistrer(&chantier(), nom).expect("écriture");
        let mut relu = charger(nom).expect("relecture");
        let _ = std::fs::remove_file(&chemin);

        let avant = relu.nb_pieces();
        let libre = relu.libres().iter().find(|p| relu.peut_poser(p.id, &module(), 1)).map(|p| p.id);
        let Some(libre) = libre else { panic!("nulle part où poser sur le vaisseau rechargé") };
        assert!(relu.poser(libre, module(), 1));
        assert!(relu.annuler(), "le chantier rechargé n'a pas d'historique");
        assert_eq!(relu.nb_pieces(), avant, "l'annulation n'a pas défait la pose");
    }
}
