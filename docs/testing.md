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

`python tools/generate-catalog.py --check` independently validates source columns,
CAS checksums and uniqueness, formula-derived masses, locale references, database
schema/application IDs, row contents and SQLite integrity. Its result comparison
does not depend on the SQLite binary encoding produced by a particular machine.

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
