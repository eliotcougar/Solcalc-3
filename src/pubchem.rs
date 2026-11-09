//! Explicit, bounded PubChem lookup. No network operation runs on the UI thread.
use crate::{
    settings::Language,
    user_db::{self, UserRecord},
};
use serde_json::Value;
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};

const MAX_RESULTS: usize = 10;
const MAX_BODY: usize = 4 * 1024 * 1024;
static LAST_REQUEST: Mutex<Option<Instant>> = Mutex::new(None);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    InvalidQuery,
    NotFound,
    Network(String),
    Http(u32),
    InvalidResponse,
    Cancelled,
}
impl Error {
    pub fn message(&self, language: Language) -> String {
        match self {
            Self::InvalidQuery => language
                .choose(
                    "Enter a name or CAS number (up to 200 characters).",
                    "Введите название или номер CAS (не более 200 символов).",
                )
                .into(),
            Self::NotFound => language
                .choose(
                    "No suitable substances were found in PubChem.",
                    "Подходящие вещества в PubChem не найдены.",
                )
                .into(),
            Self::Network(detail) => format!(
                "{} {}",
                language.choose(
                    "Could not connect to PubChem:",
                    "Не удалось подключиться к PubChem:"
                ),
                detail.chars().take(160).collect::<String>()
            ),
            Self::Http(429 | 503) => language
                .choose(
                    "PubChem is busy. Try again later.",
                    "PubChem перегружен. Повторите поиск позже.",
                )
                .into(),
            Self::Http(code) => format!(
                "{} {code}",
                language.choose(
                    "PubChem request failed. HTTP:",
                    "Ошибка запроса PubChem. HTTP:"
                )
            ),
            Self::InvalidResponse => language
                .choose(
                    "PubChem returned an unsupported or oversized response.",
                    "PubChem вернул неподдерживаемый или слишком большой ответ.",
                )
                .into(),
            Self::Cancelled => language
                .choose("Search cancelled.", "Поиск отменён.")
                .into(),
        }
    }
}
#[derive(Clone, Debug)]
pub struct Compound {
    pub cid: u64,
    pub record: UserRecord,
    pub cas_numbers: Vec<String>,
}
pub struct Pending {
    receiver: Receiver<Result<Vec<Compound>, Error>>,
    cancelled: Arc<AtomicBool>,
}
impl Pending {
    pub fn poll(&self) -> Option<Result<Vec<Compound>, Error>> {
        match self.receiver.try_recv() {
            Ok(value) => Some(value),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => {
                Some(Err(Error::Network("Worker stopped".into())))
            }
        }
    }
}
impl Drop for Pending {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }
}
pub fn start(query: String) -> Result<Pending, Error> {
    validate_query(&query)?;
    let (sender, receiver) = mpsc::channel();
    let cancelled = Arc::new(AtomicBool::new(false));
    let worker_cancelled = cancelled.clone();
    std::thread::Builder::new()
        .name("pubchem-lookup".into())
        .spawn(move || {
            let result = lookup_with(&query, |path| {
                pace(&worker_cancelled)?;
                get(path, &worker_cancelled)
            });
            let _ = sender.send(result);
        })
        .map_err(|error| Error::Network(error.to_string()))?;
    Ok(Pending {
        receiver,
        cancelled,
    })
}
fn validate_query(query: &str) -> Result<(), Error> {
    if query.trim().is_empty() || query.chars().count() > 200 || query.chars().any(char::is_control)
    {
        Err(Error::InvalidQuery)
    } else {
        Ok(())
    }
}
fn pace(cancelled: &AtomicBool) -> Result<(), Error> {
    let mut last = LAST_REQUEST
        .lock()
        .map_err(|_| Error::Network("Request limiter unavailable".into()))?;
    while last.is_some_and(|t| t.elapsed() < Duration::from_millis(260)) {
        if cancelled.load(Ordering::Relaxed) {
            return Err(Error::Cancelled);
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    if cancelled.load(Ordering::Relaxed) {
        return Err(Error::Cancelled);
    }
    *last = Some(Instant::now());
    Ok(())
}
fn encode(value: &str) -> String {
    let mut result = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) {
            result.push(byte as char);
        } else {
            use std::fmt::Write;
            let _ = write!(result, "%{byte:02X}");
        }
    }
    result
}
fn text(value: &Value, max: usize) -> Option<String> {
    let value = value.as_str()?.trim();
    (!value.is_empty() && value.chars().count() <= max && !value.chars().any(char::is_control))
        .then(|| value.to_owned())
}
fn weight(value: &Value) -> Option<f64> {
    let value = value.as_f64().or_else(|| value.as_str()?.parse().ok())?;
    (value.is_finite() && value > 0.0).then_some(value)
}
fn lookup_with(
    query: &str,
    mut fetch: impl FnMut(&str) -> Result<Value, Error>,
) -> Result<Vec<Compound>, Error> {
    validate_query(query)?;
    let query = query.trim();
    let identifiers = fetch(&format!(
        "/rest/pug/compound/name/{}/cids/JSON",
        encode(query)
    ))?;
    let values = identifiers
        .pointer("/IdentifierList/CID")
        .and_then(Value::as_array)
        .ok_or(Error::InvalidResponse)?;
    let mut ids = vec![];
    for value in values {
        if let Some(id) = value.as_u64().filter(|id| *id > 0) {
            if !ids.contains(&id) {
                ids.push(id);
            }
            if ids.len() == MAX_RESULTS {
                break;
            }
        }
    }
    if ids.is_empty() {
        return Err(Error::NotFound);
    }
    let list = ids.iter().map(u64::to_string).collect::<Vec<_>>().join(",");
    let properties = fetch(&format!("/rest/pug/compound/cid/{list}/property/Title,IUPACName,MolecularFormula,MolecularWeight/JSON"))?;
    let synonyms = match fetch(&format!("/rest/pug/compound/cid/{list}/synonyms/JSON")) {
        Ok(value) => value,
        Err(Error::NotFound) => Value::Null,
        Err(error) => return Err(error),
    };
    let mut aliases = HashMap::new();
    if let Some(rows) = synonyms
        .pointer("/InformationList/Information")
        .and_then(Value::as_array)
    {
        for row in rows {
            if let (Some(id), Some(names)) = (row["CID"].as_u64(), row["Synonym"].as_array()) {
                aliases.insert(
                    id,
                    names
                        .iter()
                        .take(256)
                        .filter_map(|value| text(value, 1000))
                        .collect::<Vec<_>>(),
                );
            }
        }
    }
    let rows = properties
        .pointer("/PropertyTable/Properties")
        .and_then(Value::as_array)
        .ok_or(Error::InvalidResponse)?;
    let mut result = vec![];
    for row in rows {
        let Some(cid) = row["CID"].as_u64().filter(|id| ids.contains(id)) else {
            continue;
        };
        if result.iter().any(|entry: &Compound| entry.cid == cid) {
            continue;
        }
        let Some(molar_mass) = weight(&row["MolecularWeight"]) else {
            continue;
        };
        let Some(name_en) = text(&row["Title"], 200).or_else(|| text(&row["IUPACName"], 200))
        else {
            continue;
        };
        let Some(formula) = text(&row["MolecularFormula"], 200) else {
            continue;
        };
        let mut names = aliases.remove(&cid).unwrap_or_default();
        if let Some(iupac) = text(&row["IUPACName"], 1000) {
            names.push(iupac);
        }
        let mut numbers = names
            .iter()
            .filter(|s| user_db::valid_cas(s))
            .cloned()
            .collect::<Vec<_>>();
        numbers.sort();
        numbers.dedup();
        // Several CAS synonyms can refer to different forms or historical IDs.
        // Keep them visible, but never silently pick the first one.
        let cas = if user_db::valid_cas(query) && numbers.iter().any(|s| s.as_str() == query) {
            query.into()
        } else if numbers.len() == 1 {
            numbers[0].clone()
        } else {
            String::new()
        };
        let mut chosen = vec![format!("PubChem CID {cid}")];
        let mut length = chosen[0].chars().count();
        for alias in numbers.iter().cloned().chain(names) {
            let count = alias.chars().count();
            if alias != name_en && !chosen.contains(&alias) && length + count < 2000 {
                length += count + 1;
                chosen.push(alias);
            }
        }
        result.push(Compound {
            cid,
            cas_numbers: numbers,
            record: UserRecord {
                cas,
                formula,
                molar_mass,
                name_en,
                synonyms_en: chosen.join(";"),
                locale: "en".into(),
                ..Default::default()
            },
        });
    }
    if result.is_empty() {
        Err(Error::NotFound)
    } else {
        Ok(result)
    }
}

fn get(path: &str, cancelled: &AtomicBool) -> Result<Value, Error> {
    transport::download("pubchem.ncbi.nlm.nih.gov", 443, path, true, cancelled)
}

mod transport {
    use super::*;
    use std::io::Read;
    pub(super) fn download(
        host: &str,
        port: u16,
        path: &str,
        secure: bool,
        cancelled: &AtomicBool,
    ) -> Result<Value, Error> {
        if cancelled.load(Ordering::Relaxed) {
            return Err(Error::Cancelled);
        }
        let mut config = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .max_redirects(0)
            .max_redirects_will_error(false)
            .timeout_global(Some(Duration::from_secs(25)))
            .timeout_resolve(Some(Duration::from_secs(6)))
            .timeout_connect(Some(Duration::from_secs(6)))
            .timeout_recv_response(Some(Duration::from_secs(8)))
            .timeout_per_read(Some(Duration::from_secs(3)))
            .user_agent(concat!("Solcalc/", env!("CARGO_PKG_VERSION")));
        if host == "127.0.0.1" {
            config = config.proxy(None);
        }
        let agent = config.build().new_agent();
        let url = format!(
            "{}://{host}:{port}{path}",
            if secure { "https" } else { "http" }
        );
        let mut response = agent
            .get(&url)
            .header("Accept", "application/json")
            .call()
            .map_err(|error| Error::Network(error.to_string()))?;
        let status = response.status().as_u16() as u32;
        if status == 404 {
            return Err(Error::NotFound);
        }
        if status != 200 {
            return Err(Error::Http(status));
        }
        if response
            .headers()
            .get("content-length")
            .and_then(|s| s.to_str().ok())
            .and_then(|s| s.parse::<usize>().ok())
            .is_some_and(|length| length > MAX_BODY)
        {
            return Err(Error::InvalidResponse);
        }
        let mut bytes = vec![];
        let mut reader = response.body_mut().as_reader();
        loop {
            if cancelled.load(Ordering::Relaxed) {
                return Err(Error::Cancelled);
            }
            let mut buffer = [0u8; 16384];
            let count = reader
                .read(&mut buffer)
                .map_err(|error| Error::Network(error.to_string()))?;
            if count == 0 {
                break;
            }
            if bytes.len() + count > MAX_BODY {
                return Err(Error::InvalidResponse);
            }
            bytes.extend_from_slice(&buffer[..count]);
        }
        serde_json::from_slice(&bytes).map_err(|_| Error::InvalidResponse)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn fixture(query: &str, properties: Value, synonyms: Value) -> Result<Vec<Compound>, Error> {
        let mut replies = vec![
            json!({"IdentifierList":{"CID":[5234]}}),
            properties,
            synonyms,
        ]
        .into_iter();
        lookup_with(query, |_| Ok(replies.next().unwrap()))
    }
    fn salt() -> Value {
        json!({"PropertyTable":{"Properties":[{"CID":5234,"Title":"Sodium chloride","MolecularFormula":"ClNa","MolecularWeight":"58.44","ExactMass":57.9586}]}})
    }
    #[test]
    fn uses_average_mass_and_preserves_identifiers_without_writing_a_database() {
        let values = fixture("7647-14-5", salt(), json!({"InformationList":{"Information":[{"CID":5234,"Synonym":["Sodium chloride","table salt","7647-14-5"]}]}})).unwrap();
        let record = &values[0].record;
        assert_eq!(record.molar_mass, 58.44);
        assert_eq!(record.cas, "7647-14-5");
        assert!(record.synonyms_en.contains("PubChem CID 5234"));
        assert!(record.synonyms_en.contains("table salt"));
        assert!(record.id.is_none());
        user_db::validate(record).unwrap();
    }
    #[test]
    fn imported_results_require_save_and_cannot_overwrite_an_existing_user_cas() {
        let dir = std::env::temp_dir().join(format!(
            "solcalc-pubchem-import-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("user.sqlite");
        let mut store = user_db::UserStore::open(&path).unwrap();
        let id = store
            .save(&UserRecord {
                name_en: "My salt".into(),
                molar_mass: 58.5,
                cas: "7647-14-5".into(),
                locale: "ru".into(),
                local_name: "Моя соль".into(),
                ..Default::default()
            })
            .unwrap();
        drop(store);
        let before = std::fs::read(&path).unwrap();
        let fetched = fixture(
            "7647-14-5",
            salt(),
            json!({"InformationList":{"Information":[{"CID":5234,"Synonym":["7647-14-5"]}]}}),
        )
        .unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), before);
        let mut store = user_db::UserStore::open(&path).unwrap();
        assert!(store.save(&fetched[0].record).is_err());
        let original = store.get(id, "ru").unwrap();
        assert_eq!(original.name_en, "My salt");
        assert_eq!(original.local_name, "Моя соль");
        assert_eq!(original.molar_mass, 58.5);
        drop(store);
        assert_eq!(std::fs::read(&path).unwrap(), before);
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn ambiguous_cas_is_not_silently_chosen_and_malformed_properties_are_rejected() {
        let result = fixture("salt", salt(), json!({"InformationList":{"Information":[{"CID":5234,"Synonym":["7647-14-5","50-99-7"]}]}})).unwrap();
        assert!(result[0].record.cas.is_empty());
        assert!(result[0].record.synonyms_en.contains("50-99-7"));
        for mass in [
            json!(0),
            json!(-1),
            json!("NaN"),
            json!("1e999"),
            Value::Null,
        ] {
            let mut property = salt();
            property["PropertyTable"]["Properties"][0]["MolecularWeight"] = mass;
            assert_eq!(
                fixture("salt", property, json!({})).unwrap_err(),
                Error::NotFound
            );
        }
        let mut property = salt();
        property["PropertyTable"]["Properties"][0]["CID"] = json!(9);
        assert_eq!(
            fixture("salt", property, json!({})).unwrap_err(),
            Error::NotFound
        );
    }
    #[test]
    fn query_encoding_and_limits_and_transport_errors() {
        assert_eq!(encode("NaCl / 10%"), "NaCl%20%2F%2010%25");
        assert_eq!(encode("соль"), "%D1%81%D0%BE%D0%BB%D1%8C");
        for query in ["".into(), "x".repeat(201), "NaCl\0".into()] {
            assert_eq!(validate_query(&query), Err(Error::InvalidQuery));
        }
        assert_eq!(
            lookup_with("salt", |_| Err(Error::Http(503))).unwrap_err(),
            Error::Http(503)
        );
        assert_eq!(
            lookup_with("salt", |_| Ok(json!({"html":"captcha"}))).unwrap_err(),
            Error::InvalidResponse
        );
        assert_eq!(pace(&AtomicBool::new(true)), Err(Error::Cancelled));
        assert_ne!(
            Error::Http(429).message(Language::En),
            Error::Http(429).message(Language::Ru)
        );
    }
    #[test]
    fn candidate_and_synonym_limits_are_bounded() {
        let mut calls = 0;
        let result = lookup_with("salt", |path| {
            calls += 1;
            if calls == 1 { Ok(json!({"IdentifierList":{"CID":(1..100).collect::<Vec<_>>()}})) }
            else if calls == 2 {
                assert!(path.contains("cid/1,2,3,4,5,6,7,8,9,10/"));
                let mut value = salt(); value["PropertyTable"]["Properties"][0]["CID"] = json!(1); Ok(value)
            } else { Ok(json!({"InformationList":{"Information":[{"CID":1,"Synonym":(0..300).map(|i|format!("synonym {i} {}","x".repeat(100))).collect::<Vec<_>>()}]}})) }
        }).unwrap();
        assert_eq!(calls, 3);
        assert!(result[0].record.synonyms_en.chars().count() <= 2000);
        user_db::validate(&result[0].record).unwrap();
    }
    #[cfg(windows)]
    #[test]
    fn native_http_transport_reads_json_and_handles_http_and_invalid_bodies() {
        use std::{
            io::{Read, Write},
            net::TcpListener,
        };
        for (status, body, expected) in [
            ("200 OK", "{\"ok\":true}", Ok(json!({"ok":true}))),
            ("429 Too Many Requests", "busy", Err(Error::Http(429))),
            (
                "200 OK",
                "<html>blocked</html>",
                Err(Error::InvalidResponse),
            ),
        ] {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let port = listener.local_addr().unwrap().port();
            let server = std::thread::spawn(move || {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut buffer = [0; 2048];
                let _ = stream.read(&mut buffer).unwrap();
                write!(
                    stream,
                    "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                )
                .unwrap();
            });
            assert_eq!(
                transport::download("127.0.0.1", port, "/test", false, &AtomicBool::new(false)),
                expected
            );
            server.join().unwrap();
        }
    }
    #[cfg(windows)]
    #[test]
    fn native_transport_rejects_oversized_valid_json_and_honors_cancellation() {
        use std::{
            io::{Read, Write},
            net::TcpListener,
        };
        assert_eq!(
            transport::download("127.0.0.1", 1, "/", false, &AtomicBool::new(true)),
            Err(Error::Cancelled)
        );
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut buffer = [0; 2048];
            let _ = stream.read(&mut buffer).unwrap();
            let body = format!("\"{}\"", "a".repeat(MAX_BODY));
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(response.as_bytes());
        });
        assert_eq!(
            transport::download("127.0.0.1", port, "/", false, &AtomicBool::new(false)),
            Err(Error::InvalidResponse)
        );
        server.join().unwrap();
    }
    #[cfg(windows)]
    #[test]
    #[ignore = "Explicit live PubChem connectivity check; ordinary tests stay offline"]
    fn live_pubchem_smoke() {
        let cancelled = AtomicBool::new(false);
        let results = lookup_with("7647-14-5", |path| {
            pace(&cancelled)?;
            get(path, &cancelled)
        })
        .unwrap();
        assert!(results
            .iter()
            .any(|c| c.record.cas == "7647-14-5" && (c.record.molar_mass - 58.44).abs() < 0.01));
        println!("{}", results[0].record.name_en);
    }
}
