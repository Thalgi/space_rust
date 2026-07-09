//! Critères de stabilité orbitale de **Holman & Wiegert (1999)** pour les binaires.
//! Formules d'ajustement donnant le demi-grand axe critique au-delà / en deçà
//! duquel une planète est éjectée. Servent à ne générer (et n'accepter) que des
//! orbites plausibles dans les systèmes à plusieurs étoiles.
//!
//! Convention : `a_bin` = demi-grand axe du binaire ; `mu` = rapport de masse ;
//! `e` = excentricité du binaire. Résultat dans la même unité que `a_bin`.

/// **Type P (circumbinaire)** : demi-grand axe critique **minimum** pour une orbite
/// stable autour du COUPLE. Une planète est stable si `a > a_crit_p`.
/// `mu = m2/(m1+m2)` (rapport de masse du couple), typiquement ∈ [0.1, 0.5].
pub fn a_crit_p(a_bin: f32, mu: f32, e: f32) -> f32 {
    let mu = mu.clamp(0.1, 0.5);
    let e = e.clamp(0.0, 0.7);
    let r = 1.60 + 5.10 * e - 2.22 * e * e + 4.12 * mu - 4.27 * e * mu - 5.09 * mu * mu
        + 4.61 * e * e * mu * mu;
    a_bin * r
}

/// **Type S (circumstellaire)** : demi-grand axe critique **maximum** pour une orbite
/// stable autour d'UNE étoile du couple. Une planète est stable si `a < a_crit_s`.
/// `mu` = masse de l'AUTRE étoile / masse totale.
#[allow(dead_code)] // utilisé par la génération S-type (chantier suivant)
pub fn a_crit_s(a_bin: f32, mu: f32, e: f32) -> f32 {
    let mu = mu.clamp(0.1, 0.9);
    let e = e.clamp(0.0, 0.7);
    let r = 0.464 - 0.380 * mu - 0.631 * e + 0.586 * mu * e + 0.150 * e * e - 0.198 * mu * e * e;
    a_bin * r.max(0.05)
}

/// Rapport de masse `mu = m_petite / (m1 + m2)` d'un couple, borné [0.1, 0.5].
pub fn rapport_masse(m1: f32, m2: f32) -> f32 {
    (m1.min(m2) / (m1 + m2).max(1e-6)).clamp(0.1, 0.5)
}
