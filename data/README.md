# Substance catalogs

ChEBI is the sole bundled English chemical database. The app loads it together
with the active language's localization databases, then merges independently
stored user records. Bundled catalogs are read-only and update with the executable;
they are never deployed into, imported into, or used to reseed the user database.

## Sources and generated databases

| Source (UTF-8 TSV) | Generated read-only database |
| --- | --- |
| `source/chebi/main.en.tsv` | `bundled/main.en.sqlite` |
| `source/locales/<locale>.tsv` | `bundled/locales/<locale>.sqlite` |
| `source/chebi/locales/<locale>.tsv` | `bundled/chebi-locales/<locale>.sqlite` |

The former 131-entry English source and alias catalog have been removed. Every
bundled English name, synonym, formula, average molar mass and CAS reference now
comes from ChEBI, including substances that previously used Solcalc values.
User values still take precedence for user customizations.

## ChEBI import

The catalog contains **104,087** fixed-composition substances from ChEBI release
**257**, dated **2026-11-05**. Source: https://www.ebi.ac.uk/chebi ; license:
CC BY 4.0. The full license is in `source/chebi/LICENSE.txt`. This is a filtered,
normalized and language-separated subset of ChEBI.

The English TSV columns are `chebi_id`, preferred `cas`, JSON-array `cas_numbers`,
`formula`, `name`, `molar_mass`, and JSON-array `synonyms`. Stable ChEBI IDs identify
entries; CAS cross-references may be absent or shared between entities. Multiple
references remain searchable. Source masses are positive finite **average masses**,
never monoisotopic masses or locally recalculated formula masses.

The importer accepts CHECKED/OK entries with fixed formulas. It excludes submitted
and deprecated entries, missing masses, repeat units, R groups, wildcards and
undefined stoichiometry. Invalid CAS checksums are ignored. Names and translations
use ChEBI's language codes and remain separate. This release supplies German,
Spanish, French and Latin names; EN/RU remain the available UI languages.

The manifest records release, source URLs, input SHA-256 hashes, counts and
exclusions. To reproduce/update the import, download `compounds.tsv.gz`,
`chemical_data.tsv.gz`, `names.tsv.gz`, `database_accession.tsv.gz`, `README` and
`LICENSE` from the official flat-file directory or release archive, then run:

```powershell
python tools/import-chebi.py --input-dir .tools/chebi
python tools/generate-catalog.py
python tools/generate-catalog.py --check
python -m unittest discover -s tests -v
```

Normal builds/checks use committed sources and do not download data. Python 3.10+
with its standard library is sufficient for regeneration; the app needs no Python.

## Independent translations

Manual localization TSV columns are `cas`, `formula`, `name`, `synonyms`. Formula
identifies chemical composition only; locale files do not supply English names or
masses. The generator matches CAS **and** atom counts, including hydrate water,
to ChEBI entities. A formula mismatch never attaches a translation to another form.
Generated locale databases reference ChEBI IDs. ChEBI's own locale TSVs already
use these IDs in `substance_id`, alongside `name` and JSON-array `synonyms`.

All 131 existing Russian names and synonym lists remain in `source/locales/ru.tsv`.
106 match 114 ChEBI entities. The remaining 25 translations stay in source for future
catalog updates; they do not create extra chemical entries or replace ChEBI data.
Manual localized names take precedence over additional ChEBI locale names, which
remain searchable. English and active-locale names/synonyms are combined at runtime;
inactive locales are not loaded. Missing translations fall back to English.

The generator discovers locale TSVs and writes `bundled/locale_index.rs`. For a new
locale, add a TSV, regenerate, and add its UI translations/language selector in
`src/i18n.rs` and `src/settings.rs`. Codes are lowercase language tags such as `de`
or `pt-br`. Unknown locales fall back to English. Bundled schema version is 1;
application IDs distinguish main (`0x534F4C43`), locale (`0x534F4C4C`) and user
(`0x534F4C55`) databases.

CAS matching overlays a user record only when it unambiguously identifies one
bundled entity. Ambiguous references retain standalone user records and values.
Multiple user records using different aliases of one bundled entity also remain
independently searchable and editable.

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
search of bundled and user records is entirely offline. No user database, profile, or test data is included
in the distribution archive. Back up the user file separately when moving PCs.

## Optional PubChem results

PubChem is an explicit online lookup, not a bundled-data source or automatic
catalog updater. Name/CAS queries fetch up to ten candidates, their formulas,
average `MolecularWeight`, names and synonyms. A CID tag is retained for provenance.
Ambiguous CAS lists are shown without choosing an arbitrary identifier. The user
reviews a result in the editor before Save; duplicate user CAS values are rejected.
The user schema remains version 2, with no migration needed for this feature.

The worker bounds request frequency, timeouts, result counts and response size.
Rustls verifies HTTPS with current WebPKI roots. Requests are never made during
startup or ordinary offline search. Live availability depends on the service and
network; errors do not change the bundled or user databases.
