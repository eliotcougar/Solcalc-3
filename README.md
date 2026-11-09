# Solcalc 3

[![Checks](https://github.com/eliotcougar/Solcalc-3/actions/workflows/checks.yml/badge.svg)](https://github.com/eliotcougar/Solcalc-3/actions/workflows/checks.yml)
[![CodeQL](https://github.com/eliotcougar/Solcalc-3/actions/workflows/codeql.yml/badge.svg)](https://github.com/eliotcougar/Solcalc-3/actions/workflows/codeql.yml)
[![Release](https://img.shields.io/github/v/release/eliotcougar/Solcalc-3)](https://github.com/eliotcougar/Solcalc-3/releases/latest)

**A small, native Windows calculator for routine solution preparation.**

Calculate solute mass, solution volume, concentration, dilutions and stock batches.
Solcalc is written in Rust with a Win32 interface, starts directly from its
executable. Calculations and bundled search work offline; PubChem lookup is optional.
It needs no Python installation, browser
runtime or self-extraction step.

## Download

**[Get the latest release](https://github.com/eliotcougar/Solcalc-3/releases/latest)** · Windows 10/11, x64

The development version **3.6.0** adds the ChEBI expansion and optional PubChem
lookup described below. The packaged release **3.5.1** contains the original
131-entry catalog.

| Package | Use it when… |
| --- | --- |
| [Installer — 3.5.1](https://github.com/eliotcougar/Solcalc-3/releases/download/v3.5.1/Solcalc-3.5.1-x64-setup.exe) | You want Start menu integration and a choice of installation for yourself or all users. Only all-users installation needs administrator permission. |
| [Portable ZIP — 3.5.1](https://github.com/eliotcougar/Solcalc-3/releases/download/v3.5.1/Solcalc-3.5.1-windows-x64.zip) | You want to extract a folder and run `Solcalc.exe` without installing. |

[SHA-256 checksums](https://github.com/eliotcougar/Solcalc-3/releases/download/v3.5.1/SHA256SUMS.txt)
are included with each release. Both packages run the same native executable.

## Three calculators

| Calculator | What it does |
| --- | --- |
| **Mass / volume** | Solve mass, volume, concentration or molar mass/specific activity by leaving that field blank. |
| **Dilution** | Calculate stock and solvent volumes from the initial concentration, target concentration and final volume. |
| **Stock preparation** | Plan stock for multiple samples, including extra volume and batch rounding, then calculate the preparation mass or volume. |

Results update as you type and automatically choose convenient display units.
For example, a small volume can display in µL while a larger one displays in mL.
Decimal commas and scientific notation are accepted.

For a quick mass calculation, enter **58.44 g/mol**, **100 mM**, and **100 mL**,
leaving mass blank: the result is **584.4 mg**. Volume means final solution volume.

Protein solutions use **U/mg** in place of g/mol, with **U/mL** and **kU/mL**
concentration units. Switching the basis changes the available concentration
selectors and preserves separate inputs for each basis.

## Find substances, keep your own

The development catalog contains **104,099 substances**: our original **131 common
reagents**, including separate hydrated forms, plus **103,968 ChEBI substances**.
Original names, formulas, molar masses and Russian translations take precedence.
Compatible ChEBI aliases enrich existing entries. Search by
name, synonym, formula or CAS number. English and chemical notation are always
available; the selected language adds its localized names and synonyms.
Large result lists show the first 300 matches; refine the query to find a
particular substance. ChEBI IDs such as `CHEBI:27732` are searchable too.

- Search `7647-14-5`, `NaCl`, or a familiar name such as `Epsom salt`.
- Selecting a substance fills its molar mass in both Mass / volume and Stock
  preparation; their other calculation inputs remain independent.
- Add your own substances with just **one name and molar mass**. Formula, CAS and
  synonyms are optional. Edit or delete user entries from the same picker.
- A user customization of a bundled CAS appears as one combined entry. Deleting
  that customization restores the bundled entry.

Bundled catalogs update with the application. Your database stays separate;
changes to its structure use transactional migrations and a backup.
[Catalog sources, schemas and chemical-data provenance →](data/README.md)

For something missing locally, choose **PubChem…** in the picker. Search by name
or CAS, inspect the formula, molar mass and identifiers, then choose **Import…**.
The normal editor lets you review and save a user entry before applying it.
An existing user CAS cannot be overwritten by an import. Requests run in the
background; editing the query or closing the dialog cancels that search.
Network errors leave the offline catalog and saved data available.

## Appearance and data

Choose **Light / Dark / System** appearance and **EN / RU** language from the
upper-right dropdowns. Both settings are remembered. System mode follows Windows'
app theme, and Russian results use decimal commas.

Each Windows user keeps a separate profile, including with an all-users install:

```text
%LOCALAPPDATA%\Solcalc\
  settings.ini
  user-substances.sqlite
```

Updates and uninstall preserve this folder and any migration backups. Copy it
when moving your settings and substances to another computer. The portable app
also uses this profile folder; it does not store personal data beside its EXE.
Calculation inputs and selected substances are kept for the current session.

**Ctrl+1 / Ctrl+2 / Ctrl+3** switch calculators. **Ctrl+F** focuses search in the
substance picker; **Enter** applies a selection and **Escape** closes the picker.
Read the [user guide](docs/user-guide.md) for unit details and calculation behavior.

## Build and contribute

Use the pinned Rust toolchain, MSVC C++ build tools, and a Windows SDK:

```powershell
.\build.ps1 -Check
```

Build distribution packages from the same executable:

```powershell
.\tools\bootstrap-installer.ps1
.\build-installer.ps1 -SkipBuild
.\build-portable.ps1 -SkipBuild
.\tools\write-checksums.ps1
```

See [build instructions](docs/building.md), [tests and CI](docs/testing.md), and
[contribution rules](CONTRIBUTING.md). CI runs Windows tests, catalog validation,
dependency review and CodeQL. The owner can push directly; contributor changes
use checked pull requests. Force pushes and deletion of the default branch are
blocked for everyone.

The original Python application is preserved in [solcalc2](solcalc2/).
Third-party dependencies and their licenses are listed in
[THIRD-PARTY-NOTICES.txt](THIRD-PARTY-NOTICES.txt).
