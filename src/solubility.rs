//! Read-only measurements for exact bundled chemical identities.
use crate::{
    calc::{self, SolubilityUnit},
    i18n,
    settings::Language,
    substances::{self, Substance},
    user_db,
};
use std::{collections::HashMap, sync::OnceLock};

const DB: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/data/bundled/solubility.sqlite"
));

pub struct Measurement {
    pub solvent: String,
    solvent_molar_mass: f64,
    amount: Amount,
    temperature_c: Option<f64>,
    ph: Option<f64>,
    pub source: String,
}

enum Amount {
    Measured {
        unit: SolubilityUnit,
        value: f64,
        density: Option<f64>,
    },
    Insoluble,
    PracticallyInsoluble,
    EsolPrediction(f64),
}

impl Measurement {
    pub fn molarity(&self, molar_mass: f64) -> Option<f64> {
        match self.amount {
            Amount::Measured {
                unit,
                value,
                density,
            } => unit.molarity(value, molar_mass, self.solvent_molar_mass, density),
            _ => None,
        }
    }

    /// Gives a way to compare the preparation directly by mass when the
    /// density needed for a final-volume comparison was not reported.
    pub fn solvent_mass_ratio(&self, molar_mass: f64) -> Option<f64> {
        match self.amount {
            Amount::Measured {
                unit,
                value,
                density: None,
            } => unit.solvent_to_solute_ratio(value, molar_mass, self.solvent_molar_mass),
            _ => None,
        }
    }

    pub fn is_qualitative(&self) -> bool {
        matches!(
            self.amount,
            Amount::Insoluble | Amount::PracticallyInsoluble
        )
    }

    pub fn prediction(&self) -> Option<f64> {
        match self.amount {
            Amount::EsolPrediction(molarity) => Some(molarity),
            _ => None,
        }
    }

    pub fn prediction_text(language: Language, molarity: f64) -> String {
        let label = i18n::text(language, "Predicted water solubility (ESOL)");
        let value = calc::display_concentration(molarity);
        i18n::numbers(language, &format!("{label}\n≈ {value}")).into_owned()
    }

    pub fn reference(&self, language: Language, limit: Option<f64>) -> String {
        let amount = match limit {
            Some(limit) => format!("≈ {}", calc::display_concentration(limit)),
            None => self.reported_amount(language),
        };
        self.with_conditions(language, amount)
    }

    pub fn picker_line(&self, language: Language) -> String {
        if let Some(molarity) = self.prediction() {
            let estimate = Self::prediction_text(language, molarity);
            return format!("{estimate} · {}", self.source);
        }
        let solvent = i18n::text(language, &self.solvent);
        let details = self.with_conditions(language, self.reported_amount(language));
        format!("{solvent}: {details} · {}", self.source)
    }

    fn with_conditions(&self, language: Language, mut amount: String) -> String {
        let mut conditions = Vec::new();
        if let Some(temperature) = self.temperature_c {
            conditions.push(format!("{} °C", calc::number(temperature)));
        } else {
            conditions.push(
                language
                    .choose("21 °C assumed", "21 °C предположительно")
                    .into(),
            );
        }
        if let Some(ph) = self.ph {
            conditions.push(format!("pH {}", calc::number(ph)));
        }
        if !conditions.is_empty() {
            amount.push_str(&format!(" ({})", conditions.join(", ")));
        }
        i18n::numbers(language, &amount).into_owned()
    }

    fn reported_amount(&self, language: Language) -> String {
        let (unit, value) = match self.amount {
            Amount::Measured { unit, value, .. } => (unit, calc::number(value)),
            Amount::Insoluble => return i18n::text(language, "Insoluble").into(),
            Amount::PracticallyInsoluble => {
                return i18n::text(language, "Practically insoluble").into();
            }
            Amount::EsolPrediction(molarity) => return Self::prediction_text(language, molarity),
        };
        let unit = match unit {
            SolubilityUnit::GramsPerLitre => i18n::text(language, "g/L solution"),
            SolubilityUnit::MolesPerLitre => "mol/L",
            SolubilityUnit::MassPercent => language.choose("% by mass", "% по массе"),
            SolubilityUnit::GramsPer100GramsSolvent => i18n::text(language, "g/100 g solvent"),
            SolubilityUnit::MolesPerKilogramSolvent => i18n::text(language, "mol/kg solvent"),
            SolubilityUnit::MoleFraction => {
                return format!(
                    "{} {value}",
                    i18n::text(language, "Saturation mole fraction")
                );
            }
        };
        format!("{value} {unit}")
    }
}

type Catalog = HashMap<String, Vec<Measurement>>;
static MEASUREMENTS: OnceLock<Catalog> = OnceLock::new();

pub fn for_substance(entry: &Substance) -> &'static [Measurement] {
    // CAS-based user overlays can replace the formula and molar mass.
    // Only attach measurements to the unmodified bundled chemical entry.
    if entry.user_id.is_some() {
        return &[];
    }
    let Some(key) = entry.id.strip_prefix("bundled:chebi:") else {
        return &[];
    };
    MEASUREMENTS
        .get_or_init(|| {
            load().unwrap_or_else(|error| {
                eprintln!("Could not load solubility measurements: {error}");
                Catalog::new()
            })
        })
        .get(key)
        .map(Vec::as_slice)
        .unwrap_or_default()
}

fn load() -> user_db::Result<Catalog> {
    let db = substances::embedded(DB, 0x534F4C53)?;
    let read = || -> rusqlite::Result<Catalog> {
        let mut query = db.prepare(
            "SELECT chebi_id,solvent,unit,value,temperature_c,ph,density,source,solvent_molar_mass
             FROM solubility ORDER BY chebi_id, solvent <> 'Water', solvent",
        )?;
        let mut rows = query.query([])?;
        let mut catalog = Catalog::new();
        while let Some(row) = rows.next()? {
            let amount = match row.get_ref(2)?.as_str()? {
                "insoluble" => Amount::Insoluble,
                "practically_insoluble" => Amount::PracticallyInsoluble,
                "esol_prediction" => Amount::EsolPrediction(row.get(3)?),
                unit => Amount::Measured {
                    unit: match unit {
                        "g_l_solution" => SolubilityUnit::GramsPerLitre,
                        "mol_l_solution" => SolubilityUnit::MolesPerLitre,
                        "mass_percent" => SolubilityUnit::MassPercent,
                        "g_100g_solvent" => SolubilityUnit::GramsPer100GramsSolvent,
                        "mol_kg_solvent" => SolubilityUnit::MolesPerKilogramSolvent,
                        "mole_fraction" => SolubilityUnit::MoleFraction,
                        _ => return Err(rusqlite::Error::InvalidQuery),
                    },
                    value: row.get(3)?,
                    density: row.get(6)?,
                },
            };
            catalog.entry(row.get(0)?).or_default().push(Measurement {
                solvent: row.get(1)?,
                solvent_molar_mass: row.get(8)?,
                amount,
                temperature_c: row.get(4)?,
                ph: row.get(5)?,
                source: row.get(7)?,
            });
        }
        Ok(catalog)
    };
    read().map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reference_can_compare_stock_strength_and_preserves_unconvertible_units() {
        let catalog = load().unwrap();
        let caffeine = &catalog["27732"];
        assert_eq!(caffeine[0].solvent, "Water");
        assert_eq!(caffeine[0].prediction(), None);
        let limit = caffeine[0].molarity(194.194).unwrap();
        assert!((limit - 0.11123).abs() < 1e-5);
        let stock = calc::stock(1e-3, 50e-6, 10.0, 10e-3, 0.0, 0.0).unwrap();
        assert!(calc::materially_less(limit, stock.concentration));
        assert!(!calc::materially_less(limit, 10e-3));
        assert!(!calc::materially_less(limit, limit));

        let ethanol = caffeine.iter().find(|m| m.solvent == "Ethanol").unwrap();
        assert_eq!(ethanol.molarity(194.194), None);
        let prep = calc::prepare_stock(
            calc::Basis::Molar,
            194.194,
            stock.concentration,
            stock.batch_volume,
            None,
            None,
        )
        .unwrap();
        let minimum = prep.mass * ethanol.solvent_mass_ratio(194.194).unwrap();
        assert!((minimum - 3.48547).abs() < 1e-5);
        assert!(calc::materially_less(3.0, minimum));
        assert!(!calc::materially_less(3.5, minimum));
        assert!(ethanol.picker_line(Language::Ru).contains("Этанол"));
        assert!(ethanol.picker_line(Language::Ru).contains(&ethanol.source));
    }

    #[test]
    fn esol_water_fallback_has_a_distinct_amount_and_label() {
        let catalog = load().unwrap();
        let water = &catalog["7"][0];
        assert_eq!(water.solvent, "Water");
        assert!((water.prediction().unwrap() - 0.0027).abs() < 1e-10);
        assert_eq!(water.molarity(136.238), None);
        assert_eq!(water.solvent_mass_ratio(136.238), None);
        assert!(!water.is_qualitative());
        assert!(water.picker_line(Language::Ru).starts_with("Прогноз"));
    }

    #[test]
    fn explicit_insolubility_and_unqualified_molar_data_remain_distinct() {
        let catalog = load().unwrap();
        let barium_sulfate = &catalog["133326"][0];
        assert_eq!(barium_sulfate.solvent, "Water");
        assert!(barium_sulfate.is_qualitative());
        assert_eq!(barium_sulfate.molarity(233.391), None);
        assert!(barium_sulfate
            .reference(Language::En, None)
            .starts_with("Insoluble"));
        let sodium_sulfate = catalog["32586"]
            .iter()
            .find(|measurement| measurement.solvent == "Ethanol")
            .unwrap();
        assert!(sodium_sulfate
            .reference(Language::Ru, None)
            .starts_with("Практически нерастворимо"));

        let reference = catalog
            .values()
            .flatten()
            .find(|measurement| measurement.source == "https://doi.org/10.57745/CZVZIA")
            .unwrap();
        assert_eq!(reference.solvent, "Water");
        assert!(!reference.is_qualitative());
        assert!(reference.molarity(200.0).is_some());
        assert_eq!(reference.solvent_mass_ratio(200.0), None);
        assert!(reference
            .reference(Language::En, None)
            .contains("21 °C assumed"));
    }

    #[test]
    fn experimental_limits_can_warn_in_water_and_change_with_solvent() {
        let catalog = load().unwrap();
        let paracetamol = &catalog["46195"][0];
        assert_eq!(paracetamol.solvent, "Water");
        let limit = paracetamol.molarity(151.165).unwrap();
        assert!((limit - 0.092619).abs() < 1e-6);
        assert!(calc::materially_less(limit, 0.1));
        assert!(!calc::materially_less(limit, 0.05));

        let iodide = &catalog["33167"];
        let limit = |solvent| {
            iodide
                .iter()
                .find(|m| m.solvent == solvent)
                .and_then(|m| m.molarity(149.894))
                .unwrap()
        };
        assert!(calc::materially_less(limit("Isopropanol"), 0.5));
        assert!(!calc::materially_less(limit("Water"), 0.5));
    }
}
