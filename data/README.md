# Substance catalogs

ChEBI is the sole bundled English chemical database. The app loads it together
with the active language's localization databases, then merges independently
stored user records. Bundled catalogs are read-only and update with the executable;
they are never deployed into, imported into, or used to reseed the user database.

## Sources and generated databases

| Source (UTF-8 TSV) | Generated read-only database |
| --- | --- |
| `source/chebi/main.en.tsv`, `source/chebi/excluded-entities.tsv`, and `source/chebi/stereoisomers.tsv` | `bundled/main.en.sqlite` |
| `source/locales/<locale>.tsv` | `bundled/locales/<locale>.sqlite` |
| `source/chebi/locales/<locale>.tsv` | `bundled/chebi-locales/<locale>.sqlite` |
| `source/solubility.tsv`, `source/solubility-reference.tsv`, `source/solubility-prediction.tsv`, and `source/solvents.tsv` | `bundled/solubility.sqlite` |

The former 131-entry English source and alias catalog have been removed. Every
bundled English name, synonym, formula, average molar mass and CAS reference now
comes from ChEBI, including substances that previously used Solcalc values.
User values still take precedence for user customizations.

## ChEBI import

The picker contains **92,287** selectable records from ChEBI release
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

The importer classifies group descendants and substituent relationships from
the ChEBI CORE ontology, and uses the reported net charge to exclude individual
ions. Neutral salts, hydrates and zwitterions are eligible. The generated
picker excludes 2,314 groups and residues and 9,486 other charged entries.
Their ChEBI source records and references are retained for provenance.

The 8,222 stereoisomer groups use single-component standard InChI. Members
have matching connectivity, composition and non-stereochemical layers.
Records without compatible structures remain individually searchable. The
picker shows the best match per group. Expanding it reveals the individual
entries and keeps each member's ChEBI ID, CAS, translation and solubility
measurements attached to that member.

The manifest records release, source URLs, input SHA-256 hashes, counts and
exclusions. To reproduce/update the import, download `compounds.tsv.gz`,
`chemical_data.tsv.gz`, `names.tsv.gz`, `database_accession.tsv.gz`, `README` and
`LICENSE` from the official flat-file directory or release archive. Add the same
release's CORE OBO export as `chebi_core.obo.gz` in the input directory, then run:

```powershell
python tools/import-chebi.py --input-dir .tools/chebi
python tools/generate-catalog.py
python tools/generate-catalog.py --check
python -m unittest discover -s tests -v
```

Normal builds/checks use committed sources and do not download data. Python 3.10+
with its standard library is sufficient for regeneration; the app needs no Python.

## Independent translations

Manual localization TSV columns are `substance_id`, `cas`, `formula`, `name`,
`synonyms`. Set `substance_id` to a ChEBI ID for a direct translation, including
entries without CAS or with isotope formulas. Leave it blank to match by CAS and
formula, as the existing Russian supplement does. Formula
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

## Solubility measurements

The solubility catalog is a separate, read-only supplement to ChEBI. Each row
identifies one chemical form by ChEBI ID and formula, and records a reference for
one solvent. Keep the temperature and pH when the source provides them. The
generator checks the ID and formula against the current ChEBI source. Add curated
rows to `source/solubility.tsv` and run the normal generation and check commands.
Curated entries take precedence over imported references for the same form and solvent.

Store the reported unit and value, plus an HTTPS link to the source. The
supported units are `g_l_solution`, `mol_l_solution`, `mass_percent`,
`g_100g_solvent`, `mol_kg_solvent` and `mole_fraction`. Use `insoluble` or
`practically_insoluble` with a blank value for an explicit qualitative finding.
Keep the distinction in the source and leave unknown temperatures blank.
A density in g/mL is optional for all four composition units. It must be for
the saturated **solution** at the same conditions. The solvent table lists
the original dataset names, SMILES and average molar masses calculated from
those SMILES using RDKit. The app uses these masses to convert reported mole
fractions to solvent-to-solute mass ratios. For hydrates, verify
whether the source reports concentrations on the hydrated or anhydrous basis.
Cite the measurement and avoid transferring values between forms or isomers.

The [BigSolDB 2.2](https://zenodo.org/records/22648301) subset by Krasnov, Malikov,
Kiseleva, Tatarin, Sosnin and Bezzubov is licensed CC BY 4.0. The importer matches
unique full InChIKeys, excludes conflicting CAS cross-references and annotated
polymorphs or hydrates, and selects one measurement per solvent nearest 25 °C
within 20–30 °C. At tied distances it selects the reported value closest to the
median on the log scale. It retains each study DOI and converts temperatures to °C.
The BigSolDB mol/L column uses pure solvent volume. Solcalc displays the
original measured mole fractions and can compare these with a recipe when
the user enters the solvent mass.

[MixtureSolDB v3](https://doi.org/10.5281/zenodo.18660057), by Malikov and
co-authors, is licensed CC BY 4.0. Its pure-solvent endpoints add measurements
for substance and solvent pairs absent from the other sources. The importer
requires an exact unique InChIKey and matching CAS, excludes disconnected
structures and annotated solid forms, and selects the measurement nearest
25 °C within 20–30 °C. It retains the original mole fraction and study DOI.

The [AqSolDBc](https://doi.org/10.57745/CZVZIA) subset by Llompart and co-authors
uses the 2024-12-04 release, licensed under the
[Open Licence 2.0](https://spdx.org/licenses/etalab-2.0.html).
It adds water references for single-constituent organic compounds with exact
identity matches, excluding flagged errors and duplicate standard deviations
greater than 0.3 log S units. Where composition metadata is blank, the importer
checks for a single connected, carbon-containing structure in the curated SMILES.
The importer converts reported log S values to mol/L and leaves individual
measurement temperatures and pH blank. Solcalc assumes 21 °C when displaying
and comparing measurements without a reported temperature.

The [2018 OPERA study supplement](https://doi.org/10.1186/s13321-018-0263-1)
from Mansouri, Grulke, Judson and Williams supplies experimental aqueous
measurements from its water-solubility training and test sets. These data were
published under CC0. The importer requires a unique full InChIKey match from
the original SMILES, a matching single CAS number, a source citation, and a
reported temperature of 20–30 °C. It skips records flagged as salts or solvents
and conflicting duplicates. Both compiled water sources include values at the
1,000,000 mg/L miscibility placeholder. The importer excludes concentrations
at or above that value, allowing for rounding. Reported log molarity is
converted to mol/L. The OPERA measurements take precedence over reference-only
water records from BigSolDB or AqSolDBc.

The curated rows cite individual studies and compilations such as IUPAC–NIST,
PubChem HSDB, FAO/WHO JECFA monographs and ILO/WHO Chemical Safety Cards.
They also cite individual supplier safety data sheets and specifications for
the exact substance and solvent. Preparation or quality-control concentrations
are excluded from saturation limits. For the lithium salts, the curated rows
use the 25 °C measurements in pure solvents. [Xin et al., table 3](https://doi.org/10.1016/j.fluid.2017.12.034)
The IUPAC–NIST rows retain the reported units, experimental temperatures,
and densities where measured for the same saturated solution. Sodium iodate
and lithium chloride use the anhydrous mass basis reported in the studies.
[Volume 30](https://srdata.nist.gov/solubility/IUPAC/SDS-30/SDS-30.aspx) · [Volume 47](https://iupac.github.io/SolubilityDataSeries/volumes/SDS-47.pdf)
For sodium iodide, the rows retain the original binary solution mole fractions
and corresponding saturated-solution densities. [Pawar et al., tables 1–3](https://doi.org/10.1021/je300754n)

Common salts use aqueous measurements at the tabulated temperature, with
saturated-solution densities where the same table lists them. The organic
solvent rows retain the listed mass of the exact salt or hydrate per 100 g of
solvent. A blank table entry supplies no solubility value.
[Martins et al., tables 1–2](https://doi.org/10.1590/1413-7054202347018722) · [Sigma-Aldrich aqueous table](https://www.sigmaaldrich.com/HK/en/support/calculators-and-apps/solubility-table-compounds-water-temperature) · [Merck LabTools, p. 63](https://www.merckmillipore.com/deepweb/assets/sigmaaldrich/product/documents/294/990/034866-merc170054-bro-labtools-lo-mk.pdf#page=63)

For calcium chloride dihydrate and hexahydrate, magnesium chloride hexahydrate,
sodium acetate trihydrate, and sodium carbonate and sulfate decahydrates, the
aqueous values express the same measured saturation point as grams of hydrate
weighed per 100 g of final solution. Multiply the dry-salt mass fraction by the
ratio of the hydrate molar mass to the dry-salt molar mass. The final solution
composition, temperature and any matching saturated-solution density are the
same on either mass basis.
[Calcium chloride, table 1](https://doi.org/10.3390/ma10070691) · [Sodium sulfate in water](https://doi.org/10.1021/je980243v)
For sodium acetate, the aqueous values use the Sigma-Aldrich saturation
composition and density. The measured mole fraction in Wu et al. independently
places the aqueous saturation near 32% dry salt at 20 °C. The ethanol and
acetic acid entries retain the experimental mole fractions from the same study.
[Wu et al., tables 2–4](https://doi.org/10.1016/j.cjche.2021.06.029)

The water entries for sodium carbonate and magnesium chloride use the dry-salt
saturation mass fraction and matching liquid density from the Sigma-Aldrich
table. The hydrates use that same density with the hydrate mass basis.
[Sigma-Aldrich aqueous table](https://www.sigmaaldrich.com/HK/en/support/calculators-and-apps/solubility-table-compounds-water-temperature)

The 20 °C densities for potassium iodide, sodium bromide, magnesium nitrate,
ammonium sulfate, calcium chloride and its hydrates come from Laliberté's
aqueous-electrolyte correlation evaluated at the tabulated dry-salt mass
fraction. The lithium bromide density uses Khairulin et al., equations 2–3
and table 2, at 62.668% and 20 °C. The potassium carbonate density uses the
supplier's specific gravity for the solution saturated at 20 °C, rounded to
two decimal places.
[Laliberté](https://doi.org/10.1021/je8008123) · [Khairulin et al.](https://www.sibran.ru/upload/iblock/19a/19a8e3a311256ca9085cebd5a5dc5dfb.pdf) · [Reagents potassium carbonate specification](https://www.fishersci.com/shop/products/potassium-carbonate-saturated-at-20-c-reagents/01334202)

Ammonium dihydrogen phosphate, diammonium hydrogen phosphate, lithium carbonate
and ammonium bromide use concentration and density at the same saturation point.
The ammonium bromide entry converts Eder's specific gravity at the 15 °C
saturation composition to density. Potassium cyanide and mercury(II) bromide
use published saturation concentrations per litre of final solution.
[Ammonium dihydrogen phosphate, tables S1–S2](https://doi.org/10.1021/acsomega.2c04534) · [Diammonium hydrogen phosphate, ThermoML](https://trc.nist.gov/ThermoML/10.1016/j.fluid.2018.07.016.html) · [Lithium carbonate, table 1 at zero added sodium carbonate](https://doi.org/10.11949/0438-1157.20190647) · [Ammonium bromide, Eder and Gerlach](https://archive.org/details/chemicalsolu00comerich) · [Potassium cyanide, table 1](https://doi.org/10.1039/DT9800000001) · [Mercury(II) bromide, table 12](https://srd.nist.gov/JPCRD/jpcrd274.pdf)

The sodium metabisulfite specification gives 39.6% by mass at 20 °C.
That is equivalent to 43.35% sodium bisulfite by mass in water, using
Na₂S₂O₅ + H₂O → 2 NaHSO₃. The catalog uses 1.35 g/mL as an approximate density
from the supplier's 1.34–1.37 specific gravity range for 42–44% aqueous
sodium bisulfite at 70 °F.
[Metabisulfite saturation table](https://www.tennantsdistribution.com/LinkClick.aspx?fileticket=9GE84Iq9UUE%3D&portalid=0) · [Dimmitt solution specification](https://dimmittsulfur.wordpress.com/wp-content/uploads/2015/10/sodium-bisulphote.pdf)

The broader water audit used the following sources for saturation-density
conversions. The density figures are in g/mL.

- At 25 °C, Laliberté's aqueous-electrolyte correlation uses the existing
  saturation compositions for ammonium chloride, sodium nitrate, potassium
  dihydrogen phosphate and potassium nitrate. Each composition is within the
  published fit range. [Laliberté](https://doi.org/10.1021/je8008123)
- Cesium chloride uses approximately 1.92 at 25 °C, from IUPAC's measured
  densities near 65.7% by mass. Rubidium chloride uses 1.500 at 25 °C from
  interpolation of IUPAC's saturation densities at 20 and 30 °C.
  [Cesium chloride, table 2](https://iupac.github.io/SolubilityDataSeries/volumes/SDS-47.pdf#page=407) ·
  [Rubidium chloride, table 4](https://iupac.github.io/SolubilityDataSeries/volumes/SDS-47.pdf#page=378)
- Sodium hydroxide uses the 52% density at 20 °C from Olin's table 4a.
  Potassium hydroxide uses the nearest reported composition, 55% at 25 °C,
  for the 54.75% saturation entry. Sodium chlorate uses approximately 1.43
  from the 25 °C, 1.005 g/g water point on Environment Canada's density curve.
  [Olin](https://olinchloralkali.com/wp-content/uploads/sites/5/2025/11/102-00539-1122-Olin-Sodium-Hydroxide-Product-Stewardship-Manual-2025.pdf) ·
  [Allebrod et al., table 1](https://doi.org/10.1016/j.ijhydene.2012.02.088) ·
  [Environment Canada, figure 2](https://publications.gc.ca/collections/collection_2019/eccc/En48-10-37-1985-eng.pdf#page=17)
- D-fructose uses 1.406 from interpolation to 80.6% and 25 °C in the ICUMSA
  table, which extrapolates its density fit above 70% by mass. Ammonium
  thiocyanate uses approximately 1.15 from the 23.5 °C density fit, extrapolated
  from the highest measured composition of 62.6% to the catalog's 64.2% at
  21.85 °C.
  [ICUMSA SPS-4, table D](https://www.icumsa.org/methods/icumsa-specification-and-standard-sps-4-1998/) ·
  [Borrero-Echeverry and Case, equation 2](https://arxiv.org/abs/1605.07562)
- Fluphenazine decanoate, etoricoxib and amiodarone hydrochloride use measured
  saturated-solution densities archived by NIST. Glycine uses the paired 25 °C
  solubility and density from Dalton and Schmidt.
  [Fluphenazine decanoate](https://trc.nist.gov/ThermoML/10.1016/j.fluid.2014.01.044.html) ·
  [Etoricoxib](https://trc.nist.gov/ThermoML/10.1021/acs.jced.7b00709.html) ·
  [Amiodarone hydrochloride](https://trc.nist.gov/ThermoML/10.1021/je201268b.html) ·
  [Glycine, table V](https://doi.org/10.1016/S0021-9258(18)75234-4)

Eight further overrides use source-reported saturation concentrations per litre
of solution. For octogen (HMX), the EPA's 5 mg/L at 25 °C supersedes a
conflicting imported mole fraction. The other entries are succinic acid,
sucralose, p-cymene, enrofloxacin, metronidazole, D-mannitol and myo-inositol.
Each row links to its volumetric source.

To refresh the imported subset, install RDKit in a Python environment and use
the BigSolDB 2.2 CSV, AqSolDBc TSV export, the original OPERA S1 ZIP
(`13321_2018_263_MOESM1_ESM.zip`), and ChEBI Core OBO for the catalog release.
Include the MixtureSolDB v3 CSV as well. RDKit is used only for this import.
Build-time checks use the committed TSV files.

```powershell
python tools/import_solubility.py --bigsol .tools/BigSolDBv2.2.csv --aqsol .tools/AqSolDBc.tab --opera .tools/13321_2018_263_MOESM1_ESM.zip --chebi-obo .tools/chebi_core.obo.gz --mixturesol .tools/MixtureSolDB.csv
python tools/predict_solubility.py .tools/chebi_core.obo.gz
python tools/generate-catalog.py
python tools/generate-catalog.py --check
```

The prediction generator uses Delaney's ESOL equation with RDKit 2026.09.2
descriptors and structures from the matching ChEBI release. It screens for
single-component organic structures with no formal charges, isotopes or
radicals, with 100–600 g/mol molecular weight and a calculated logP of −2 to 6.
These are screening bounds for Solcalc. The build discards predictions
whenever a water measurement or qualitative finding exists. [Delaney, 2004](https://doi.org/10.1021/ci034243x)

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
