use crate::settings::Language;
use std::borrow::Cow;

const STRINGS: &[(&str, &str)] = &[
    ("Enter at least one name.", "Введите хотя бы одно название."),
    ("Enter molar mass.", "Введите молярную массу."),
    (
        "Enter a valid CAS number, for example 7647-14-5, or leave it blank.",
        "Введите корректный номер CAS, например 7647-14-5, или оставьте поле пустым.",
    ),
    ("Invalid locale code.", "Некорректный код языка."),
    (
        "A name, formula or synonym is too long or contains control characters.",
        "Название, формула или синоним слишком длинные или содержат управляющие символы.",
    ),
    (
        "A user entry with this CAS number already exists. Edit that entry instead.",
        "В базе пользователя уже есть запись с этим номером CAS. Откройте её для изменения.",
    ),
    (
        "This entry changed in another window. Reopen it before saving.",
        "Эта запись изменена в другом окне. Откройте её заново перед сохранением.",
    ),
    (
        "This entry changed in another window. Reopen it before deleting.",
        "Эта запись изменена в другом окне. Откройте её заново перед удалением.",
    ),
    (
        "This user entry no longer exists. Reopen the catalog.",
        "Эта запись пользователя больше не существует. Откройте каталог заново.",
    ),
    (
        "The user database was created by a newer Solcalc version. It was not changed.",
        "База пользователя создана более новой версией Solcalc. Она не изменена.",
    ),
    (
        "This file is not a Solcalc user database. It was not changed.",
        "Этот файл не является базой пользователя Solcalc. Он не изменён.",
    ),
    (
        "Unrecognized user database structure. It was not changed.",
        "Структура базы пользователя не распознана. Она не изменена.",
    ),
    ("&Substance…", "&Вещество…"),
    (
        "Solcalc — Solution Calculator",
        "Solcalc — Калькулятор растворов",
    ),
    ("&1  Mass / volume", "&1  Масса / объём"),
    ("&2  Dilution", "&2  Разведение"),
    ("&3  Stock preparation", "&3  Приготовление"),
    ("&Molar mass", "&Молярная масса"),
    ("Molar &mass", "Молярная &масса"),
    ("&Specific activity", "Удельная &активность"),
    ("Specific activity", "Удельная активность"),
    ("Concentration basis", "Тип концентрации"),
    (
        "Required protein mass\n\nm = C × V / SA",
        "Масса белка\n\nm = C × V / SA",
    ),
    (
        "Required final volume\n\nV = m × SA / C",
        "Конечный объём\n\nV = m × SA / C",
    ),
    (
        "Activity concentration\n\nC = m × SA / V",
        "Концентрация активности\n\nC = m × SA / V",
    ),
    (
        "Specific activity\n\nSA = C × V / m",
        "Удельная активность\n\nSA = C × V / m",
    ),
    ("&Concentration", "&Концентрация"),
    ("&Volume", "&Объём"),
    ("&Mass", "М&асса"),
    ("M&ass", "М&асса"),
    ("&Stock concentration", "Концентрация &стока"),
    ("&Desired concentration", "&Нужная концентрация"),
    ("&Final volume", "&Конечный объём"),
    ("Final sample &volume", "&Объём образца"),
    ("Stock &per sample", "&Сток на образец"),
    ("&Number of samples", "&Число образцов"),
    ("Final &concentration", "Конечная &концентрация"),
    ("&Extra volume", "&Доп. объём"),
    ("Round batch up to", "Округление вверх"),
    ("Preparation", "Приготовление"),
    ("&Batch volume", "&Объём стока"),
    ("RESULT", "РЕЗУЛЬТАТ"),
    ("C&lear inputs", "О&чистить поля"),
    ("Light", "Светлая"),
    ("Dark", "Тёмная"),
    ("System", "Системная"),
    ("No rounding", "Без округления"),
    (
        "Required solute mass\n\nm = MW × C × V",
        "Масса вещества\n\nm = MW × C × V",
    ),
    (
        "Required final volume\n\nV = m / (MW × C)",
        "Конечный объём\n\nV = m / (MW × C)",
    ),
    (
        "Concentration\n\nC = m / (MW × V)",
        "Концентрация\n\nC = m / (MW × V)",
    ),
    (
        "Molar mass\n\nMW = m / (C × V)",
        "Молярная масса\n\nMW = m / (C × V)",
    ),
    ("Molar mass", "Молярная масса"),
    ("Concentration", "Концентрация"),
    ("Volume", "Объём"),
    ("Mass", "Масса"),
    ("Stock concentration", "Концентрация стока"),
    ("Desired concentration", "Нужная концентрация"),
    ("Final volume", "Конечный объём"),
    ("Final sample volume", "Объём образца"),
    ("Stock per sample", "Сток на образец"),
    ("Number of samples", "Число образцов"),
    ("Final concentration", "Конечная концентрация"),
    ("Extra volume", "Дополнительный объём"),
    ("Batch volume", "Объём стока"),
    ("Available mass", "Имеющаяся масса"),
    ("Planned volume", "Планируемый объём"),
    ("Converted value", "Значение в базовых единицах"),
    ("Result", "Результат"),
    ("Leave one field blank.", "Оставьте одно поле пустым."),
    (
        "Leave mass or batch volume blank.",
        "Оставьте поле массы или объёма стока пустым.",
    ),
    (
        "Result is outside the supported numeric range. Check the values and units.",
        "Результат вне допустимого числового диапазона. Проверьте значения и единицы.",
    ),
    (
        "Desired concentration cannot exceed stock concentration.",
        "Нужная концентрация не может превышать концентрацию стока.",
    ),
    (
        "Stock per sample cannot exceed the final sample volume.",
        "Объём стока на образец не может превышать конечный объём образца.",
    ),
    (
        "Number of samples must be a whole number from 1 to 1,000,000,000.",
        "Число образцов должно быть целым от 1 до 1 000 000 000.",
    ),
    (
        "Extra volume must be zero or greater.",
        "Дополнительный объём не может быть отрицательным.",
    ),
    (
        "Invalid rounding increment.",
        "Недопустимый шаг округления.",
    ),
    (
        "Batch volume is smaller than the planned stock volume.",
        "Объём стока меньше запланированного.",
    ),
];
pub fn text(language: Language, en: &str) -> &str {
    if language == Language::En {
        return en;
    }
    STRINGS
        .iter()
        .find(|(key, _)| *key == en)
        .map(|(_, ru)| *ru)
        .unwrap_or(en)
}
pub fn error(language: Language, en: &str) -> String {
    let translated = text(language, en);
    if language == Language::En || translated != en {
        return translated.to_owned();
    }
    for (suffix, ru) in [
        (" must be a finite number.", "должно быть конечным числом"),
        (
            " must be greater than zero and within range.",
            "должно быть больше нуля и в допустимом диапазоне",
        ),
    ] {
        if let Some(label) = en.strip_suffix(suffix) {
            return format!("{}: значение {}.", text(language, label), ru);
        }
    }
    if let Some(label) = en
        .strip_prefix("Enter a number for ")
        .and_then(|s| s.strip_suffix('.'))
    {
        let key = STRINGS
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(label))
            .map(|(key, _)| *key)
            .unwrap_or(label);
        return format!("{}: введите число.", text(language, key));
    }
    en.to_owned()
}
/// Localize decimal separators in displayed numbers, never edit the inputs.
pub fn numbers(language: Language, input: &str) -> Cow<'_, str> {
    if language == Language::En {
        return Cow::Borrowed(input);
    }
    let chars: Vec<_> = input.chars().collect();
    Cow::Owned(
        chars
            .iter()
            .enumerate()
            .map(|(i, &c)| {
                if c == '.'
                    && i > 0
                    && i + 1 < chars.len()
                    && chars[i - 1].is_ascii_digit()
                    && chars[i + 1].is_ascii_digit()
                {
                    ','
                } else {
                    c
                }
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_keys_are_unique_and_translated() {
        let mut keys = std::collections::HashSet::new();
        for (en, ru) in STRINGS {
            assert!(keys.insert(*en), "{en}");
            assert!(!ru.is_empty());
            assert_eq!(text(Language::En, en), *en);
            assert_eq!(text(Language::Ru, en), *ru);
        }
    }
    #[test]
    fn validation_messages_are_localized() {
        for en in [
            "Molar mass must be a finite number.",
            "Specific activity must be greater than zero and within range.",
            "Enter a number for specific activity.",
            "Volume must be greater than zero and within range.",
            "Enter a number for mass.",
            "Leave one field blank.",
        ] {
            assert!(error(Language::Ru, en)
                .chars()
                .any(|c| ('А'..='я').contains(&c)));
        }
    }
    #[test]
    fn decimals_are_localized_without_changing_prose_or_si_units() {
        assert_eq!(
            numbers(Language::Ru, "100.5 µg in 1.7 mL. MW × C × V"),
            "100,5 µg in 1,7 mL. MW × C × V"
        );
        assert_eq!(numbers(Language::En, "1.7 mL"), "1.7 mL");
    }
}
