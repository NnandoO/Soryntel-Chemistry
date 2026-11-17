//! Puente para vistas históricas que requieren etiquetas con vida estática.
//! El catálogo se compila con rust-i18n; las vistas nuevas deben usar t!().
use std::{collections::HashMap, sync::OnceLock};

pub fn text(key: &'static str) -> &'static str {
    static TEXT: OnceLock<HashMap<String, String>> = OnceLock::new();
    TEXT.get_or_init(|| {
        let catalog: HashMap<String, serde_json::Value> =
            serde_json::from_str(include_str!("../locales/es.json"))
                .expect("catálogo español válido");
        catalog
            .into_keys()
            .filter(|k| k != "_version")
            .map(|key| {
                let value = rust_i18n::t!(&key, locale = "es").into_owned();
                (key, value)
            })
            .collect()
    })
    .get(key)
    .map(String::as_str)
    .unwrap_or(key)
}
