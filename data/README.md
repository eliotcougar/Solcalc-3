# Substance catalogs

The app embeds an English/chemical SQLite catalog and independent localization
SQLite catalogs. It opens only the English base and the active locale, merges
by stable substance ID, then merges user records by CAS when supplied.
Bundled catalogs are read-only and updated by replacing the executable. They
are not deployed into, imported into, or used to reseed the user's database.

## Source and generated files

| Source (real tab-separated UTF-8 TSV) | Generated read-only database |
| --- | --- |
| `source/main.en.tsv` | `bundled/main.en.sqlite` |
| `source/locales/ru.tsv` | `bundled/locales/ru.sqlite` |
| `source/locales/<locale>.tsv` | `bundled/locales/<locale>.sqlite` |

English base columns: `id`, `cas`, `formula`, `name`, `synonyms`.
Locale columns: `substance_id`, `name`, `synonyms`. Synonyms are separated by
semicolons. Locales store additional names and synonyms, never chemical masses
or formulas. Missing locale rows or names fall back to English. English names
and synonyms stay searchable; inactive locale synonyms are not loaded.

The generator computes the base molar masses, validates CAS format/checksums,
unique IDs and CAS values, locale foreign keys and codes, and database integrity.
The 131 bundled records include 52 hydrates and 328 synonyms. Hydrates and
anhydrous substances have different records and CAS values. Isomers may share a
formula and mass but retain their identities.

Run Python 3.10+ with its standard library only:

```powershell
python tools/generate-catalog.py
python tools/generate-catalog.py --check
```

The generator discovers locale TSVs and generates `bundled/locale_index.rs`,
so adding a catalog locale does not require changing the database loader.
For a future locale, add its TSV, regenerate the outputs, and add its UI
translations and language-selection code in `src/i18n.rs` / `src/settings.rs`.
Codes use lowercase language tags, for example `de` or `pt-br`. The catalog
loader accepts a locale code and falls back to English if that catalog is absent.
No Python dependency is required to build or run the app; Rust embeds the
checked-in SQLite files. The bundled schema version is 1, catalog revision 2.
The application IDs distinguish main (`0x534F4C43`), locale (`0x534F4C4C`) and user
(`0x534F4C55`) SQLite files.

## User-owned database

Path: `%LOCALAPPDATA%\Solcalc\user-substances.sqlite`.

- `user_substances`: integer identity, optional CAS, optional formula, molar mass,
  optional English name, English synonyms, and optimistic-concurrency revision.
- `user_localizations`: substance identity, locale, optional name and synonyms;
  the primary key is `(substance_id, locale)`.
- A positive finite molar mass and at least one name are required. A localized
  name alone is sufficient. When English is unavailable, an available user name
  is used as a display/search fallback in other UI languages. Formula, CAS and
  synonyms are optional. An edit cannot remove the last available name.
- English and the active locale are saved together in one transaction. Other
  locales and unknown extra columns are left intact. A stale revision cannot
  overwrite a record changed in another window/process.
- Nonblank CAS values are unique within the user file. Matching bundled CAS
  produces one combined row: user values take precedence; missing optional names
  and formulas fall back to bundled values; bundled names remain searchable.
  Blank CAS records retain separate identities, even when formulas match.
- Add creates user entries; Edit / customize edits a user entry or creates a
  user customization of a bundled entry. Bundled files are never modified.
- Delete removes a user record and all its locales in one transaction, with
  revision checking and rollback if any reference blocks removal. Deleting a
  CAS-matched customization exposes the bundled entry again. It never deletes
  bundled records. The user schema remains version 2.

The current user schema is version 2 (`PRAGMA user_version`). Schema 1 contains
the chemical/English table; schema 2 adds the revision column and localization
table. This migration is additive and transactional. Before changing an existing
schema, SQLite's online backup creates a uniquely named
`user-substances.pre-v<version>-<timestamp>-<pid>.sqlite` beside the user file.
Backups include committed WAL data and never overwrite earlier backups. A failed
backup prevents migration; a failed migration rolls back. Unknown or newer
schemas are reported without resetting the file. Reopening a current schema,
changing locales, or updating bundled catalogs does not rewrite user data.

Database connections are opened lazily when needed for the picker. Runtime
search is entirely offline. No user database, profile, or test data is included
in the distribution archive. Back up the user file separately when moving PCs.

## Chemical data provenance

Molar masses are sums of the displayed formulas, rounded to 0.001 g/mol, using
CIAAW Abridged Standard Atomic Weights 2024, retrieved 2026-11-07:
https://www.ciaaw.org/abridged-atomic-weights.htm . The exact values and formula
parser are recorded in the generator.
These are conventional formula masses; supplier rounding can differ in the last
digits. Hydrate water is included explicitly. User molar masses are entered by
the user, independently of the optional formula.

CAS fields are curated identifiers for each specified form. The build checks
syntax, check digits, and uniqueness; that check does not certify chemical
identity. Primary-reference spot checks include:

- Glucose monohydrate, 14431-43-7:
  https://www.sigmaaldrich.com/US/en/product/mm/104074
- Lactose monohydrate, 10039-26-6:
  https://www.sigmaaldrich.com/US/en/product/mm/137045
- Sodium phosphate dibasic heptahydrate, 7782-85-6:
  https://www.sigmaaldrich.com/US/en/product/aldrich/431478
- Calcium chloride hexahydrate, 7774-34-7:
  https://www.sigmaaldrich.com/US/en/substance/calciumchloridehexahydrate219087774347
- Magnesium sulfate monohydrate, 14168-73-1:
  https://www.sigmaaldrich.com/US/en/sds/ALDRICH/434183
- MES monohydrate, 145224-94-8:
  https://www.sigmaaldrich.com/sds/sigma/69892
- Aluminum sulfate octadecahydrate, 7784-31-8:
  https://www.thermofisher.com/order/catalog/product/es/en/123680010
- Raffinose pentahydrate, 17629-30-0:
  https://www.sigmaaldrich.com/US/en/product/mm/107419
- Dipotassium hydrogen phosphate trihydrate, 16788-57-1:
  https://www.sigmaaldrich.com/US/en/product/mm/105099

Names, synonyms and records are curated in this project, not represented as a
bulk import or certification by a reference provider. Variable-composition
mixtures, isotope-enriched materials and purity corrections are not part of
the bundled catalog.
