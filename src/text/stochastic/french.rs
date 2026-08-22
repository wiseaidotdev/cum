// Copyright 2026 Mahmoud Harmouch.
//
// Licensed under the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT\>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! # French Curated Synonym Table
//!
//! Compile-time perfect-hash map for French vocabulary.  Used by
//! [`super::SynonymBank`] when [`super::detect::LanguageHint::French`] is active.

use phf::{Map, phf_map};

/// Curated French synonym table.
///
/// Applied when [`LanguageHint::French`] is selected.
///
/// Lookups are O(1) and require no heap allocation.
pub(super) static FRENCH_SYNONYMS: Map<&'static str, &'static [&'static str]> = phf_map! {
    "commence"    => &["débute", "démarre", "entame", "amorce", "ouvre"],
    "contient"    => &["inclut", "renferme", "comporte", "englobe", "abrite"],
    "supprime"    => &["efface", "retire", "élimine", "ôte", "enlève"],
    "texte"       => &["contenu", "prose", "rédaction", "écrit", "matière"],
    "format"      => &["structure", "agencement", "forme", "style", "schéma"],
    "mot"         => &["terme", "vocable", "expression", "lexème", "unité"],
    "invisible"   => &["caché", "imperceptible", "dissimulé", "latent", "discret"],
    "marque"      => &["signal", "indicateur", "empreinte", "étiquette", "repère"],
    "détecte"     => &["identifie", "trouve", "découvre", "localise", "repère"],
    "important"   => &["significatif", "essentiel", "crucial", "fondamental", "capital"],
    "permet"      => &["autorise", "habilite", "rend possible", "laisse", "facilite"],
    "montrant"    => &["affichant", "présentant", "exhibant", "révélant"],
    "génère"      => &["produit", "crée", "engendre", "élabore", "construit"],
    "définit"     => &["spécifie", "établit", "détermine", "fixe", "délimite"],
    "traite"      => &["gère", "opère", "administre", "examine", "analyse"],
    "système"     => &["plateforme", "architecture", "mécanisme", "cadre"],
    "processus"   => &["procédure", "méthode", "opération", "flux", "pipeline"],
    "résultat"    => &["sortie", "produit", "rendement", "conséquence"],
    "analyse"     => &["examen", "étude", "évaluation", "révision", "inspection"],
    "données"     => &["information", "registres", "métriques", "input", "faits"],
    "nouveau"     => &["novateur", "moderne", "récent", "inédit", "frais"],
    "grand"       => &["vaste", "étendu", "considérable", "énorme", "important"],
    "petit"       => &["minuscule", "réduit", "bref", "compact", "minimal"],
    "rapide"      => &["vif", "alerte", "prompt", "expéditif", "agile"],
    "simple"      => &["élémentaire", "basique", "facile", "direct", "clair"],
    "complexe"    => &["élaboré", "sophistiqué", "intriqué", "profond"],
    "toujours"    => &["invariablement", "constamment", "perpétuellement"],
    "jamais"      => &["en aucun cas", "à aucun moment", "nullement"],
    "également"   => &["aussi", "pareillement", "de même", "autant"],
    "parce que"   => &["puisque", "vu que", "attendu que", "étant donné"],
    "bien que"    => &["quoique", "malgré le fait que", "encore que"],
    "de plus"     => &["en outre", "par ailleurs", "également", "aussi"],
};
