# Changelog

## Solcalc 3.6.2

### Offline solubility warnings

- In **Mass / volume** and **Stock preparation**, choose a bundled substance
  and a solvent to check the preparation against the reported solubility.
  Stock preparation checks the strength of the stock you will make.
- The offline catalog includes **8,428 reported entries across 43 solvents** for
  **2,875 substances**, including explicit insolubility findings. The substance
  picker shows the source and available temperature and pH. When the source
  omits temperature, Solcalc displays **21 °C assumed**.
- Solcalc converts composition-based limits to molarity when it has the
  saturated-solution density. The catalog includes density and direct
  volume-based data for common salts, alkalis, and other reagents. When a
  density is unavailable, Solcalc shows the minimum solvent mass and can
  compare it with the amount you enter.
- For **58,513 eligible entries** without a reported water value, Solcalc can
  show a labeled ESOL estimate and prompt you to verify preparations that
  exceed it.

### Substance search and catalog

- Search ranks name prefixes ahead of synonyms and matches inside names.
  Typing `suc` or `сах` brings up sucrose first. Exact CAS and formula matches
  still take priority.
- The bundled picker excludes standalone ions, residues, and chemical groups,
  leaving **92,287 selectable ChEBI entries**. Neutral salts and hydrates
  remain available.
- Each stereoisomer group shows its best match. Choose **Show stereoisomers**
  to select an individual form with its own identifiers and solubility data.
  Exact name and CAS searches can still find a specific form directly.

### Updating on Windows

The x64 installer supports current-user and all-users installation. The
portable ZIP runs after extraction. Both use the same native executable.
Settings and user substances remain in `%LOCALAPPDATA%\Solcalc`.

[Catalog sources and methodology](https://github.com/eliotcougar/Solcalc-3/blob/v3.6.2/data/README.md) ·
[Full changes since 3.6.1](https://github.com/eliotcougar/Solcalc-3/compare/v3.6.1...v3.6.2)

## Earlier releases

[Solcalc 3.6.1](https://github.com/eliotcougar/Solcalc-3/releases/tag/v3.6.1) ·
[Solcalc 3.5.1](https://github.com/eliotcougar/Solcalc-3/releases/tag/v3.5.1)
