// Copyright 2026 Mahmoud Harmouch.
//
// Licensed under the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT\>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! # German Curated Synonym Table
//!
//! Compile-time perfect-hash map for German vocabulary.  Used by
//! [`super::SynonymBank`] when [`super::detect::LanguageHint::German`] is active.

use phf::{Map, phf_map};

/// Curated German synonym table.
///
/// Applied when [`LanguageHint::German`] is selected.
///
/// Lookups are O(1) and require no heap allocation.
pub(super) static GERMAN_SYNONYMS: Map<&'static str, &'static [&'static str]> = phf_map! {
    "beginnt"     => &["startet", "eröffnet", "leitet ein", "initiiert", "setzt an"],
    "enthält"     => &["beinhaltet", "umfasst", "schließt ein", "birgt", "hält"],
    "entfernt"    => &["löscht", "beseitigt", "tilgt", "streicht", "eliminiert"],
    "text"        => &["inhalt", "prosa", "schrift", "material", "passage"],
    "format"      => &["struktur", "darstellung", "form", "stil", "anordnung"],
    "wort"        => &["term", "ausdruck", "bezeichnung", "vokabel", "lexem"],
    "unsichtbar"  => &["verborgen", "latent", "versteckt", "verdeckt"],
    "markierung"  => &["zeichen", "signal", "kennung", "hinweis", "etikett"],
    "erkennt"     => &["identifiziert", "findet", "entdeckt", "lokalisiert"],
    "wichtig"     => &["bedeutsam", "wesentlich", "maßgeblich", "erheblich"],
    "ermöglicht"  => &["erlaubt", "gestattet", "befähigt", "ermächtigt"],
    "zeigt"       => &["präsentiert", "stellt dar", "weist aus", "offenbart"],
    "erzeugt"     => &["produziert", "erstellt", "generiert", "bildet"],
    "definiert"   => &["legt fest", "bestimmt", "spezifiziert", "beschreibt"],
    "verarbeitet" => &["behandelt", "bearbeitet", "bewältigt", "managt"],
    "system"      => &["plattform", "architektur", "mechanismus", "rahmen"],
    "prozess"     => &["ablauf", "verfahren", "vorgang", "methode", "pipeline"],
    "ergebnis"    => &["ausgabe", "resultat", "produkt", "folge", "ertrag"],
    "analyse"     => &["untersuchung", "studie", "bewertung", "prüfung"],
    "daten"       => &["informationen", "angaben", "fakten", "input", "belege"],
    "neu"         => &["neuartig", "modern", "aktuell", "innovativ", "frisch"],
    "groß"        => &["umfangreich", "ausgedehnt", "erheblich", "enorm"],
    "klein"       => &["gering", "minimal", "kompakt", "knapp", "winzig"],
    "schnell"     => &["rasch", "flink", "zügig", "prompt", "eilig"],
    "einfach"     => &["schlicht", "unkompliziert", "simpel", "direkt", "klar"],
    "komplex"     => &["vielschichtig", "anspruchsvoll", "aufwendig", "tiefgründig"],
    "immer"       => &["stets", "ständig", "beständig", "unaufhörlich"],
    "niemals"     => &["nie", "keineswegs", "auf keinen Fall", "keinerlei"],
    "außerdem"    => &["zusätzlich", "ferner", "überdies", "darüber hinaus"],
    "weil"        => &["da", "denn", "zumal", "angesichts", "aufgrund"],
    "obwohl"      => &["wenngleich", "auch wenn", "trotzdem", "dennoch"],
    "zudem"       => &["darüber hinaus", "überdies", "ferner", "außerdem"],
};
