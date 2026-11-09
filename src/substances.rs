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
    pub cas_numbers: Vec<String>,
    pub source: String,
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
    formula_key: String,
    cas_keys: Vec<String>,
    sort_key: String,
    sort_key_en: String,
}
impl Substance {
    pub fn external(
        id: String,
        record: UserRecord,
        source: String,
        cas_numbers: Vec<String>,
    ) -> Self {
        let mut entry = Self {
            id,
            cas: record.cas,
            cas_numbers,
            source,
            name_en: record.name_en,
            fallback_name: record.other_name,
            formula: record.formula,
            molar_mass: record.molar_mass,
            synonyms_en: record.synonyms_en,
            locale: record.locale,
            local_name: record.local_name,
            local_synonyms: record.local_synonyms,
            user_id: record.id,
            user_revision: record.id.map(|_| record.revision),
            keys: vec![],
            formula_key: String::new(),
            cas_keys: vec![],
            sort_key: String::new(),
            sort_key_en: String::new(),
        };
        entry.index(&[]);
        entry
    }
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
        self.formula_key = normalize(&self.formula);
        self.cas_keys = self.cas_numbers.iter().map(|s| normalize(s)).collect();
        if !self.cas.is_empty() {
            self.cas_keys.push(normalize(&self.cas));
        }
        self.sort_key_en = if self.name_en.is_empty() {
            &self.fallback_name
        } else {
            &self.name_en
        }
        .to_lowercase();
        self.sort_key = if self.local_name.is_empty() {
            self.sort_key_en.clone()
        } else {
            self.local_name.to_lowercase()
        };
        self.keys = [
            self.formula.as_str(),
            self.cas.as_str(),
            self.name_en.as_str(),
            self.local_name.as_str(),
            self.fallback_name.as_str(),
            self.source.as_str(),
        ]
        .into_iter()
        .chain(self.synonyms_en.split(';'))
        .chain(self.local_synonyms.split(';'))
        .chain(additional.iter().map(String::as_str))
        .chain(self.cas_numbers.iter().map(String::as_str))
        .filter(|s| !s.is_empty())
        .map(normalize)
        .collect();
        self.keys.sort_unstable();
        self.keys.dedup();
    }
}
#[derive(Clone, Debug)]
pub struct Catalog {
    pub entries: Vec<Substance>,
}
pub struct SearchResults {
    pub hits: Vec<usize>,
    pub total: usize,
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
                    cas_numbers: vec![r.get(1)?],
                    source: "Solcalc".into(),
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
                    formula_key: String::new(),
                    cas_keys: vec![],
                    sort_key: String::new(),
                    sort_key_en: String::new(),
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| e.to_string())?;
        let chebi = embedded(
            include_bytes!("../data/bundled/chebi.en.sqlite"),
            0x534F4C43,
        )?;
        let mut extra = chebi.prepare("SELECT id,cas,formula,name,molar_mass,synonyms,cas_numbers FROM substances ORDER BY id").map_err(|e|e.to_string())?;
        let imported = extra
            .query_map([], |r| {
                let id: String = r.get(0)?;
                let numbers: String = r.get(6)?;
                Ok(Substance::external(
                    format!("bundled:{id}"),
                    UserRecord {
                        cas: r.get(1)?,
                        formula: r.get(2)?,
                        name_en: r.get(3)?,
                        molar_mass: r.get(4)?,
                        synonyms_en: r.get(5)?,
                        locale: locale.into(),
                        ..Default::default()
                    },
                    id.to_uppercase(),
                    numbers
                        .split(';')
                        .filter(|s| !s.is_empty())
                        .map(str::to_owned)
                        .collect(),
                ))
            })
            .map_err(|e| e.to_string())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| e.to_string())?;
        entries.extend(imported);
        let aliases = embedded(
            include_bytes!("../data/bundled/chebi.aliases.sqlite"),
            0x534F4C43,
        )?;
        let mut alias_query = aliases
            .prepare("SELECT substance_id,source_id,synonyms,cas_numbers FROM aliases")
            .map_err(|e| e.to_string())?;
        let aliases = alias_query
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        for row in aliases {
            let (id, source, synonyms, numbers) = row.map_err(|e| e.to_string())?;
            if let Some(entry) = entries.iter_mut().find(|e| e.id == format!("bundled:{id}")) {
                entry.synonyms_en =
                    join_aliases(&entry.synonyms_en, &format!("{source};{synonyms}"));
                for number in numbers.split(';').filter(|s| !s.is_empty()) {
                    if !entry.cas_numbers.iter().any(|s| s == number) {
                        entry.cas_numbers.push(number.into());
                    }
                }
            }
        }
        for locales in [bundled::LOCALES, bundled::CHEBI_LOCALES] {
            if let Some((_, bytes)) = locales.iter().find(|(code, _)| *code == locale) {
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
                        if entry.local_name.is_empty() {
                            entry.local_name = name.clone();
                        } else {
                            entry.local_synonyms = join_aliases(&entry.local_synonyms, name);
                        }
                        entry.local_synonyms = join_aliases(&entry.local_synonyms, synonyms);
                        entry.keys.clear();
                    }
                }
            }
        }
        for entry in &mut entries {
            if entry.keys.is_empty() {
                entry.index(&[]);
            }
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
        // Imported cross-references can be ambiguous. Never silently attach a
        // user customization to the first of several different ChEBI entities.
        let mut by_cas: HashMap<String, Option<usize>> = HashMap::new();
        for (i, entry) in self.entries.iter().enumerate() {
            for cas in entry
                .cas_numbers
                .iter()
                .chain(std::iter::once(&entry.cas))
                .filter(|s| !s.is_empty())
            {
                by_cas
                    .entry(cas.clone())
                    .and_modify(|prior| {
                        if *prior != Some(i) {
                            *prior = None;
                        }
                    })
                    .or_insert(Some(i));
            }
        }
        // Two user records may use different CAS aliases of the same bundled
        // entity. Keep both independently editable instead of letting one hide
        // the other behind a shared bundled row.
        let mut target_counts: HashMap<usize, usize> = HashMap::new();
        for user in &records {
            if user.id.is_some() {
                if let Some(index) = by_cas.get(&user.cas).copied().flatten() {
                    *target_counts.entry(index).or_default() += 1;
                }
            }
        }
        for user in records {
            let Some(user_id) = user.id else { continue };
            if let Some(index) = by_cas
                .get(&user.cas)
                .copied()
                .flatten()
                .filter(|index| target_counts.get(index) == Some(&1))
            {
                let base = &mut self.entries[index];
                // CAS identifies the same chemical form. A user customization
                // wins for editable values; bundled names remain searchable.
                let original = vec![base.name_en.clone(), base.local_name.clone()];
                base.cas = user.cas.clone();
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
                    cas_numbers: vec![],
                    source: "User".into(),
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
                    formula_key: String::new(),
                    cas_keys: vec![],
                    sort_key: String::new(),
                    sort_key_en: String::new(),
                };
                entry.index(&[]);
                self.entries.push(entry);
            }
        }
    }
    pub fn find(&self, id: &str) -> Option<&Substance> {
        self.entries.iter().find(|e| e.id == id)
    }
    #[cfg(test)]
    pub fn search(&self, query: &str, language: Language) -> Vec<usize> {
        self.search_limited(query, language, usize::MAX).hits
    }
    pub fn search_limited(&self, query: &str, language: Language, limit: usize) -> SearchResults {
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
                let rank = if full.is_empty() || e.formula_key == full || e.cas_keys.contains(&full)
                {
                    0
                } else if e.keys.binary_search(&full).is_ok() {
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
        let total = hits.len();
        let key = |(rank, i): &(u8, usize)| {
            let entry = &self.entries[*i];
            let priority = if entry.user_id.is_some() {
                0
            } else if entry.source == "Solcalc" {
                1
            } else {
                2
            };
            (
                *rank,
                priority,
                if entry.locale == language.code() {
                    &entry.sort_key
                } else {
                    &entry.sort_key_en
                },
                &entry.id,
            )
        };
        let compare = |a: &(u8, usize), b: &(u8, usize)| key(a).cmp(&key(b));
        if limit < hits.len() {
            hits.select_nth_unstable_by(limit, compare);
            hits.truncate(limit);
        }
        hits.sort_unstable_by(compare);
        SearchResults {
            hits: hits.into_iter().map(|(_, i)| i).collect(),
            total,
        }
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
    fn chebi_search_preserves_every_curated_identity_and_limits_large_lists() {
        let catalog = Catalog::bundled("ru").unwrap();
        let core = embedded(include_bytes!("../data/bundled/main.en.sqlite"), 0x534F4C43).unwrap();
        let mut query = core
            .prepare("SELECT id,cas,formula,name,molar_mass FROM substances")
            .unwrap();
        let rows = query
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, f64>(4)?,
                ))
            })
            .unwrap();
        for row in rows {
            let (id, cas, formula, name, mass) = row.unwrap();
            let entry = catalog.find(&format!("bundled:{id}")).unwrap();
            assert_eq!(
                (&entry.cas, &entry.formula, &entry.name_en, entry.molar_mass),
                (&cas, &formula, &name, mass)
            );
            assert!(!entry.local_name.is_empty());
        }
        let caffeine = first(&catalog, "CHEBI:27732");
        assert_eq!(caffeine.cas, "58-08-2");
        assert_eq!(caffeine.formula, "C8H10N4O2");
        assert_eq!(caffeine.molar_mass, 194.194);
        let all = catalog.search_limited("", Language::Ru, 25);
        assert_eq!(all.total, catalog.entries.len());
        assert_eq!(all.hits.len(), 25);
        assert_eq!(all.hits, catalog.search("", Language::Ru)[..25]);
        assert_eq!(catalog.search_limited("", Language::Ru, 0).hits.len(), 0);
    }
    #[test]
    fn ambiguous_cross_references_do_not_overwrite_user_records_or_choose_an_arbitrary_entity() {
        let make = |id: &str, numbers: Vec<String>| {
            Substance::external(
                id.into(),
                UserRecord {
                    name_en: id.into(),
                    cas: numbers[0].clone(),
                    molar_mass: 100.0,
                    locale: "en".into(),
                    ..Default::default()
                },
                "ChEBI".into(),
                numbers,
            )
        };
        let mut catalog = Catalog {
            entries: vec![
                make("bundled:chebi:a", vec!["50-99-7".into(), "492-62-6".into()]),
                make("bundled:chebi:b", vec!["50-99-7".into()]),
            ],
        };
        catalog.merge(vec![UserRecord {
            id: Some(42),
            cas: "50-99-7".into(),
            name_en: "My exact material".into(),
            molar_mass: 123.456,
            locale: "en".into(),
            ..Default::default()
        }]);
        assert_eq!(catalog.entries.len(), 3);
        assert_eq!(catalog.find("user:42").unwrap().molar_mass, 123.456);
        assert!(catalog.entries[..2].iter().all(|e| e.user_id.is_none()));
        assert_eq!(
            catalog.search_limited("50-99-7", Language::En, 10).hits[0],
            2
        );
        let mut catalog = Catalog {
            entries: vec![make(
                "bundled:chebi:a",
                vec!["50-99-7".into(), "492-62-6".into()],
            )],
        };
        catalog.merge(vec![UserRecord {
            id: Some(7),
            cas: "492-62-6".into(),
            name_en: "My alias material".into(),
            molar_mass: 180.2,
            locale: "en".into(),
            ..Default::default()
        }]);
        assert_eq!(catalog.entries.len(), 1);
        assert_eq!(catalog.entries[0].user_id, Some(7));
        assert_eq!(catalog.entries[0].cas, "492-62-6");
        assert!(!catalog.search("50-99-7", Language::En).is_empty());
    }
    #[test]
    fn different_user_cas_aliases_of_one_entity_remain_independently_editable() {
        let mut catalog = Catalog {
            entries: vec![Substance::external(
                "bundled:chebi:example".into(),
                UserRecord {
                    cas: "50-99-7".into(),
                    name_en: "Bundled material".into(),
                    molar_mass: 180.156,
                    locale: "ru".into(),
                    ..Default::default()
                },
                "CHEBI:example".into(),
                vec!["50-99-7".into(), "492-62-6".into()],
            )],
        };
        catalog.merge(vec![
            UserRecord {
                id: Some(1),
                cas: "50-99-7".into(),
                name_en: "First user material".into(),
                molar_mass: 180.2,
                locale: "ru".into(),
                local_name: "Первое вещество".into(),
                ..Default::default()
            },
            UserRecord {
                id: Some(2),
                cas: "492-62-6".into(),
                name_en: "Second user material".into(),
                molar_mass: 181.0,
                locale: "ru".into(),
                local_name: "Второе вещество".into(),
                ..Default::default()
            },
        ]);
        assert_eq!(catalog.entries.len(), 3);
        assert!(catalog.entries[0].user_id.is_none());
        let first = catalog.find("user:1").unwrap();
        let second = catalog.find("user:2").unwrap();
        assert_eq!(first.molar_mass, 180.2);
        assert_eq!(first.name(Language::Ru), "Первое вещество");
        assert_eq!(second.molar_mass, 181.0);
        assert_eq!(second.name(Language::Ru), "Второе вещество");
        for query in ["Первое вещество", "Second user material"] {
            assert_eq!(catalog.search(query, Language::Ru).len(), 1);
        }
    }
    #[test]
    fn bundled_integrity_and_locale_separation() {
        let en = Catalog::bundled("en").unwrap();
        let ru = Catalog::bundled("ru").unwrap();
        let unknown = Catalog::bundled("zz").unwrap();
        assert_eq!(en.entries.len(), 104099);
        assert_eq!(ru.entries.len(), en.entries.len());
        assert_eq!(
            en.entries.iter().filter(|e| e.source == "Solcalc").count(),
            131
        );
        let mut cas = std::collections::HashSet::new();
        for e in &en.entries {
            if e.source == "Solcalc" {
                assert!(user_db::valid_cas(&e.cas));
                assert!(cas.insert(&e.cas));
            }
            assert!(e.cas_numbers.iter().all(|n| user_db::valid_cas(n)));
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
        assert!(unknown
            .entries
            .iter()
            .filter(|e| e.source == "Solcalc")
            .all(|e| e.local_name.is_empty()));
        assert!(unknown.entries.iter().all(|e| e.local_name.is_empty()));
        let masses: HashMap<_, _> = en.entries.iter().map(|e| (&e.id, e.molar_mass)).collect();
        for e in &ru.entries {
            assert_eq!(e.molar_mass, masses[&e.id]);
            if e.source == "Solcalc" {
                assert!(!e.local_name.is_empty());
            }
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
        assert_eq!(c.search("", Language::En).len(), c.entries.len());
        assert!(c.search("C6H12O6", Language::En).len() >= 4);
    }
    #[test]
    fn user_and_bundled_merge_by_cas_without_duplicate_or_erasing_aliases() {
        let mut c = Catalog::bundled("ru").unwrap();
        let before = c.entries.len();
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
        assert_eq!(c.entries.len(), before);
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
        let before = c.entries.len();
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
        assert_eq!(c.entries.len(), before + 2);
        assert!(c.find("user:1").is_some());
        assert!(c.find("user:2").is_some());
        assert!(c.search("C6H12O6", Language::En).len() >= 6);
    }
    #[test]
    fn partial_locale_falls_back_to_english_and_user_mass_is_used_in_calculation() {
        let mut c = Catalog::bundled("de").unwrap();
        assert!(c.entries.iter().any(|e| !e.local_name.is_empty()));
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
