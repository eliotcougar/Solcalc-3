//! Pure calculation code. Quantities cross this boundary in SI base units:
//! grams, litres, and either mol/L with g/mol, or U/mL with U/mg.
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub struct Error(pub String);
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Unit {
    Litre,
    Millilitre,
    Microlitre,
    Gram,
    Milligram,
    Microgram,
    Nanogram,
    Kilogram,
    Molar,
    Millimolar,
    Micromolar,
    Nanomolar,
    Picomolar,
    Nanolitre,
    UnitsPerMillilitre,
    KiloUnitsPerMillilitre,
}
impl Unit {
    pub fn scale(self) -> f64 {
        match self {
            Self::Litre | Self::Gram | Self::Molar | Self::UnitsPerMillilitre => 1.0,
            Self::Millilitre | Self::Milligram | Self::Millimolar => 1e-3,
            Self::Microlitre | Self::Microgram | Self::Micromolar => 1e-6,
            Self::Nanolitre | Self::Nanogram | Self::Nanomolar => 1e-9,
            Self::Picomolar => 1e-12,
            Self::Kilogram | Self::KiloUnitsPerMillilitre => 1e3,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Litre => "L",
            Self::Millilitre => "mL",
            Self::Microlitre => "µL",
            Self::Gram => "g",
            Self::Milligram => "mg",
            Self::Microgram => "µg",
            Self::Nanogram => "ng",
            Self::Kilogram => "kg",
            Self::Molar => "M",
            Self::Millimolar => "mM",
            Self::Micromolar => "µM",
            Self::Nanomolar => "nM",
            Self::Picomolar => "pM",
            Self::Nanolitre => "nL",
            Self::UnitsPerMillilitre => "U/mL",
            Self::KiloUnitsPerMillilitre => "kU/mL",
        }
    }
    pub fn to_base(self, value: f64) -> Result<f64> {
        positive(value * self.scale(), "Converted value")
    }
    pub fn display(self, base: f64) -> String {
        format!("{} {}", number(base / self.scale()), self.label())
    }
}
pub const VOLUME: &[Unit] = &[
    Unit::Litre,
    Unit::Millilitre,
    Unit::Microlitre,
    Unit::Nanolitre,
];
pub const MASS: &[Unit] = &[
    Unit::Gram,
    Unit::Milligram,
    Unit::Microgram,
    Unit::Nanogram,
    Unit::Kilogram,
];
pub const CONCENTRATION: &[Unit] = &[
    Unit::Molar,
    Unit::Millimolar,
    Unit::Micromolar,
    Unit::Nanomolar,
    Unit::Picomolar,
];
pub const ACTIVITY_CONCENTRATION: &[Unit] =
    &[Unit::UnitsPerMillilitre, Unit::KiloUnitsPerMillilitre];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Basis {
    #[default]
    Molar,
    Activity,
}
impl Basis {
    pub fn from_index(index: usize) -> Self {
        if index == 1 {
            Self::Activity
        } else {
            Self::Molar
        }
    }
    pub fn index(self) -> usize {
        usize::from(self == Self::Activity)
    }
    pub fn coefficient_label(self) -> &'static str {
        match self {
            Self::Molar => "Molar mass",
            Self::Activity => "Specific activity",
        }
    }
    pub fn concentration_units(self) -> &'static [Unit] {
        match self {
            Self::Molar => CONCENTRATION,
            Self::Activity => ACTIVITY_CONCENTRATION,
        }
    }
    pub fn display_concentration(self, value: f64) -> String {
        match self {
            Self::Molar => display_concentration(value),
            Self::Activity => display_auto(value, ACTIVITY_CONCENTRATION),
        }
    }
    pub fn mass(self, coefficient: f64, volume: f64, concentration: f64) -> Result<f64> {
        match self {
            Self::Molar => mass(coefficient, volume, concentration),
            Self::Activity => {
                positive(coefficient, "Specific activity")?;
                positive(volume, "Volume")?;
                positive(concentration, "Concentration")?;
                // C[U/mL] * V[L] / S[U/mg] gives grams: the two factors
                // of 1000 for mL/L and mg/g cancel exactly.
                output((volume * concentration) / coefficient)
            }
        }
    }
    pub fn volume(self, coefficient: f64, mass: f64, concentration: f64) -> Result<f64> {
        match self {
            Self::Molar => volume(coefficient, mass, concentration),
            Self::Activity => {
                positive(coefficient, "Specific activity")?;
                positive(mass, "Mass")?;
                positive(concentration, "Concentration")?;
                output((mass * coefficient) / concentration)
            }
        }
    }
    pub fn concentration(self, coefficient: f64, mass: f64, volume: f64) -> Result<f64> {
        match self {
            Self::Molar => concentration(coefficient, mass, volume),
            Self::Activity => {
                positive(coefficient, "Specific activity")?;
                positive(mass, "Mass")?;
                positive(volume, "Volume")?;
                output((mass * coefficient) / volume)
            }
        }
    }
}

/// Select display units only; input values and calculation/rounding units
/// never change. Respect the six significant digits shown by number() at
/// prefix boundaries, so a rounded 1000 µL is presented as 1 mL instead.
pub fn display_auto(base: f64, units: &[Unit]) -> String {
    if base == 0.0 {
        return format!("0 {}", units[0].label());
    }
    let shown_magnitude = number(base.abs()).parse::<f64>().unwrap_or(base.abs());
    let selected = units
        .iter()
        .copied()
        .filter(|unit| shown_magnitude >= unit.scale())
        .max_by(|a, b| a.scale().total_cmp(&b.scale()))
        .or_else(|| {
            units
                .iter()
                .copied()
                .min_by(|a, b| a.scale().total_cmp(&b.scale()))
        })
        .expect("A quantity must have at least one display unit");
    selected.display(base)
}

pub fn display_volume(litres: f64) -> String {
    display_auto(litres, VOLUME)
}
pub fn display_mass(grams: f64) -> String {
    display_auto(grams, MASS)
}
pub fn display_concentration(molar: f64) -> String {
    display_auto(molar, CONCENTRATION)
}

/// Decimal comma is accepted, but thousands separators and mixed separators
/// are deliberately rejected instead of guessing a magnitude.
pub fn parse(text: &str, label: &str) -> Result<Option<f64>> {
    let text = text.trim();
    if text.is_empty() {
        return Ok(None);
    }
    let value = text
        .replace(',', ".")
        .parse::<f64>()
        .map_err(|_| Error(format!("Enter a number for {}.", label.to_lowercase())))?;
    if !value.is_finite() {
        return Err(Error(format!("{label} must be a finite number.")));
    }
    Ok(Some(value))
}
pub fn positive(value: f64, label: &str) -> Result<f64> {
    if !value.is_finite() || value <= 0.0 {
        Err(Error(format!(
            "{label} must be greater than zero and within range."
        )))
    } else {
        Ok(value)
    }
}
fn output(value: f64) -> Result<f64> {
    positive(value, "Result").map_err(|_| {
        Error("Result is outside the supported numeric range. Check the values and units.".into())
    })
}

pub fn mass(molar_mass: f64, volume: f64, concentration: f64) -> Result<f64> {
    positive(molar_mass, "Molar mass")?;
    positive(volume, "Volume")?;
    positive(concentration, "Concentration")?;
    output(molar_mass * (volume * concentration))
}
pub fn volume(molar_mass: f64, mass: f64, concentration: f64) -> Result<f64> {
    positive(molar_mass, "Molar mass")?;
    positive(mass, "Mass")?;
    positive(concentration, "Concentration")?;
    output((mass / molar_mass) / concentration)
}
pub fn concentration(molar_mass: f64, mass: f64, volume: f64) -> Result<f64> {
    positive(molar_mass, "Molar mass")?;
    positive(mass, "Mass")?;
    positive(volume, "Volume")?;
    output((mass / molar_mass) / volume)
}

#[derive(Debug, PartialEq)]
pub enum MissingQuantity {
    MolarMass(f64),
    SpecificActivity(f64),
    Concentration(f64),
    Volume(f64),
    Mass(f64),
}

/// Solve the one blank field. Several blanks mean that input is incomplete;
/// four filled fields are ambiguous and must not select a result arbitrarily.
pub fn solve_solution(
    basis: Basis,
    coefficient: Option<f64>,
    concentration: Option<f64>,
    volume: Option<f64>,
    mass: Option<f64>,
) -> Result<Option<MissingQuantity>> {
    for (value, label) in [
        (coefficient, basis.coefficient_label()),
        (concentration, "Concentration"),
        (volume, "Volume"),
        (mass, "Mass"),
    ] {
        if let Some(value) = value {
            positive(value, label)?;
        }
    }
    Ok(match (coefficient, concentration, volume, mass) {
        (Some(coefficient), Some(c), Some(v), None) => {
            Some(MissingQuantity::Mass(basis.mass(coefficient, v, c)?))
        }
        (Some(coefficient), Some(c), None, Some(m)) => {
            Some(MissingQuantity::Volume(basis.volume(coefficient, m, c)?))
        }
        (Some(coefficient), None, Some(v), Some(m)) => Some(MissingQuantity::Concentration(
            basis.concentration(coefficient, m, v)?,
        )),
        (None, Some(c), Some(v), Some(m)) => Some(match basis {
            Basis::Molar => MissingQuantity::MolarMass(output((m / v) / c)?),
            Basis::Activity => MissingQuantity::SpecificActivity(output((c * v) / m)?),
        }),
        (Some(_), Some(_), Some(_), Some(_)) => return Err(Error("Leave one field blank.".into())),
        _ => None,
    })
}

#[derive(Debug)]
pub struct Preparation {
    pub mass: f64,
    pub volume: f64,
}

/// With both preparation fields blank, use the planned batch volume. Once
/// either is entered, calculate the other; never reinterpret one input as
/// another quantity when the user changes what they want to solve.
pub fn prepare_stock(
    basis: Basis,
    coefficient: f64,
    concentration: f64,
    planned_volume: f64,
    volume: Option<f64>,
    mass: Option<f64>,
) -> Result<Preparation> {
    positive(planned_volume, "Planned volume")?;
    match (volume, mass) {
        (Some(_), Some(_)) => Err(Error("Leave mass or batch volume blank.".into())),
        (None, Some(m)) => Ok(Preparation {
            mass: m,
            volume: basis.volume(coefficient, m, concentration)?,
        }),
        (v, None) => {
            let v = v.unwrap_or(planned_volume);
            if materially_less(v, planned_volume) {
                return Err(Error(
                    "Batch volume is smaller than the planned stock volume.".into(),
                ));
            }
            Ok(Preparation {
                mass: basis.mass(coefficient, v, concentration)?,
                volume: v,
            })
        }
    }
}

#[derive(Debug)]
pub struct Dilution {
    pub stock: f64,
    pub solvent: f64,
    pub factor: f64,
}
pub fn dilute(stock: f64, desired: f64, final_volume: f64) -> Result<Dilution> {
    positive(stock, "Stock concentration")?;
    positive(desired, "Desired concentration")?;
    positive(final_volume, "Final volume")?;
    if desired > stock {
        return Err(Error(
            "Desired concentration cannot exceed stock concentration.".into(),
        ));
    }
    let stock_volume = output((desired / stock) * final_volume)?;
    let factor = output(stock / desired)?;
    Ok(Dilution {
        stock: stock_volume,
        solvent: (final_volume - stock_volume).max(0.0),
        factor,
    })
}

#[derive(Debug)]
pub struct Stock {
    pub concentration: f64,
    pub minimum_volume: f64,
    pub batch_volume: f64,
}
pub fn stock(
    sample_volume: f64,
    addition: f64,
    samples: f64,
    final_concentration: f64,
    extra_percent: f64,
    round_litres: f64,
) -> Result<Stock> {
    positive(sample_volume, "Final sample volume")?;
    positive(addition, "Stock per sample")?;
    positive(final_concentration, "Final concentration")?;
    if addition > sample_volume {
        return Err(Error(
            "Stock per sample cannot exceed the final sample volume.".into(),
        ));
    }
    if !samples.is_finite() || samples < 1.0 || samples.fract() != 0.0 || samples > 1_000_000_000.0
    {
        return Err(Error(
            "Number of samples must be a whole number from 1 to 1,000,000,000.".into(),
        ));
    }
    if !extra_percent.is_finite() || extra_percent < 0.0 {
        return Err(Error("Extra volume must be zero or greater.".into()));
    }
    if !round_litres.is_finite() || round_litres < 0.0 {
        return Err(Error("Invalid rounding increment.".into()));
    }
    let concentration = output(final_concentration * (sample_volume / addition))?;
    let minimum_volume = output(addition * samples)?;
    let with_extra = output(minimum_volume * (1.0 + extra_percent / 100.0))?;
    let batch_volume = if round_litres > 0.0 {
        let steps = with_extra / round_litres;
        // Do not add a whole increment because of floating point noise at an
        // exact boundary. The tolerance is relative to the step count.
        let rounded = (steps - 8.0 * f64::EPSILON * steps.abs()).ceil() * round_litres;
        output(rounded.max(with_extra))?
    } else {
        with_extra
    };
    Ok(Stock {
        concentration,
        minimum_volume,
        batch_volume,
    })
}

pub fn number(value: f64) -> String {
    if value == 0.0 {
        return "0".into();
    }
    if !value.is_finite() {
        return "Out of range".into();
    }
    let magnitude = value.abs().log10().floor() as i32;
    if !(-4..6).contains(&magnitude) {
        let s = format!("{value:.5e}");
        let (mantissa, exponent) = s.split_once('e').unwrap();
        format!(
            "{}e{}",
            mantissa.trim_end_matches('0').trim_end_matches('.'),
            exponent
        )
    } else {
        let decimals = (5 - magnitude).max(0) as usize;
        let s = format!("{value:.decimals$}");
        if s.contains('.') {
            s.trim_end_matches('0').trim_end_matches('.').into()
        } else {
            s
        }
    }
}

/// Comparisons at unit-conversion boundaries allow a few floating-point ULPs.
/// This prevents a 300 µL batch being rejected as less than 300 µL.
pub fn materially_less(actual: f64, required: f64) -> bool {
    actual < required && required - actual > 16.0 * f64::EPSILON * required.abs()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() <= expected.abs().max(1e-300) * 1e-12,
            "{actual} != {expected}"
        );
    }
    #[test]
    fn salt_mass_and_inverse() {
        close(mass(58.44, 0.1, 0.1).unwrap(), 0.5844);
        close(volume(58.44, 0.5844, 0.1).unwrap(), 0.1);
        close(concentration(58.44, 0.5844, 0.1).unwrap(), 0.1);
    }
    #[test]
    fn blank_field_selects_each_solution_quantity() {
        use MissingQuantity::*;
        for (input, expected) in [
            ([Some(58.44), Some(0.1), Some(0.1), None], Mass(0.5844)),
            ([Some(58.44), Some(0.1), None, Some(0.5844)], Volume(0.1)),
            (
                [Some(58.44), None, Some(0.1), Some(0.5844)],
                Concentration(0.1),
            ),
            ([None, Some(0.1), Some(0.1), Some(0.5844)], MolarMass(58.44)),
        ] {
            let actual = solve_solution(Basis::Molar, input[0], input[1], input[2], input[3])
                .unwrap()
                .unwrap();
            match (actual, expected) {
                (Mass(a), Mass(b))
                | (Volume(a), Volume(b))
                | (Concentration(a), Concentration(b))
                | (MolarMass(a), MolarMass(b)) => close(a, b),
                _ => panic!("Wrong quantity selected"),
            }
        }
    }
    #[test]
    fn incomplete_solution_clears_result_and_full_solution_is_ambiguous() {
        assert_eq!(
            solve_solution(Basis::Molar, None, None, None, None).unwrap(),
            None
        );
        assert_eq!(
            solve_solution(Basis::Molar, Some(58.44), None, Some(0.1), None).unwrap(),
            None
        );
        assert!(solve_solution(
            Basis::Molar,
            Some(58.44),
            Some(0.1),
            Some(0.1),
            Some(0.5844)
        )
        .is_err());
    }
    #[test]
    fn zero_or_invalid_values_are_not_blank() {
        for value in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert!(
                solve_solution(Basis::Molar, Some(58.44), Some(0.1), Some(value), None).is_err()
            );
            assert!(solve_solution(Basis::Molar, Some(value), None, None, None).is_err());
        }
    }
    #[test]
    fn blank_stock_field_selects_mass_or_volume() {
        let planned = 0.0003;
        let default = prepare_stock(Basis::Molar, 180.156, 0.001, planned, None, None).unwrap();
        close(default.mass, 0.0000540468);
        close(default.volume, planned);
        let explicit =
            prepare_stock(Basis::Molar, 180.156, 0.001, planned, Some(0.0005), None).unwrap();
        close(explicit.mass, 0.000090078);
        let inverse = prepare_stock(
            Basis::Molar,
            180.156,
            0.001,
            planned,
            None,
            Some(default.mass),
        )
        .unwrap();
        close(inverse.volume, planned);
    }
    #[test]
    fn stock_preparation_rejects_ambiguous_or_insufficient_volume() {
        assert!(prepare_stock(
            Basis::Molar,
            180.156,
            0.001,
            0.0003,
            Some(0.0003),
            Some(0.001)
        )
        .is_err());
        assert!(prepare_stock(Basis::Molar, 180.156, 0.001, 0.0003, Some(0.0002), None).is_err());
        let small =
            prepare_stock(Basis::Molar, 180.156, 0.001, 0.0003, None, Some(0.000001)).unwrap();
        assert!(materially_less(small.volume, 0.0003));
    }
    #[test]
    fn every_unit_round_trip() {
        for unit in VOLUME
            .iter()
            .chain(MASS)
            .chain(CONCENTRATION)
            .chain(ACTIVITY_CONCENTRATION)
        {
            close(unit.to_base(123.45).unwrap() / unit.scale(), 123.45);
        }
        close(Unit::Microlitre.to_base(100.0).unwrap(), 0.0001);
        close(Unit::Milligram.to_base(5.0).unwrap(), 0.005);
        close(Unit::Micromolar.to_base(10.0).unwrap(), 0.00001);
    }
    #[test]
    fn activity_units_are_only_per_millilitre() {
        assert_eq!(
            Basis::Activity
                .concentration_units()
                .iter()
                .map(|u| u.label())
                .collect::<Vec<_>>(),
            ["U/mL", "kU/mL"]
        );
        close(Unit::KiloUnitsPerMillilitre.to_base(0.01).unwrap(), 10.0);
        assert_eq!(Basis::from_index(1), Basis::Activity);
        assert_eq!(Basis::from_index(0), Basis::Molar);
    }
    #[test]
    fn protein_mass_uses_specific_activity_and_accounts_for_mg_and_ml() {
        let c = Unit::UnitsPerMillilitre.to_base(10.0).unwrap();
        let v = Unit::Millilitre.to_base(100.0).unwrap();
        let mass = Basis::Activity.mass(100.0, v, c).unwrap();
        close(mass, Unit::Milligram.to_base(10.0).unwrap());
        close(
            Basis::Activity
                .mass(
                    100.0,
                    v,
                    Unit::KiloUnitsPerMillilitre.to_base(0.01).unwrap(),
                )
                .unwrap(),
            mass,
        );
        close(Basis::Activity.volume(100.0, mass, c).unwrap(), v);
        close(Basis::Activity.concentration(100.0, mass, v).unwrap(), c);
    }
    #[test]
    fn protein_blank_field_selects_all_four_quantities() {
        for (input, target) in [
            ([Some(100.0), Some(10.0), Some(0.1), None], 0),
            ([Some(100.0), Some(10.0), None, Some(0.01)], 1),
            ([Some(100.0), None, Some(0.1), Some(0.01)], 2),
            ([None, Some(10.0), Some(0.1), Some(0.01)], 3),
        ] {
            let actual = solve_solution(Basis::Activity, input[0], input[1], input[2], input[3])
                .unwrap()
                .unwrap();
            match (target, actual) {
                (0, MissingQuantity::Mass(v)) => close(v, 0.01),
                (1, MissingQuantity::Volume(v)) => close(v, 0.1),
                (2, MissingQuantity::Concentration(v)) => close(v, 10.0),
                (3, MissingQuantity::SpecificActivity(v)) => close(v, 100.0),
                _ => panic!("Incorrect protein result type"),
            }
        }
    }
    #[test]
    fn protein_incomplete_and_ambiguous_values_follow_the_blank_rule() {
        assert_eq!(
            solve_solution(Basis::Activity, None, None, None, None).unwrap(),
            None
        );
        assert!(solve_solution(
            Basis::Activity,
            Some(100.0),
            Some(10.0),
            Some(0.1),
            Some(0.01)
        )
        .is_err());
        for v in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert!(solve_solution(Basis::Activity, Some(v), Some(10.0), Some(0.1), None).is_err());
            assert!(Basis::Activity.mass(100.0, v, 10.0).is_err());
            assert!(Basis::Activity.volume(100.0, 0.01, v).is_err());
        }
    }
    #[test]
    fn activity_dilution_accepts_mixed_u_and_ku_units() {
        let d = dilute(
            Unit::KiloUnitsPerMillilitre.to_base(1.0).unwrap(),
            Unit::UnitsPerMillilitre.to_base(10.0).unwrap(),
            0.1,
        )
        .unwrap();
        close(d.stock, 0.001);
        close(d.solvent, 0.099);
        close(d.factor, 100.0);
        assert!(dilute(
            Unit::KiloUnitsPerMillilitre.to_base(0.01).unwrap(),
            20.0,
            0.1
        )
        .is_err());
    }
    #[test]
    fn protein_stock_planning_prepares_and_inverts_the_correct_mass() {
        let s = stock(0.002, 0.00002, 48.0, 0.5, 10.0, 0.0001).unwrap();
        close(s.concentration, 50.0);
        close(s.minimum_volume, 0.00096);
        close(s.batch_volume, 0.0011);
        let prep = prepare_stock(
            Basis::Activity,
            250.0,
            s.concentration,
            s.batch_volume,
            None,
            None,
        )
        .unwrap();
        close(prep.mass, 0.00022);
        close(
            prepare_stock(
                Basis::Activity,
                250.0,
                s.concentration,
                s.batch_volume,
                None,
                Some(prep.mass),
            )
            .unwrap()
            .volume,
            s.batch_volume,
        );
        assert!(prepare_stock(
            Basis::Activity,
            250.0,
            s.concentration,
            s.batch_volume,
            Some(0.001),
            None
        )
        .is_err());
    }
    #[test]
    fn protein_concentration_automatically_chooses_u_or_ku_per_ml() {
        for (value, display) in [
            (0.01, "0.01 U/mL"),
            (999.999, "999.999 U/mL"),
            (1000.0, "1 kU/mL"),
            (999.9999, "1 kU/mL"),
            (12500.0, "12.5 kU/mL"),
        ] {
            assert_eq!(Basis::Activity.display_concentration(value), display);
        }
    }
    #[test]
    fn protein_inverse_properties_across_activity_and_volume_scales() {
        for a in [0.01, 1.0, 100.0, 100000.0] {
            for v in [1e-6, 0.001, 0.1, 1.0] {
                for c in [0.001, 1.0, 100.0, 1000.0] {
                    let m = Basis::Activity.mass(a, v, c).unwrap();
                    close(Basis::Activity.volume(a, m, c).unwrap(), v);
                    close(Basis::Activity.concentration(a, m, v).unwrap(), c);
                    match solve_solution(Basis::Activity, None, Some(c), Some(v), Some(m))
                        .unwrap()
                        .unwrap()
                    {
                        MissingQuantity::SpecificActivity(s) => close(s, a),
                        _ => panic!("Wrong result"),
                    }
                }
            }
        }
    }
    #[test]
    fn protein_arithmetic_range_is_checked() {
        assert!(Basis::Activity.mass(1e-300, 1e300, 1e300).is_err());
        assert!(Basis::Activity.volume(1e300, 1e300, 1e-300).is_err());
        assert!(Basis::Activity
            .concentration(1e-300, 1e-300, 1e300)
            .is_err());
    }
    #[test]
    fn automatic_volume_units_follow_magnitude() {
        for (litres, expected) in [
            (2.5, "2.5 L"),
            (0.0015, "1.5 mL"),
            (0.0005, "500 µL"),
            (4.5e-7, "450 nL"),
            (0.0, "0 L"),
        ] {
            assert_eq!(display_volume(litres), expected);
        }
    }
    #[test]
    fn automatic_mass_and_concentration_units_cover_small_and_large_values() {
        for (grams, expected) in [
            (1500.0, "1.5 kg"),
            (1.5, "1.5 g"),
            (0.0015, "1.5 mg"),
            (0.0001, "100 µg"),
            (2.5e-9, "2.5 ng"),
        ] {
            assert_eq!(display_mass(grams), expected);
        }
        for (molar, expected) in [
            (2.5, "2.5 M"),
            (0.001, "1 mM"),
            (1e-5, "10 µM"),
            (2.5e-9, "2.5 nM"),
            (1e-12, "1 pM"),
        ] {
            assert_eq!(display_concentration(molar), expected);
        }
    }
    #[test]
    fn automatic_units_respect_boundaries_and_display_precision() {
        for (litres, expected) in [
            (0.000999999, "999.999 µL"),
            (0.001, "1 mL"),
            (0.0009999999, "1 mL"),
            (0.9999999, "1 L"),
        ] {
            assert_eq!(display_volume(litres), expected);
        }
        assert_eq!(display_mass(0.0009999999), "1 mg");
        assert_eq!(display_concentration(0.0009999999), "1 mM");
    }
    #[test]
    fn automatic_display_is_independent_of_input_unit() {
        let original = [1.5, 1500.0, 1_500_000.0];
        let mut results = Vec::new();
        for (input, unit) in original.into_iter().zip(VOLUME.iter().take(3)) {
            results.push(display_volume(unit.to_base(input).unwrap()));
        }
        assert_eq!(results, ["1.5 L", "1.5 L", "1.5 L"]);
        assert_eq!(original, [1.5, 1500.0, 1_500_000.0]);
    }
    #[test]
    fn dilution_and_stock_results_choose_units_independently() {
        let d = dilute(1.0, 0.0001, 0.5).unwrap();
        assert_eq!(display_volume(d.stock), "50 µL");
        assert_eq!(display_volume(d.solvent), "499.95 mL");
        let s = stock(0.001, 0.00001, 150.0, 0.00001, 10.0, 0.0001).unwrap();
        assert_eq!(display_concentration(s.concentration), "1 mM");
        assert_eq!(display_volume(s.minimum_volume), "1.5 mL");
        assert_eq!(display_volume(s.batch_volume), "1.7 mL");
        assert_eq!(
            display_mass(mass(180.156, s.batch_volume, s.concentration).unwrap()),
            "306.265 µg"
        );
    }
    #[test]
    fn values_outside_available_prefixes_keep_their_magnitude() {
        assert_eq!(display_volume(1e-15), "1e-6 nL");
        assert_eq!(display_concentration(1e-15), "0.001 pM");
        assert_eq!(display_mass(1e12), "1e9 kg");
    }
    #[test]
    fn mixed_unit_mass() {
        let result = mass(
            180.156,
            Unit::Millilitre.to_base(100.0).unwrap(),
            Unit::Millimolar.to_base(5.0).unwrap(),
        )
        .unwrap();
        close(result / Unit::Milligram.scale(), 90.078);
    }
    #[test]
    fn dilution_balance() {
        let d = dilute(1.0, 0.01, 0.1).unwrap();
        close(d.stock, 0.001);
        close(d.solvent, 0.099);
        close(d.factor, 100.0);
        close(d.stock + d.solvent, 0.1);
    }
    #[test]
    fn equal_concentrations_need_no_solvent() {
        let d = dilute(0.1, 0.1, 0.005).unwrap();
        close(d.stock, 0.005);
        assert_eq!(d.solvent, 0.0);
    }
    #[test]
    fn rejects_concentrating_by_dilution() {
        assert!(dilute(0.01, 0.1, 1.0).is_err());
    }
    #[test]
    fn stock_uses_actual_target_concentration() {
        let s = stock(0.001, 0.00001, 24.0, 0.00001, 10.0, 0.0001).unwrap();
        close(s.concentration, 0.001);
        close(s.minimum_volume, 0.00024);
        close(s.batch_volume, 0.0003);
        close(
            mass(180.156, s.batch_volume, s.concentration).unwrap(),
            0.0000540468,
        );
    }
    #[test]
    fn stock_rounds_in_physical_units() {
        for (step, expected) in [
            (0.0, 0.000264),
            (0.00001, 0.00027),
            (0.0001, 0.0003),
            (0.001, 0.001),
            (0.01, 0.01),
        ] {
            close(
                stock(0.001, 0.00001, 24.0, 1e-5, 10.0, step)
                    .unwrap()
                    .batch_volume,
                expected,
            );
        }
    }
    #[test]
    fn exact_rounding_boundary_does_not_jump() {
        close(
            stock(1.0, 0.1, 10.0, 1.0, 10.0, 0.1).unwrap().batch_volume,
            1.1,
        );
    }
    #[test]
    fn batch_never_smaller_than_extra_volume() {
        for count in 1..500 {
            let s = stock(0.001, 0.000003, count as f64, 1e-6, 10.0, 0.0001).unwrap();
            assert!(s.batch_volume >= s.minimum_volume * 1.1);
        }
    }
    #[test]
    fn invalid_sample_counts() {
        for count in [0.0, -1.0, 1.5, f64::NAN, f64::INFINITY, 1e10] {
            assert!(stock(1.0, 0.1, count, 1.0, 10.0, 0.0).is_err());
        }
    }
    #[test]
    fn invalid_stock_quantities() {
        assert!(stock(0.1, 1.0, 1.0, 1.0, 0.0, 0.0).is_err());
        assert!(stock(1.0, 0.1, 1.0, 1.0, -1.0, 0.0).is_err());
        assert!(stock(1.0, 0.1, 1.0, 1.0, 1.0, -1.0).is_err());
    }
    #[test]
    fn rejects_nonpositive_and_nonfinite_inputs() {
        for v in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert!(mass(v, 1.0, 1.0).is_err());
            assert!(mass(1.0, v, 1.0).is_err());
            assert!(mass(1.0, 1.0, v).is_err());
            assert!(volume(1.0, v, 1.0).is_err());
            assert!(concentration(1.0, 1.0, v).is_err());
            assert!(dilute(v, 1.0, 1.0).is_err());
        }
    }
    #[test]
    fn arithmetic_range_is_checked() {
        assert!(mass(1e300, 1e300, 1e300).is_err());
        assert!(mass(1e-300, 1e-300, 1e-300).is_err());
        assert!(Unit::Microlitre.to_base(1e-320).is_err());
        assert!(volume(1e-300, 1e300, 1e-300).is_err());
    }
    #[test]
    fn decimal_comma_and_scientific_notation() {
        assert_eq!(parse(" 1,25e-3 ", "Volume").unwrap(), Some(0.00125));
        assert_eq!(parse("", "Volume").unwrap(), None);
        for text in ["1,234.5", "1 234", "hello", "NaN", "inf", "1e999"] {
            assert!(parse(text, "Volume").is_err());
        }
    }
    #[test]
    fn formatting_preserves_small_nonzero_values() {
        assert_eq!(number(0.0), "0");
        assert_eq!(number(0.5844), "0.5844");
        assert_eq!(number(1e-10), "1e-10");
        assert_eq!(number(1e8), "1e8");
        assert_eq!(number(100.0), "100");
        assert_eq!(number(0.0001), "0.0001");
    }
    #[test]
    fn preparation_boundary_ignores_conversion_noise() {
        let s = stock(0.001, 0.00001, 24.0, 0.00001, 10.0, 0.0001).unwrap();
        let from_mass = volume(
            180.156,
            0.0540468 * Unit::Milligram.scale(),
            s.concentration,
        )
        .unwrap();
        assert!(!materially_less(from_mass, s.batch_volume));
        assert!(!materially_less(
            Unit::Microlitre.to_base(300.0).unwrap(),
            s.batch_volume
        ));
        assert!(materially_less(
            Unit::Microlitre.to_base(299.9).unwrap(),
            s.batch_volume
        ));
    }
    #[test]
    fn inverse_properties_across_units() {
        for mw in [18.015, 58.44, 180.156, 66000.0] {
            for v in [1e-6, 0.001, 0.1, 1.0] {
                for c in [1e-9, 1e-6, 0.001, 1.0] {
                    let m = mass(mw, v, c).unwrap();
                    close(volume(mw, m, c).unwrap(), v);
                    close(concentration(mw, m, v).unwrap(), c);
                }
            }
        }
    }
}
