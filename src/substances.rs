//! English chemical database + the active locale + independently stored user data.
use crate::settings::Language;
use crate::user_db::{self, UserRecord, UserStore};
use rusqlite::Connection;
use std::{collections::HashMap, path::Path};
mod bundled {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/data/bundled/locale_index.rs"
    ));
}

#[derive(Clone, Debug)]
pub struct Substance {
    pub id: String,
    pub cas: String,
    pub name_en: String,
    pub fallback_name: String,
    pub formula: String,
    pub molar_mass: f64,
    pub synonyms_en: String,
    pub locale: String,
    pub local_name: String,
    pub local_synonyms: String,
    pub user_id: Option<i64>,
    pub user_revision: Option<i64>,
    keys: Vec<String>,
}
impl Substance {
    pub fn name(&self, language: Language) -> &str {
        if self.locale == language.code() && !self.local_name.is_empty() {
            &self.local_name
        } else if !self.name_en.is_empty() {
            &self.name_en
        } else {
            &self.fallback_name
        }
    }
    pub fn synonyms(&self) -> String {
        join_aliases(&self.synonyms_en, &self.local_synonyms)
    }
    pub fn user_record(&self, locale: &str) -> UserRecord {
        UserRecord {
            id: None,
            cas: self.cas.clone(),
            formula: self.formula.clone(),
            molar_mass: self.molar_mass,
            name_en: self.name_en.clone(),
            synonyms_en: self.synonyms_en.clone(),
            locale: locale.into(),
            local_name: if self.locale == locale {
                self.local_name.clone()
            } else {
                String::new()
            },
            local_synonyms: if self.locale == locale {
                self.local_synonyms.clone()
            } else {
                String::new()
            },
            ..Default::default()
        }
    }
    pub fn formula_display(&self) -> String {
        let mut coefficient = true;
        self.formula
            .chars()
            .map(|c| {
                if matches!(c, '·' | '.' | '*') {
                    coefficient = true;
                } else if !c.is_ascii_digit() {
                    coefficient = false;
                }
                if c.is_ascii_digit() && !coefficient {
                    "₀₁₂₃₄₅₆₇₈₉".chars().nth(c as usize - '0' as usize).unwrap()
                } else {
                    c
                }
            })
            .collect()
    }
    fn index(&mut self, additional: &[String]) {
        self.keys = [
            self.formula.as_str(),
            self.cas.as_str(),
            self.name_en.as_str(),
            self.local_name.as_str(),
            self.fallback_name.as_str(),
        ]
        .into_iter()
        .chain(self.synonyms_en.split(';'))
        .chain(self.local_synonyms.split(';'))
        .chain(additional.iter().map(String::as_str))
        .filter(|s| !s.is_empty())
        .map(normalize)
        .collect();
    }
}
#[derive(Clone, Debug)]
pub struct Catalog {
    pub entries: Vec<Substance>,
}
impl Catalog {
    pub fn bundled(locale: &str) -> user_db::Result<Self> {
        let core = embedded(include_bytes!("../data/bundled/main.en.sqlite"), 0x534F4C43)?;
        let mut stmt = core
            .prepare("SELECT id,cas,formula,name,molar_mass,synonyms FROM substances ORDER BY id")
            .map_err(|e| e.to_string())?;
        let mut entries = stmt
            .query_map([], |r| {
                Ok(Substance {
                    id: format!("bundled:{}", r.get::<_, String>(0)?),
                    cas: r.get(1)?,
                    formula: r.get(2)?,
                    name_en: r.get(3)?,
                    fallback_name: String::new(),
                    molar_mass: r.get(4)?,
                    synonyms_en: r.get(5)?,
                    locale: locale.into(),
                    local_name: String::new(),
                    local_synonyms: String::new(),
                    user_id: None,
                    user_revision: None,
                    keys: vec![],
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| e.to_string())?;
        if let Some((_, bytes)) = bundled::LOCALES.iter().find(|(code, _)| *code == locale) {
            let localized = embedded(bytes, 0x534F4C4C)?;
            let stored: String = localized
                .query_row("SELECT value FROM metadata WHERE key='locale'", [], |r| {
                    r.get(0)
                })
                .map_err(|e| e.to_string())?;
            if stored != locale {
                return Err("Bundled locale database has the wrong language.".into());
            }
            let mut stmt = localized
                .prepare("SELECT substance_id,name,synonyms FROM translations")
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map([], |r| {
                    Ok((
                        format!("bundled:{}", r.get::<_, String>(0)?),
                        (r.get::<_, String>(1)?, r.get::<_, String>(2)?),
                    ))
                })
                .map_err(|e| e.to_string())?;
            let translations = rows
                .collect::<rusqlite::Result<HashMap<_, _>>>()
                .map_err(|e| e.to_string())?;
            for entry in &mut entries {
                if let Some((name, synonyms)) = translations.get(&entry.id) {
                    entry.local_name = name.clone();
                    entry.local_synonyms = synonyms.clone();
                }
            }
        }
        for entry in &mut entries {
            entry.index(&[]);
        }
        Ok(Self { entries })
    }
    pub fn load(locale: &str, path: &Path) -> user_db::Result<Self> {
        let mut catalog = Self::bundled(locale)?;
        let user = UserStore::open(path)?;
        catalog.merge(user.records(locale)?);
        Ok(catalog)
    }
    pub fn merge(&mut self, records: Vec<UserRecord>) {
        for user in records {
            let Some(user_id) = user.id else { continue };
            if let Some(base) = self
                .entries
                .iter_mut()
                .find(|e| !user.cas.is_empty() && e.cas == user.cas)
            {
                // CAS identifies the same chemical form. A user customization
                // wins for editable values; bundled names remain searchable.
                let original = vec![base.name_en.clone(), base.local_name.clone()];
                if !user.name_en.is_empty() {
                    base.name_en = user.name_en;
                }
                if !user.formula.is_empty() {
                    base.formula = user.formula;
                }
                base.molar_mass = user.molar_mass;
                base.synonyms_en = join_aliases(&base.synonyms_en, &user.synonyms_en);
                if !user.local_name.is_empty() {
                    base.local_name = user.local_name;
                }
                base.local_synonyms = join_aliases(&base.local_synonyms, &user.local_synonyms);
                base.user_id = Some(user_id);
                base.user_revision = Some(user.revision);
                base.index(&original);
            } else {
                let fallback_name = if !user.local_name.is_empty() {
                    user.local_name.clone()
                } else {
                    user.other_name.clone()
                };
                let mut entry = Substance {
                    id: format!("user:{user_id}"),
                    cas: user.cas,
                    name_en: user.name_en,
                    fallback_name,
                    formula: user.formula,
                    molar_mass: user.molar_mass,
                    synonyms_en: user.synonyms_en,
                    locale: user.locale,
                    local_name: user.local_name,
                    local_synonyms: user.local_synonyms,
                    user_id: Some(user_id),
                    user_revision: Some(user.revision),
                    keys: vec![],
                };
                entry.index(&[]);
                self.entries.push(entry);
            }
        }
    }
    pub fn find(&self, id: &str) -> Option<&Substance> {
        self.entries.iter().find(|e| e.id == id)
    }
    pub fn search(&self, query: &str, language: Language) -> Vec<usize> {
        let full = normalize(query);
        let terms: Vec<_> = query
            .split_whitespace()
            .map(normalize)
            .filter(|s| !s.is_empty())
            .collect();
        let mut hits: Vec<_> = self
            .entries
            .iter()
            .enumerate()
            .filter_map(|(i, e)| {
                let rank = if full.is_empty()
                    || normalize(&e.formula) == full
                    || (!e.cas.is_empty() && normalize(&e.cas) == full)
                {
                    0
                } else if e.keys.contains(&full) {
                    1
                } else if e.keys.iter().any(|s| s.starts_with(&full)) {
                    2
                } else if e.keys.iter().any(|s| s.contains(&full))
                    || (!terms.is_empty()
                        && terms.iter().all(|t| e.keys.iter().any(|s| s.contains(t))))
                {
                    3
                } else {
                    return None;
                };
                Some((rank, i))
            })
            .collect();
        hits.sort_by_cached_key(|(rank, i)| {
            (
                *rank,
                self.entries[*i].name(language).to_lowercase(),
                self.entries[*i].id.clone(),
            )
        });
        hits.into_iter().map(|(_, i)| i).collect()
    }
}
fn embedded(bytes: &'static [u8], application: i64) -> user_db::Result<Connection> {
    let mut conn = Connection::open_in_memory().map_err(|e| e.to_string())?;
    conn.deserialize_bytes(rusqlite::MAIN_DB, bytes)
        .map_err(|e| e.to_string())?;
    let app: i64 = conn
        .pragma_query_value(None, "application_id", |r| r.get(0))
        .map_err(|e| e.to_string())?;
    let schema: i64 = conn
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .map_err(|e| e.to_string())?;
    if app != application || schema != 1 {
        return Err("Unsupported bundled database format.".into());
    }
    Ok(conn)
}
fn join_aliases(a: &str, b: &str) -> String {
    let mut result = vec![];
    for text in a
        .split(';')
        .chain(b.split(';'))
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        if !result.contains(&text) {
            result.push(text);
        }
    }
    result.join(";")
}
fn normalize(value: &str) -> String {
    value
        .chars()
        .flat_map(char::to_lowercase)
        .filter_map(|c| match c {
            'ё' => Some('е'),
            '₀'..='₉' => char::from_u32('0' as u32 + c as u32 - '₀' as u32),
            _ if c.is_alphanumeric() => Some(c),
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn first<'a>(catalog: &'a Catalog, q: &str) -> &'a Substance {
        &catalog.entries[catalog.search(q, Language::Ru)[0]]
    }
    #[test]
    fn bundled_integrity_and_locale_separation() {
        let en = Catalog::bundled("en").unwrap();
        let ru = Catalog::bundled("ru").unwrap();
        let unknown = Catalog::bundled("de").unwrap();
        assert_eq!(en.entries.len(), 131);
        assert_eq!(ru.entries.len(), 131);
        let mut cas = std::collections::HashSet::new();
        for e in &en.entries {
            assert!(user_db::valid_cas(&e.cas));
            assert!(cas.insert(&e.cas));
            assert!(!e.name_en.is_empty() && !e.formula.is_empty());
            assert!(e.molar_mass.is_finite() && e.molar_mass > 0.0);
            assert!(e.local_name.is_empty());
        }
        assert!(en.search("медный купорос", Language::En).is_empty());
        assert_eq!(first(&ru, "медный купорос").cas, "7758-99-8");
        assert_eq!(
            first(&unknown, "Epsom salt").name(Language::En),
            "Magnesium sulfate heptahydrate"
        );
        assert!(unknown.entries.iter().all(|e| e.local_name.is_empty()));
        for e in &ru.entries {
            assert_eq!(e.molar_mass, en.find(&e.id).unwrap().molar_mass);
            assert!(!e.local_name.is_empty());
        }
    }
    #[test]
    fn search_names_synonyms_cas_formulas_and_hydrates() {
        let c = Catalog::bundled("ru").unwrap();
        for (q, cas) in [
            ("Epsom salt", "10034-99-8"),
            ("трилон Б", "6381-92-6"),
            ("7647-14-5", "7647-14-5"),
            ("7647145", "7647-14-5"),
            ("CaCl2.2H2O", "10035-04-8"),
            ("mgso₄ * 7h₂o", "10034-99-8"),
            ("sodium dibasic dodeca", "10039-32-4"),
        ] {
            assert_eq!(first(&c, q).cas, cas, "{q}");
        }
        assert_eq!(first(&c, "MgSO4").molar_mass, 120.361);
        assert_eq!(first(&c, "MgSO4.7H2O").formula_display(), "MgSO₄·7H₂O");
        assert_eq!(
            c.search("трёхводный", Language::Ru),
            c.search("трехводный", Language::Ru)
        );
        assert!(c.search("zzzzxyz", Language::Ru).is_empty());
        assert_eq!(c.search("", Language::En).len(), 131);
        assert!(c.search("C6H12O6", Language::En).len() >= 4);
    }
    #[test]
    fn user_and_bundled_merge_by_cas_without_duplicate_or_erasing_aliases() {
        let mut c = Catalog::bundled("ru").unwrap();
        let id = first(&c, "table salt").id.clone();
        c.merge(vec![UserRecord {
            id: Some(7),
            cas: "7647-14-5".into(),
            name_en: "My salt".into(),
            formula: "NaCl".into(),
            molar_mass: 58.44,
            synonyms_en: "bench salt".into(),
            locale: "ru".into(),
            local_name: "Моя соль".into(),
            local_synonyms: "рабочая соль".into(),
            ..Default::default()
        }]);
        assert_eq!(c.entries.len(), 131);
        for q in [
            "Sodium chloride",
            "table salt",
            "Хлорид натрия",
            "рабочая соль",
            "bench salt",
        ] {
            let e = first(&c, q);
            assert_eq!(e.id, id);
            assert_eq!(e.name(Language::Ru), "Моя соль");
            assert_eq!(e.user_id, Some(7));
        }
    }
    #[test]
    fn user_records_without_cas_remain_distinct_and_bundled_updates_do_not_replace_them() {
        let mut c = Catalog::bundled("en").unwrap();
        c.merge(vec![
            UserRecord {
                id: Some(1),
                name_en: "My reagent".into(),
                formula: "C6H12O6".into(),
                molar_mass: 180.2,
                locale: "en".into(),
                ..Default::default()
            },
            UserRecord {
                id: Some(2),
                name_en: "My other reagent".into(),
                formula: "C6H12O6".into(),
                molar_mass: 180.2,
                locale: "en".into(),
                ..Default::default()
            },
        ]);
        assert_eq!(c.entries.len(), 133);
        assert!(c.find("user:1").is_some());
        assert!(c.find("user:2").is_some());
        assert!(c.search("C6H12O6", Language::En).len() >= 6);
    }
    #[test]
    fn partial_locale_falls_back_to_english_and_user_mass_is_used_in_calculation() {
        let mut c = Catalog::bundled("de").unwrap();
        let mut r = first(&c, "NaCl").user_record("de");
        r.id = Some(9);
        r.name_en = "User salt".into();
        r.molar_mass = 58.45;
        c.merge(vec![r]);
        let salt = first(&c, "NaCl");
        assert_eq!(salt.name(Language::En), "User salt");
        let result = crate::calc::solve_solution(
            crate::calc::Basis::Molar,
            Some(salt.molar_mass),
            Some(0.1),
            Some(0.1),
            None,
        )
        .unwrap();
        match result {
            Some(crate::calc::MissingQuantity::Mass(v)) => assert!((v - 0.5845).abs() < 1e-10),
            _ => panic!("missing mass"),
        }
    }
}
