// Copyright 2026 Mahmoud Harmouch.
//
// Licensed under the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT\>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! # Spanish Curated Synonym Table
//!
//! Compile-time perfect-hash map for Spanish vocabulary.  Used by
//! [`super::SynonymBank`] when [`super::detect::LanguageHint::Spanish`] is active.

use phf::{Map, phf_map};

/// Curated Spanish synonym table.
///
/// Maps common Spanish words to semantically equivalent alternatives.
/// Applied when [`LanguageHint::Spanish`] (or detected Spanish) is selected.
///
/// Lookups are O(1) and require no heap allocation.
pub(super) static SPANISH_SYNONYMS: Map<&'static str, &'static [&'static str]> = phf_map! {
    "comenzar"    => &["iniciar", "empezar", "arrancar", "abrir", "lanzar"],
    "contiene"    => &["incluye", "alberga", "lleva", "abarca", "engloba"],
    "texto"       => &["contenido", "escrito", "prosa", "redacción", "material"],
    "formato"     => &["estructura", "disposición", "forma", "estilo", "esquema"],
    "palabra"     => &["término", "vocablo", "expresión", "voz", "lexema"],
    "oculto"      => &["invisible", "escondido", "latente", "encubierto"],
    "marca"       => &["señal", "indicador", "huella", "etiqueta", "rastro"],
    "detectar"    => &["identificar", "encontrar", "descubrir", "localizar"],
    "eliminar"    => &["suprimir", "borrar", "quitar", "erradicar", "remover"],
    "importante"  => &["significativo", "relevante", "esencial", "vital", "clave"],
    "permite"     => &["posibilita", "admite", "autoriza", "habilita", "deja"],
    "muestra"     => &["exhibe", "presenta", "indica", "revela", "despliega"],
    "genera"      => &["produce", "crea", "origina", "elabora", "construye"],
    "define"      => &["especifica", "establece", "determina", "fija", "delimita"],
    "procesa"     => &["maneja", "trata", "gestiona", "opera", "administra"],
    "sistema"     => &["plataforma", "arquitectura", "mecanismo", "marco"],
    "proceso"     => &["procedimiento", "método", "operación", "flujo"],
    "resultado"   => &["salida", "producto", "fruto", "rendimiento", "consecuencia"],
    "análisis"    => &["examen", "estudio", "evaluación", "revisión", "inspección"],
    "datos"       => &["información", "registros", "métricas", "input", "evidencia"],
    "nuevo"       => &["novel", "moderno", "reciente", "fresco", "inédito"],
    "grande"      => &["amplio", "extenso", "vasto", "considerable", "enorme"],
    "pequeño"     => &["diminuto", "reducido", "breve", "compacto", "mínimo"],
    "rápido"      => &["veloz", "ágil", "ligero", "expedito", "presto"],
    "lento"       => &["pausado", "gradual", "calmo", "deliberado", "tranquilo"],
    "simple"      => &["sencillo", "básico", "elemental", "llano", "directo"],
    "complejo"    => &["intrincado", "elaborado", "sofisticado", "profundo"],
    "específico"  => &["particular", "preciso", "concreto", "determinado", "puntual"],
    "general"     => &["amplio", "global", "total", "universal", "común"],
    "siempre"     => &["invariablemente", "constantemente", "perpetuamente"],
    "nunca"       => &["jamás", "en ningún momento", "de ningún modo"],
    "también"     => &["además", "asimismo", "igualmente", "incluso", "del mismo modo"],
    "porque"      => &["ya que", "dado que", "puesto que", "pues", "en vista de"],
    "aunque"      => &["si bien", "a pesar de", "aun cuando", "pese a"],
    "además"      => &["asimismo", "igualmente", "también", "por otra parte"],
};
