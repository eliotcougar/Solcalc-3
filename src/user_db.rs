//! User-owned storage. Bundled data is never copied into this database.
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub type Result<T> = std::result::Result<T, String>;
pub const SCHEMA_VERSION: i64 = 2;
const APP_ID: i64 = 0x534F4C55;
const V1: &str = "CREATE TABLE user_substances (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    cas TEXT NOT NULL DEFAULT '', formula TEXT NOT NULL,
    molar_mass REAL NOT NULL CHECK(molar_mass>0),
    name_en TEXT NOT NULL, synonyms_en TEXT NOT NULL DEFAULT '');
    CREATE UNIQUE INDEX user_cas ON user_substances(cas) WHERE cas<>'';";
const V2: &str = "ALTER TABLE user_substances ADD COLUMN revision INTEGER NOT NULL DEFAULT 1;
    CREATE TABLE user_localizations (
    substance_id INTEGER NOT NULL REFERENCES user_substances(id),
    locale TEXT NOT NULL, name TEXT NOT NULL DEFAULT '', synonyms TEXT NOT NULL DEFAULT '',
    PRIMARY KEY(substance_id,locale));";

#[derive(Clone, Debug, Default)]
pub struct UserRecord {
    pub id: Option<i64>,
    pub revision: i64,
    pub cas: String,
    pub formula: String,
    pub molar_mass: f64,
    pub name_en: String,
    pub synonyms_en: String,
    pub locale: String,
    pub local_name: String,
    pub local_synonyms: String,
    // A name in a locale not currently being edited, used only as fallback.
    pub other_name: String,
}
pub fn path() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA").map(|root| {
        PathBuf::from(root)
            .join("Solcalc")
            .join("user-substances.sqlite")
    })
}
pub fn valid_cas(value: &str) -> bool {
    let parts: Vec<_> = value.split('-').collect();
    if parts.len() != 3
        || !(2..=7).contains(&parts[0].len())
        || parts[0].starts_with('0')
        || parts[1].len() != 2
        || parts[2].len() != 1
        || !parts.iter().all(|s| s.bytes().all(|b| b.is_ascii_digit()))
    {
        return false;
    }
    let digits: Vec<_> = parts
        .concat()
        .bytes()
        .map(|b| (b - b'0') as usize)
        .collect();
    digits[..digits.len() - 1]
        .iter()
        .rev()
        .enumerate()
        .map(|(i, d)| (i + 1) * d)
        .sum::<usize>()
        % 10
        == digits[digits.len() - 1]
}
pub fn validate(record: &UserRecord) -> Result<()> {
    if record.name_en.trim().is_empty()
        && record.local_name.trim().is_empty()
        && record.other_name.trim().is_empty()
    {
        return Err("Enter at least one name.".into());
    }
    if !record.molar_mass.is_finite() || record.molar_mass <= 0.0 {
        return Err("Molar mass must be greater than zero and within range.".into());
    }
    if !record.cas.trim().is_empty() && !valid_cas(record.cas.trim()) {
        return Err("Enter a valid CAS number, for example 7647-14-5, or leave it blank.".into());
    }
    if !valid_locale(&record.locale) {
        return Err("Invalid locale code.".into());
    }
    for (text, max) in [
        (&record.name_en, 200),
        (&record.local_name, 200),
        (&record.formula, 200),
        (&record.synonyms_en, 2000),
        (&record.local_synonyms, 2000),
    ] {
        if text.chars().count() > max || text.chars().any(char::is_control) {
            return Err(
                "A name, formula or synonym is too long or contains control characters.".into(),
            );
        }
    }
    Ok(())
}
fn valid_locale(code: &str) -> bool {
    let parts: Vec<_> = code.split('-').collect();
    (2..=3).contains(&parts[0].len())
        && parts[0].bytes().all(|b| b.is_ascii_lowercase())
        && parts[1..].iter().all(|s| {
            (2..=8).contains(&s.len())
                && s.bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        })
}
pub struct UserStore {
    conn: Connection,
}
impl UserStore {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut conn = Connection::open(path).map_err(|e| e.to_string())?;
        conn.busy_timeout(Duration::from_millis(1500))
            .map_err(|e| e.to_string())?;
        conn.pragma_update(None, "foreign_keys", true)
            .map_err(|e| e.to_string())?;
        let version: i64 = conn
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .map_err(|e| e.to_string())?;
        let app: i64 = conn
            .pragma_query_value(None, "application_id", |r| r.get(0))
            .map_err(|e| e.to_string())?;
        if version > SCHEMA_VERSION {
            return Err(
                "The user database was created by a newer Solcalc version. It was not changed."
                    .into(),
            );
        }
        if app != 0 && app != APP_ID {
            return Err("This file is not a Solcalc user database. It was not changed.".into());
        }
        let tables:i64=conn.query_row("SELECT count(*) FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'",[],|r|r.get(0)).map_err(|e|e.to_string())?;
        if version == 0 && tables > 0 {
            return Err("Unrecognized user database structure. It was not changed.".into());
        }
        if version > 0 {
            // Validate the legacy shape before attempting additive migrations.
            conn.prepare(
                "SELECT id,cas,formula,molar_mass,name_en,synonyms_en FROM user_substances LIMIT 0",
            )
            .map_err(|e| e.to_string())?;
        }
        if version < SCHEMA_VERSION {
            if version > 0 {
                backup(&conn, path, version)?;
            }
            let tx = conn
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .map_err(|e| e.to_string())?;
            // Another process may have completed the migration while we waited.
            let current: i64 = tx
                .pragma_query_value(None, "user_version", |r| r.get(0))
                .map_err(|e| e.to_string())?;
            if current > SCHEMA_VERSION {
                return Err(
                    "The user database was created by a newer Solcalc version. It was not changed."
                        .into(),
                );
            }
            if current < 1 {
                tx.execute_batch(V1).map_err(|e| e.to_string())?;
            }
            if current < 2 {
                tx.execute_batch(V2).map_err(|e| e.to_string())?;
            }
            tx.pragma_update(None, "user_version", SCHEMA_VERSION)
                .map_err(|e| e.to_string())?;
            tx.pragma_update(None, "application_id", APP_ID)
                .map_err(|e| e.to_string())?;
            tx.commit().map_err(|e| e.to_string())?;
        }
        Ok(Self { conn })
    }
    pub fn records(&self, locale: &str) -> Result<Vec<UserRecord>> {
        let mut stmt=self.conn.prepare("SELECT s.id,s.revision,s.cas,s.formula,s.molar_mass,s.name_en,s.synonyms_en,
            coalesce(l.name,''),coalesce(l.synonyms,''),
            coalesce((SELECT name FROM user_localizations other WHERE other.substance_id=s.id AND other.locale<>?1 AND trim(other.name)<>'' ORDER BY other.locale LIMIT 1),'')
            FROM user_substances s LEFT JOIN user_localizations l
            ON l.substance_id=s.id AND l.locale=?1 ORDER BY s.id").map_err(|e|e.to_string())?;
        let rows = stmt
            .query_map([locale], |r| {
                Ok(UserRecord {
                    id: Some(r.get(0)?),
                    revision: r.get(1)?,
                    cas: r.get(2)?,
                    formula: r.get(3)?,
                    molar_mass: r.get(4)?,
                    name_en: r.get(5)?,
                    synonyms_en: r.get(6)?,
                    locale: locale.to_owned(),
                    local_name: r.get(7)?,
                    local_synonyms: r.get(8)?,
                    other_name: r.get(9)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.map(|r| r.map_err(|e| e.to_string())).collect()
    }
    pub fn get(&self, id: i64, locale: &str) -> Result<UserRecord> {
        self.records(locale)?
            .into_iter()
            .find(|r| r.id == Some(id))
            .ok_or_else(|| "This user entry no longer exists. Reopen the catalog.".into())
    }
    pub fn save(&mut self, record: &UserRecord) -> Result<i64> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| e.to_string())?;
        // Do not trust a cached fallback after another locale has been edited.
        let mut checked = record.clone();
        checked.other_name = if let Some(id) = record.id {
            tx.query_row("SELECT name FROM user_localizations WHERE substance_id=?1 AND locale<>?2 AND trim(name)<>'' ORDER BY locale LIMIT 1",params![id,record.locale],|r|r.get(0)).optional().map_err(|e|e.to_string())?.unwrap_or_default()
        } else {
            String::new()
        };
        validate(&checked)?;
        let cas = record.cas.trim();
        let other: Option<i64> = tx
            .query_row(
                "SELECT id FROM user_substances WHERE cas=?1 AND cas<>''",
                [cas],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        if other.is_some() && other != record.id {
            return Err(
                "A user entry with this CAS number already exists. Edit that entry instead.".into(),
            );
        }
        let values = params![
            cas,
            record.formula.trim(),
            record.molar_mass,
            record.name_en.trim(),
            record.synonyms_en.trim()
        ];
        let id = if let Some(id) = record.id {
            let changed=tx.execute("UPDATE user_substances SET cas=?1,formula=?2,molar_mass=?3,name_en=?4,synonyms_en=?5,revision=revision+1 WHERE id=?6 AND revision=?7",
                params![cas,record.formula.trim(),record.molar_mass,record.name_en.trim(),record.synonyms_en.trim(),id,record.revision]).map_err(|e|e.to_string())?;
            if changed != 1 {
                return Err(
                    "This entry changed in another window. Reopen it before saving.".into(),
                );
            }
            id
        } else {
            tx.execute("INSERT INTO user_substances(cas,formula,molar_mass,name_en,synonyms_en) VALUES(?1,?2,?3,?4,?5)",values).map_err(|e|e.to_string())?;
            tx.last_insert_rowid()
        };
        if record.locale != "en" {
            tx.execute("INSERT INTO user_localizations(substance_id,locale,name,synonyms) VALUES(?1,?2,?3,?4)
                ON CONFLICT(substance_id,locale) DO UPDATE SET name=excluded.name,synonyms=excluded.synonyms",
                params![id,record.locale,record.local_name.trim(),record.local_synonyms.trim()]).map_err(|e|e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())?;
        Ok(id)
    }
    pub fn delete(&mut self, id: i64, revision: i64) -> Result<()> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| e.to_string())?;
        let current: Option<i64> = tx
            .query_row(
                "SELECT revision FROM user_substances WHERE id=?1",
                [id],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        match current {
            None => return Err("This user entry no longer exists. Reopen the catalog.".into()),
            Some(value) if value != revision => {
                return Err(
                    "This entry changed in another window. Reopen it before deleting.".into(),
                )
            }
            _ => {}
        }
        // The parent and every localization disappear together, or neither
        // does. Unknown future foreign-key references can veto the deletion.
        tx.execute("DELETE FROM user_localizations WHERE substance_id=?1", [id])
            .map_err(|e| e.to_string())?;
        let removed = tx
            .execute(
                "DELETE FROM user_substances WHERE id=?1 AND revision=?2",
                params![id, revision],
            )
            .map_err(|e| e.to_string())?;
        if removed != 1 {
            return Err("This entry changed in another window. Reopen it before deleting.".into());
        }
        tx.commit().map_err(|e| e.to_string())
    }
}
fn backup(conn: &Connection, path: &Path, version: i64) -> Result<()> {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let backup = path.with_file_name(format!(
        "user-substances.pre-v{version}-{stamp}-{}.sqlite",
        std::process::id()
    ));
    // Never replace a previous backup. SQLite backup includes committed WAL data.
    let file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&backup)
        .map_err(|e| e.to_string())?;
    drop(file);
    conn.backup(
        rusqlite::MAIN_DB,
        &backup,
        None::<fn(rusqlite::backup::Progress)>,
    )
    .map_err(|e| format!("Could not back up the user database; migration was not run: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    fn folder() -> PathBuf {
        let p = std::env::current_dir().unwrap().join("qa").join(format!(
            "user-db-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&p).unwrap();
        p
    }
    fn record() -> UserRecord {
        UserRecord {
            name_en: "Test glucose".into(),
            synonyms_en: "test alias".into(),
            formula: "C6H12O6".into(),
            molar_mass: 180.156,
            cas: "50-99-7".into(),
            locale: "ru".into(),
            local_name: "Тестовая глюкоза".into(),
            local_synonyms: "тест".into(),
            ..Default::default()
        }
    }
    #[test]
    fn cas_checks_format_and_checksum() {
        for s in ["7647-14-5", "50-99-7", "14431-43-7"] {
            assert!(valid_cas(s));
        }
        for s in [
            "7647-14-6",
            "01-23-4",
            "1-23-4",
            "12345678-12-3",
            "AB-12-3",
            "",
        ] {
            assert!(!valid_cas(s));
        }
    }
    #[test]
    fn save_reopen_and_locales_survive_independent_edits() {
        let dir = folder();
        let path = dir.join("user.sqlite");
        let mut db = UserStore::open(&path).unwrap();
        let id = db.save(&record()).unwrap();
        drop(db);
        let mut db = UserStore::open(&path).unwrap();
        let mut en = db.get(id, "en").unwrap();
        en.name_en = "Changed English".into();
        db.save(&en).unwrap();
        let mut de = db.get(id, "de").unwrap();
        de.local_name = "Traubenzucker".into();
        db.save(&de).unwrap();
        assert_eq!(db.get(id, "ru").unwrap().local_name, "Тестовая глюкоза");
        assert_eq!(db.get(id, "de").unwrap().local_name, "Traubenzucker");
        assert_eq!(db.get(id, "ru").unwrap().name_en, "Changed English");
        drop(db);
        fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn duplicate_invalid_and_stale_writes_preserve_original() {
        let dir = folder();
        let path = dir.join("user.sqlite");
        let mut db = UserStore::open(&path).unwrap();
        let id = db.save(&record()).unwrap();
        assert!(db.save(&record()).unwrap_err().contains("CAS"));
        let mut invalid = record();
        invalid.molar_mass = f64::NAN;
        assert!(db.save(&invalid).is_err());
        let stale = db.get(id, "ru").unwrap();
        let mut latest = stale.clone();
        latest.name_en = "Latest".into();
        db.save(&latest).unwrap();
        assert!(db.save(&stale).unwrap_err().contains("another window"));
        assert_eq!(db.get(id, "ru").unwrap().name_en, "Latest");
        assert_eq!(db.records("ru").unwrap().len(), 1);
        drop(db);
        fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn additive_migration_preserves_rows_unknown_columns_and_backup() {
        let dir = folder();
        let path = dir.join("user.sqlite");
        let old = Connection::open(&path).unwrap();
        old.execute_batch(V1).unwrap();
        old.execute_batch("ALTER TABLE user_substances ADD COLUMN future_note TEXT; INSERT INTO user_substances(id,cas,formula,molar_mass,name_en,future_note) VALUES(41,'','H2O',18.015,'Legacy','keep me');PRAGMA user_version=1;").unwrap();
        drop(old);
        let db = UserStore::open(&path).unwrap();
        assert_eq!(db.get(41, "ru").unwrap().name_en, "Legacy");
        assert_eq!(
            db.conn
                .query_row("SELECT future_note FROM user_substances", [], |r| r
                    .get::<_, String>(0))
                .unwrap(),
            "keep me"
        );
        let backups: Vec<_> = fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().contains("pre-v1"))
            .collect();
        assert_eq!(backups.len(), 1);
        let original = Connection::open(backups[0].path()).unwrap();
        assert_eq!(
            original
                .pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
                .unwrap(),
            1
        );
        drop(original);
        drop(db);
        let db = UserStore::open(&path).unwrap();
        drop(db);
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 2);
        fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn future_unrecognized_and_failed_migrations_never_reset_data() {
        let dir = folder();
        let path = dir.join("user.sqlite");
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch("CREATE TABLE irreplaceable(value TEXT);INSERT INTO irreplaceable VALUES('keep');PRAGMA user_version=99;").unwrap();
        drop(conn);
        let before = fs::read(&path).unwrap();
        assert!(UserStore::open(&path).err().unwrap().contains("newer"));
        assert_eq!(fs::read(&path).unwrap(), before);
        let conn = Connection::open(&path).unwrap();
        conn.pragma_update(None, "user_version", 0).unwrap();
        drop(conn);
        assert!(UserStore::open(&path)
            .err()
            .unwrap()
            .contains("Unrecognized"));
        let bad = dir.join("bad.sqlite");
        let conn = Connection::open(&bad).unwrap();
        conn.execute_batch(V1).unwrap();
        conn.execute_batch("INSERT INTO user_substances(formula,molar_mass,name_en) VALUES('H2O',18.015,'Keep');CREATE TABLE user_localizations(blocked TEXT);PRAGMA user_version=1;").unwrap();
        drop(conn);
        assert!(UserStore::open(&bad).is_err());
        let conn = Connection::open(&bad).unwrap();
        assert_eq!(
            conn.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert!(conn
            .prepare("SELECT revision FROM user_substances")
            .is_err());
        assert_eq!(
            conn.query_row("SELECT name_en FROM user_substances", [], |r| r
                .get::<_, String>(0))
                .unwrap(),
            "Keep"
        );
        drop(conn);
        fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn migration_backup_includes_committed_wal_rows() {
        let dir = folder();
        let path = dir.join("wal.sqlite");
        let old = Connection::open(&path).unwrap();
        old.pragma_update(None, "journal_mode", "WAL").unwrap();
        old.pragma_update(None, "wal_autocheckpoint", 0).unwrap();
        old.execute_batch(V1).unwrap();
        old.execute_batch("INSERT INTO user_substances(formula,molar_mass,name_en) VALUES('H2O',18.015,'WAL row');PRAGMA user_version=1;").unwrap();
        let db = UserStore::open(&path).unwrap();
        assert_eq!(db.records("en").unwrap()[0].name_en, "WAL row");
        let saved = fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .find(|e| e.file_name().to_string_lossy().contains("pre-v1"))
            .unwrap();
        let backup = Connection::open(saved.path()).unwrap();
        assert_eq!(
            backup
                .query_row("SELECT name_en FROM user_substances", [], |r| r
                    .get::<_, String>(0))
                .unwrap(),
            "WAL row"
        );
        drop(backup);
        drop(db);
        drop(old);
        fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn loading_new_bundled_catalogs_or_languages_does_not_rewrite_user_database() {
        let dir = folder();
        let path = dir.join("user.sqlite");
        let mut db = UserStore::open(&path).unwrap();
        let id = db.save(&record()).unwrap();
        drop(db);
        let original = fs::read(&path).unwrap();
        for code in ["en", "ru", "de", "en"] {
            let catalog = crate::substances::Catalog::load(code, &path).unwrap();
            assert!(catalog.entries.iter().any(|e| e.user_id == Some(id)));
        }
        assert_eq!(fs::read(&path).unwrap(), original);
        fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn localized_name_and_mass_are_sufficient_and_fallback_survives_english_edit() {
        let dir = folder();
        let path = dir.join("user.sqlite");
        let mut db = UserStore::open(&path).unwrap();
        let mut draft = UserRecord {
            locale: "ru".into(),
            local_name: "Мой реагент".into(),
            molar_mass: 123.456,
            ..Default::default()
        };
        let id = db.save(&draft).unwrap();
        let c = crate::substances::Catalog::load("en", &path).unwrap();
        let entry = c.entries.iter().find(|e| e.user_id == Some(id)).unwrap();
        assert_eq!(entry.name(crate::settings::Language::En), "Мой реагент");
        assert!(entry.formula.is_empty());
        assert!(entry.cas.is_empty());
        let mut english = db.get(id, "en").unwrap();
        english.molar_mass = 321.0;
        db.save(&english).unwrap();
        assert_eq!(db.get(id, "ru").unwrap().local_name, "Мой реагент");
        let mut only_name = db.get(id, "ru").unwrap();
        only_name.local_name.clear();
        assert!(db
            .save(&only_name)
            .unwrap_err()
            .contains("at least one name"));
        draft.local_name.clear();
        assert!(db.save(&draft).is_err());
        drop(db);
        fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn delete_removes_all_locales_persists_and_restores_bundled_cas_entry() {
        let dir = folder();
        let path = dir.join("user.sqlite");
        let mut db = UserStore::open(&path).unwrap();
        let id = db.save(&record()).unwrap();
        let mut german = db.get(id, "de").unwrap();
        german.local_name = "Benutzerzucker".into();
        db.save(&german).unwrap();
        let mut other = record();
        other.cas.clear();
        other.name_en = "Keep this".into();
        other.synonyms_en.clear();
        other.local_synonyms.clear();
        let keep = db.save(&other).unwrap();
        let before = crate::substances::Catalog::load("ru", &path).unwrap();
        assert!(before.entries.iter().any(|e| e.user_id == Some(id)));
        db.delete(id, db.get(id, "en").unwrap().revision).unwrap();
        drop(db);
        let db = UserStore::open(&path).unwrap();
        assert!(db.get(id, "ru").is_err());
        assert_eq!(db.get(keep, "ru").unwrap().name_en, "Keep this");
        assert_eq!(
            db.conn
                .query_row(
                    "SELECT count(*) FROM user_localizations WHERE substance_id=?1",
                    [id],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
            0
        );
        let after = crate::substances::Catalog::load("ru", &path).unwrap();
        let original = after.entries.iter().find(|e| e.cas == "50-99-7").unwrap();
        assert_eq!(original.user_id, None);
        assert_eq!(original.name_en, "D-Glucose, anhydrous");
        assert!(after
            .search("test alias", crate::settings::Language::Ru)
            .is_empty());
        drop(db);
        fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn stale_and_blocked_delete_preserve_core_and_translations() {
        let dir = folder();
        let path = dir.join("user.sqlite");
        let mut db = UserStore::open(&path).unwrap();
        let id = db.save(&record()).unwrap();
        let stale = db.get(id, "ru").unwrap();
        let mut edited = stale.clone();
        edited.name_en = "Latest".into();
        db.save(&edited).unwrap();
        assert!(db
            .delete(id, stale.revision)
            .unwrap_err()
            .contains("before deleting"));
        assert_eq!(db.get(id, "ru").unwrap().local_name, "Тестовая глюкоза");
        db.conn.execute_batch("CREATE TABLE future_references(substance_id INTEGER REFERENCES user_substances(id))").unwrap();
        db.conn
            .execute("INSERT INTO future_references VALUES(?1)", [id])
            .unwrap();
        assert!(db.delete(id, db.get(id, "en").unwrap().revision).is_err());
        assert_eq!(db.get(id, "ru").unwrap().name_en, "Latest");
        assert_eq!(db.get(id, "ru").unwrap().local_name, "Тестовая глюкоза");
        assert!(db.delete(9999, 1).unwrap_err().contains("no longer exists"));
        drop(db);
        fs::remove_dir_all(dir).unwrap();
    }
}
