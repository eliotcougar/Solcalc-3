# Solcalc contributor instructions

Native Win32 solution calculator in Rust. Keep calculation logic in `calc.rs`,
localization in `i18n.rs`, and user-owned storage separate from bundled catalogs.

- Use OpenSSL curl-mingw for HTTPS on this machine; Windows' bundled curl fails.
- Run `build.ps1 -Check` for Rust changes and
  `python tools/generate-catalog.py --check` for catalog changes.
- Keep EN/RU UI behavior aligned. The blank input selects the calculation;
  display units are automatic. Activity concentrations are U/mL and kU/mL only.
- Preserve each user's data. Schema migrations must be nondestructive and
  backed up; bundled updates must never reseed or replace user records.
- Ordinary tests use synthetic inputs and temporary profiles. Installer mutation
  tests belong on disposable CI runners, not a maintainer's active profile.
- Do not add startup benchmarks. Keep private profiles, QA reports, credentials,
  caches and built executables out of published source.
- See CONTRIBUTING.md for checks and branch rules. Owner/admin direct pushes
  remain available; default-branch force pushes and deletion are blocked.
- Build installer and portable ZIP from the same versioned executable. Keep
  Cargo.toml and Windows version resources aligned; installers use the default
  Inno Setup icon. Do not publish a release unless requested.
