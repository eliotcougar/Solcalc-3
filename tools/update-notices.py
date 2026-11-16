"""Regenerate distribution notices from Cargo metadata and crate license files.

Run after `cargo fetch`, using the same Cargo environment as the build.
"""
import hashlib
import argparse
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--check', action='store_true')
args = parser.parse_args()
catalog = json.loads((ROOT / 'data/source/chebi/manifest.json').read_text(encoding='utf-8'))
metadata = json.loads(subprocess.check_output(['cargo', 'metadata', '--locked', '--format-version', '1',
                                              '--filter-platform', 'x86_64-pc-windows-msvc'], cwd=ROOT))
used = {node['id'] for node in metadata['resolve']['nodes']} - set(metadata['workspace_members'])
groups = {}
packages = []
for package in sorted(metadata['packages'], key=lambda p: (p['name'], p['version'])):
    if package['id'] not in used:
        continue
    label = f"{package['name']} {package['version']} ({package.get('license') or 'see license'})"
    packages.append(label)
    directory = Path(package['manifest_path']).parent
    candidates = set()
    for pattern in ['**/LICENSE*', '**/LICENCE*', '**/COPYING*', '**/NOTICE*']:
        candidates.update(path for path in directory.glob(pattern) if path.is_file())
    if package.get('license_file'):
        candidates.add(directory / package['license_file'])
    if not candidates:
        raise RuntimeError(f'License files missing: {label}')
    for path in sorted(candidates):
        text = path.read_text(encoding='utf-8').strip()
        key = hashlib.sha256(text.encode()).hexdigest()
        names, _ = groups.setdefault(key, ([], text))
        names.append(label)

lines = ['Solcalc third-party notices', '=' * 26, '',
         'ChEBI data', '----------',
         f"ChEBI data is from https://www.ebi.ac.uk/chebi ; ChEBI release {catalog['release']} ({catalog['release_date']}).",
         'Licensed under Creative Commons Attribution 4.0 International (CC BY 4.0).',
         'https://creativecommons.org/licenses/by/4.0/',
         'Modified for Solcalc: fixed-composition subset, normalized text and language-separated',
         'sources, CAS validation and independently matched translations, generated SQLite catalogs.',
         'The full license accompanies the catalog sources and installer as ChEBI-LICENSE.txt.', '',
         'Solubility data', '---------------',
         'Selected measurements from BigSolDB 2.2 by Krasnov, Malikov, Kiseleva, Tatarin,',
         'Sosnin and Bezzubov: https://zenodo.org/records/22648301',
         'Licensed under CC BY 4.0: https://creativecommons.org/licenses/by/4.0/',
         'Solcalc selects specific solutes, solvents and temperatures, maps them to ChEBI IDs',
         'and converts temperatures to Celsius. Individual references appear in the picker.',
         'Selected pure-solvent endpoints from MixtureSolDB v3 by Malikov and co-authors:',
         'https://doi.org/10.5281/zenodo.18660057',
         'Licensed under CC BY 4.0: https://creativecommons.org/licenses/by/4.0/',
         'Solcalc selects identity-matched measurements, maps them to ChEBI IDs and',
         'converts temperatures to Celsius. Individual study references appear in the picker.',
         'Selected AqSolDBc records by Pierre Llompart and co-authors: https://doi.org/10.57745/CZVZIA',
         'Data release 2.0, updated 2024-12-04.',
         'Open Licence 2.0: https://spdx.org/licenses/etalab-2.0.html',
         'Solcalc maps selected organic solutes to ChEBI IDs and converts log S to mol/L.',
         'Experimental water-solubility data from OPERA (Mansouri, Grulke, Judson and Williams,',
         '2018, supplement S1): https://doi.org/10.1186/s13321-018-0263-1',
         'CC0: https://creativecommons.org/publicdomain/zero/1.0/',
         'Solcalc selects identity-matched records and converts log molarity to mol/L.', '',
         'Selected numeric measurements from the IUPAC–NIST Solubility Data Series.',
         'References for individual measurements appear in the picker.',
         'Water estimates use the ESOL equation from John S. Delaney (2004):',
         'https://doi.org/10.1021/ci034243x',
         'Solcalc computes descriptors from ChEBI structures with RDKit at catalog preparation.', '',
         'PubChem', '-------',
         'Optional user-requested lookup: https://pubchem.ncbi.nlm.nih.gov/',
         'Fetched records are reviewed and saved by the user.',
         'The offline solubility catalog includes selected references from PubChem HSDB.',
         'Data-source terms and provenance: https://pubchem.ncbi.nlm.nih.gov/docs/downloads', '',
         'SQLite', '------',
         'SQLite is in the public domain: https://www.sqlite.org/copyright.html',
         'SQLite and the C runtime are statically linked into the executable.', '',
         'Rust dependencies', '-----------------', *packages, '']
for names, text in groups.values():
    lines.extend(['=' * 78, 'Applies to: ' + '; '.join(dict.fromkeys(names)), '', text, ''])
result = '\n'.join(lines).rstrip() + '\n'
if args.check:
    assert (ROOT / 'THIRD-PARTY-NOTICES.txt').read_text(encoding='utf-8') == result, 'Run tools/update-notices.py'
else:
    (ROOT / 'THIRD-PARTY-NOTICES.txt').write_text(result, encoding='utf-8')
print(f'{len(packages)} dependency notices, {len(groups)} distinct license texts')
