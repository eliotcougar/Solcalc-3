# Contributing

Use a branch and pull request for contributions. The default branch requires
Windows checks and tests, catalog validation, dependency review, and CodeQL for
Rust, Python and GitHub Actions. Resolve review conversations before merging.
This single-maintainer project does not require a second person's approval.

The owner, through the repository administrator role, can push directly after
appropriate local checks. CI still runs afterward. This bypass does not count
as evidence that a change passed validation. Force pushes and deletion of the
default branch are blocked for everyone, including administrators.

The two policies are recorded in [.github/rulesets](.github/rulesets). They must
also be applied in GitHub; adding JSON to a fork does not change its settings.
Keep required check names and workflow job names synchronized.

## Checks

```powershell
.\build.ps1 -Check
python tools/generate-catalog.py --check
```

See [building](docs/building.md) and [testing](docs/testing.md). Keep regressions
focused on behavior: calculation boundaries, unit conversion, locale fallback,
search, persistence, migration rollback and concurrent writes. Use temporary
databases, never a contributor's real profile. Formula and CAS are optional for
user substances; a positive finite molar mass and one name are sufficient.

The manual **Windows packages** workflow builds the installer and portable ZIP,
tests both installation scopes on a disposable Windows runner, and stores the
packages as artifacts. It does not create releases. Dependency updates receive
the same checks and review; there is no automatic merge or publish workflow.

Use public, reproducible inputs. Keep profiles, migration backups, logs containing
personal data, credentials and workstation-specific QA evidence outside Git.
Preserve third-party notices. Report bugs with the app version, steps and expected
result, and use synthetic values in examples.
