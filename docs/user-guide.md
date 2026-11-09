# Solcalc user guide

A native Windows solution calculator, rebuilt in Rust from `solcalc2/solcalc2.py`.

Download the installer or portable ZIP from the
[releases page](https://github.com/eliotcougar/Solcalc-3/releases/latest).
Setup offers **current user**
or **all users**; only all-users installation requires administrator permission.
It includes English and Russian installer UI, a Start menu shortcut, and an
optional desktop shortcut. Updates and uninstall preserve each user's database,
settings, and migration backups under `%LOCALAPPDATA%\Solcalc`.

Run **`Solcalc.exe`** from the installation directory or extracted ZIP.
The executable is portable: it needs no installation,
Python, browser runtime, extra DLLs, or extraction step. It targets 64-bit
Windows 10 and Windows 11. The original Python source and icon are preserved.

## Appearance and language

The top-right dropdowns select **Light / Dark / System** appearance and
**EN / RU** language. System follows the Windows app-theme setting and reacts
to theme notifications while the app is open. Switching language translates
labels, results, and validation without changing calculation inputs. Russian
results use decimal commas; SI unit symbols stay international.

Both preferences are saved as soon as they change to
`%LOCALAPPDATA%\Solcalc\settings.ini` and restored on the next launch.
Missing or invalid settings use System appearance and English.

The title-bar icon has dedicated 16, 20, 24, 32, 40, 48, and 64 pixel frames,
selected for the current display DPI. Its small artwork is simplified from
the original beaker design. Rebuild the icon with `tools/generate-icon.ps1`
if ImageMagick is installed; ordinary builds use the checked-in `.ico` file.

## Calculators

- **Mass / volume:** leave exactly one field blank to calculate mass, volume,
  concentration, or molar mass / specific activity. All fields remain editable. Multiple blanks clear the result;
  filling every field asks you to leave one blank.
- **Dilution:** enter stock concentration, desired concentration, and final
  volume. The result gives stock volume, solvent volume, and dilution factor.
- **Stock preparation:** enter the final volume of **one sample** (including
  the added stock), stock volume added per sample, sample count, and desired
  final concentration. Set extra volume (10% by default) and rounding increment.
  In the preparation section, leave mass blank to calculate
  it from batch volume, or leave batch volume blank to calculate it from mass.
  If both are blank, preparation uses the planned batch volume.

Each calculator supports two concentration bases:

- **g/mol:** the coefficient is molar mass; concentration selectors offer
  M, mM, µM, nM, and pM.
- **U/mg:** the coefficient is protein specific activity; concentration
  selectors offer only **U/mL** and **kU/mL**. The mass and volume selectors
  retain their physical units. 1 kU/mL equals 1000 U/mL.

The mass/volume and stock calculators put this choice beside the coefficient.
Dilution has a concentration-basis dropdown. Each calculator keeps separate
inputs and unit selections for its two bases during the session; changing
the basis restores that form instead of treating molar values as activity.
Language switching preserves the active basis and its unit choices.

For activity calculations, m = C × V / SA, with m in mg and V in mL
(or m in g and V in L), C in U/mL, and SA in U/mg. The inverse calculations can
also solve volume, concentration, or specific activity. For example,
100 U/mg, 10 U/mL, and 100 mL require 10 mg. Stock preparation uses the
calculated stock activity concentration with the supplied specific activity.

Results update as you type. Each calculator retains its own inputs and units
while switching modes during a session. Inputs are not saved after closing.
Changing an input unit keeps the entered number and changes its interpretation.

Every result automatically chooses its own readable units, independently of
input selectors: for example, 1500 µL displays as 1.5 mL, 500 µL remains 500 µL,
and 0.1 mg displays as 100 µg. The largest available unit giving a displayed
number of at least 1 is used; outside the available prefix range, scientific
notation or fractional values preserve the magnitude. Molar mass uses g/mol.
Display selection does not change entered values or stock rounding increments.

Supported units: L / mL / µL / nL, kg / g / mg / µg / ng,
M / mM / µM / nM / pM. Both `0.25` and `0,25` are
accepted, as is scientific notation (`2.5e-3`). Do not use thousands separators.
Results display approximately six significant figures; calculations use f64.
Use final solution volume, rather than solvent volume, in preparation formulas.

Activity concentrations automatically display in U/mL or kU/mL, independently
of the chosen input prefix. Molar mass stays in g/mol and specific activity
stays in U/mg.

## Substance database

Click **Substance… / Вещество…** in Mass / volume or in the Preparation section
of Stock preparation. The development catalog contains **104,087 ChEBI substances**.
ChEBI supplies all bundled English names, formulas, average molar masses,
synonyms and CAS cross-references where supplied. Missing translations fall
back to English; new names are not automatically translated.

Search updates as you type across English plus the selected UI language,
common aliases such as `Epsom salt` / `медный купорос` (RU), CAS numbers such as
`7647-14-5`, and formulas such as `MgSO4.7H2O` or `MgSO₄·7H₂O`.
It ignores case, punctuation, and formula subscripts and supports partial words.
Exact formula matches appear first. Isomers with the same formula remain
separate choices. Select a row, then use Enter, double-click, or **Use substance**.
Down from the search field moves to the results, Ctrl+F returns to search,
and Escape cancels. Large searches show the first 300 matches; refine the query
to narrow them. Search also accepts ChEBI identifiers such as `CHEBI:27732`.

Selection is shared between Mass / volume and Stock preparation: choosing a
substance in either calculator fills g/mol and displays its name and hydrate
formula in both. Their other molar inputs stay independent and intact. Any
calculator using U/mg restores its molar form first; its activity form is
retained separately. Manual changes to molar mass clear that calculator's
substance label. Selection is not saved after closing. Theme and language
also apply to the picker.

The details pane wraps long contents, supports vertical scrolling and copying,
and grows when you resize or maximize the picker.

The bundled catalog consists of a ChEBI English/chemical SQLite database,
with independent locale databases. Only English and the
selected locale are opened and combined. Missing translations fall back to English.
The files are embedded in the executable and replaced with app updates;
they are never copied over user records. Bundled molar masses are ChEBI's
supplied average masses. Russian names/synonyms are matched by CAS and chemical
composition; translations with no matching ChEBI form are retained in the locale
source for future catalog updates.

**Add… / Добавить…** opens the user-substance editor. Only a molar mass and
at least one name are required: the English name or a name in the active
locale. Formula, CAS number and synonyms are optional. Localized-only entries
use their available name as fallback in other UI languages. **Edit / customize…**
edits a user entry or creates a user customization of a bundled one. Editing
English fields leaves stored translations intact; editing a locale changes
only that locale. Synonyms use semicolon separators.

**Delete / Удалить** is enabled for user entries. Confirming it removes that
entry and all its localized names and synonyms together. Deleting a user
customization restores the bundled entry. Bundled substances cannot be deleted.
The Delete key works when the result list has focus; the confirmation defaults
to No. A record changed in another window must be reopened before deletion.

User records are stored in `%LOCALAPPDATA%\Solcalc\user-substances.sqlite`
and searched alongside bundled records. Rows are marked as bundled or user
data. A user entry with the same CAS as a bundled entry is shown once: user
values take precedence, and bundled names and synonyms remain searchable.
Missing optional user names/formulas use available bundled values on CAS match.
Blank CAS values remain separate entries. CAS format and checksum are checked;
a second user record with the same nonblank CAS is rejected.

The user file survives executable upgrades. Schema changes use additive,
transactional migrations with a separate SQLite backup before migrating an
existing database. Existing records, translations, and unknown extra columns
are retained. Failed migrations roll back; unrecognized or newer schemas are
reported and kept intact. Concurrent edits are checked before saving. Opening
an up-to-date user database or switching languages does not rewrite it.
Bundled and user database access is offline and deferred until the catalog is needed.

### Optional PubChem lookup

Choose **PubChem…** to open online search. If the local search field contains a
query, it is searched when this online window opens. Otherwise, enter a name or
CAS and press **Search** or Enter. Requests occur only in this window, run in a
background worker, and return up to ten suitable compounds. Changing the query
clears stale results; closing the window cancels its search.

Review the exact formula and hydrate/salt form. Use **Import…** to open the usual
user editor, then **Save** to store the result. Fetching or cancelling does not
write results to the user database. A duplicate user CAS is rejected; existing
user values and translations are not overwritten. Multiple CAS synonyms are
shown without automatically selecting an arbitrary CAS. The PubChem CID is
retained in the saved synonyms for provenance and search.

The client uses average `MolecularWeight`, not `ExactMass` or monoisotopic mass.
It validates names/masses, limits response sizes and request frequency, verifies
HTTPS certificates, and reports connection/service errors. Offline calculations
and search remain available without a network connection.

See [`data/README.md`](../data/README.md) for the separate TSV sources, schemas,
locale-extension instructions, and data provenance.

**Ctrl+1 / Ctrl+2 / Ctrl+3** switch calculators. Tab and Shift+Tab navigate
native controls; Alt shortcuts are shown by Windows.
**Clear inputs** clears its calculation fields while keeping units and batch
planning settings.

## Corrections to the Python version

- Final stock concentration is an actual input; the hidden 1 M assumption is gone.
- Stock rounding uses the selected physical increment consistently.
- Fractional sample counts, negative values, non-finite values, impossible
  dilutions, and stock additions larger than a sample are rejected.
- Optional preparation automatically uses the calculated stock concentration.
- State belongs to each mode independently and is not overwritten during a switch.
- Small results use scientific notation instead of being rounded to zero.

## Build

Requires Rust 1.89 or newer with the `x86_64-pc-windows-msvc` target, MSVC build
tools, and a Windows SDK containing `rc.exe`. With the tools available:

```powershell
.\build.ps1 -Check
```

This runs formatting, Clippy, calculation tests, and a release build, then copies
the executable into `dist`. Close the running release before rebuilding it.
Alternatively, use `cargo build --locked --release` in a VS x64 Native Tools
shell; the resource compiler must be on PATH or specified by `RC`.

`tools/env.ps1` discovers normal Rust/MSVC installations. On this development
machine it can also reuse the existing portable toolchain in the sibling
Wayrill project's `.tools` directory without modifying it. Set
`SOLCALC_MSVC_ROOT` to use a different portable MSVC/SDK installation. Cargo's
download cache for this project stays under `.tools/cargo`.

To build the installer with [Inno Setup](https://jrsoftware.org/isinfo.php):

```powershell
.\tools\bootstrap-installer.ps1  # optional: pinned, signed portable compiler
.\build-installer.ps1 -Check
```

The bootstrap uses OpenSSL curl-mingw and verifies the compiler's SHA-256 and
Authenticode signature before extracting it under `.tools`. It does not install
the compiler globally. An existing Inno Setup compiler can instead be selected
with `-Iscc` or `SOLCALC_ISCC`. Setup's `/CURRENTUSER` and `/ALLUSERS` switches
select the installation scope for unattended deployment. Both scopes run the
same executable; each Windows user retains a separate profile.

The interface uses Win32 common controls, GDI text, embedded icons,
and a per-monitor DPI manifest. There is no webview or GUI framework runtime.
Native controls keep their keyboard behavior; their surfaces are painted for
consistent light and dark colors. The title bar uses Microsoft's
[DWM theme attributes](https://learn.microsoft.com/windows/apps/desktop/modernize/apply-windows-themes),
with client-side theme handling on Windows versions that lack those attributes.
The C runtime is linked statically through the Rust
[`crt-static` target feature](https://doc.rust-lang.org/reference/linkage.html#static-and-dynamic-c-runtimes).
Windows bindings come from Microsoft's
[`windows-sys`](https://github.com/microsoft/windows-rs) crate.

See [testing](testing.md) for checks and verification limits.
