# Tests and repository policy

Run `build.ps1 -Check` on Windows: Rust formatting, Clippy on all targets with
warnings denied, all unit tests, and an optimized Windows build. Local builds
and CI share this entry point and `rust-toolchain.toml`.

The test suite covers inverse solution calculations and blank-field selection,
unit conversion and automatic display boundaries, dilution and stock planning,
protein activity units, arithmetic validation, EN/RU formatting, saved settings,
substance search and CAS merging, optional fields, user persistence and deletion,
optimistic concurrency, additive migrations, WAL-aware backups, and rollback.
Tests use synthetic inputs and temporary database directories.

ChEBI checks verify English identities, names, formulas and masses against the
bundled source, Russian locale matching and bounded result lists. Importer
fixtures cover fixed formulas/hydrates, average versus monoisotopic mass,
language separation, invalid CAS values and chemical-form matching for translations.

PubChem tests use synthetic JSON and a loopback HTTP server: query encoding,
candidate/response limits, invalid masses, CAS ambiguity, cancellation, HTTP
errors, and saving without overwriting an existing user CAS. They do not contact
the live service in ordinary CI.

An explicit live connectivity check is available separately:

```powershell
cargo test --locked pubchem::tests::live_pubchem_smoke -- --ignored --nocapture
```

The manual **Windows packages** workflow has a `verify_pubchem` input for this
check on a disposable runner. It is off by default because it depends on an
external service and network connectivity. Failed HTTPS verification is reported;
certificate checks are never disabled to make a test pass.

`python tools/generate-catalog.py --check` independently validates source columns,
CAS checksums, unique ChEBI IDs, finite source masses, fixed formulas, locale references, database
schema/application IDs, row contents and SQLite integrity. Its result comparison
does not depend on the SQLite binary encoding produced by a particular machine.
Run importer tests with `python -m unittest discover -s tests -v`.

## GitHub Actions

| Workflow | Trigger | Purpose |
| --- | --- | --- |
| Checks | Main pushes, pull requests, manual | Windows formatting/lint/tests/release build; catalog validation on Linux |
| CodeQL | Main pushes, pull requests, weekly, manual | Extended security analysis for Rust, Python and Actions |
| Dependency review | Pull requests | Reject high/critical dependency vulnerabilities |
| Windows packages | Manual | Build installer/portable ZIP, verify hashes, install/reinstall/uninstall in both scopes |

Actions are pinned to full commit SHAs; tokens default to read-only permissions.
Only CodeQL analysis jobs receive security-events write access. Dependabot proposes
Cargo and Actions updates weekly. No workflow automatically merges or publishes.

Installer smoke tests refuse to run outside a disposable GitHub-hosted runner,
with an existing installation/profile, or without the elevation needed to test
all-users installation. They check installed payloads, HKCU/HKLM registration,
reinstallation, cleanup and preservation of opaque profile fixtures, migration
backup fixtures, and unrelated installation-directory files. They do not launch
the UI or benchmark startup.

## What still needs manual verification

Unit tests and installer smoke tests do not prove native interaction, visual
layout, interactive elevation dialogs, screen-reader behavior, high-contrast
appearance, multi-monitor DPI transitions or compatibility with every Windows
installation. Check affected flows separately, using an isolated test profile.
Avoid running mutating installer tests against a maintainer's active profile.

The owner/admin can push directly after local validation. Contributors use checked
pull requests; no second maintainer approval is required. Default-branch deletion
and force pushes are blocked even for administrators. See [contribution policy](../CONTRIBUTING.md)
and the checked-in [rulesets](../.github/rulesets).
