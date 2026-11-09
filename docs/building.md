# Building Solcalc

Use Windows 10/11 x64, the Rust toolchain pinned in `rust-toolchain.toml`, Visual
Studio C++ build tools, and a Windows SDK with `rc.exe`. A standard Rustup install
automatically selects the pinned toolchain. From PowerShell in the project root:

```powershell
.\build.ps1 -Check
```

This runs formatting, Clippy with warnings denied, unit tests and a release build,
then copies `Solcalc.exe` to `dist`. Close the running executable before replacing
it. `tools/env.ps1` discovers Rust/MSVC/SDK installations without changing the
machine's PATH permanently. `SOLCALC_MSVC_ROOT` selects a portable MSVC/SDK tree;
the sibling Wayrill toolchain is an optional local fallback, never a prerequisite.
The Cargo download cache lives under `.tools/cargo`.

Alternatively, build with `cargo build --locked --release` from an x64 VS Native
Tools shell. The Windows resource compiler must be on PATH or selected by `RC`.
SQLite and the C runtime are statically linked. The UI uses Win32 controls and GDI;
no Python, webview, GUI-framework runtime or external database DLL is shipped.

## Packages

Install Inno Setup 6.7+ or extract the pinned, signed compiler locally:

```powershell
.\tools\bootstrap-installer.ps1
.\build-installer.ps1 -Check
.\build-portable.ps1 -SkipBuild
.\tools\write-checksums.ps1
```

The bootstrap uses OpenSSL curl-mingw, verifies the pinned compiler's SHA-256 and
Authenticode publisher, and extracts it into `.tools` in portable mode. Pass
`-Curl` with its full executable path when automatic discovery is unavailable.
Pass `-Iscc` to `build-installer.ps1`, or set `SOLCALC_ISCC`, to use another compiler.

Both packages contain the same versioned executable. Setup uses Inno's default
icon; application shortcuts and window icons use Solcalc's icon. Setup supports
`/CURRENTUSER` and `/ALLUSERS` for unattended deployment and asks interactively
otherwise. Application files, uninstall registration and shortcuts follow the
selected scope. User profiles remain separate under `%LOCALAPPDATA%\Solcalc`.

The portable ZIP includes the executable, user documentation, catalog sources and
third-party notices. It excludes local profiles, QA reports, caches and build logs.
Its payload is checked against the built executable. Portable describes the
installation-free executable: preferences and user substances still live in the
Windows user profile, rather than beside the executable.

The manual **Windows packages** Actions workflow builds and tests the same inputs
on a clean runner and uploads artifacts. It does not publish a release. Before
publishing, align Cargo and Windows resource versions, run checks, inspect package
contents and hashes, and test affected UI or installer behavior. Keep third-party
notices with binary distributions.

## Catalogs and icons

The checked-in SQLite catalogs build without Python. After editing their TSV
sources, regenerate and validate with Python 3.10+:

```powershell
python tools/generate-catalog.py
python tools/generate-catalog.py --check
python -m unittest discover -s tests -v
```

See [catalog schemas, locales and provenance](../data/README.md).
The normalized ChEBI sources are committed, so normal builds and checks do not
download ChEBI or contact PubChem. To update a ChEBI release, download the official
flat files into a working directory and run `tools/import-chebi.py --input-dir`
with that directory, then regenerate the catalogs. The manifest records release
and input hashes; curated sources and user profiles are never importer outputs.

After changing Cargo dependencies, run `python tools/update-notices.py` in the
same build environment after `cargo fetch` to regenerate distribution notices.
`tools/generate-icon.ps1` rebuilds small-icon frames using ImageMagick; ordinary
builds use the checked-in icon assets. The original Python app and large icon
are preserved under `solcalc2/`.
