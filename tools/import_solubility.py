"""Select experimental solubility records for exact bundled ChEBI identities."""
import argparse
from collections import defaultdict
import csv
from functools import cache
import gzip
import json
import math
from pathlib import Path
import re
from statistics import median
from zipfile import ZipFile

from chebi_catalog import ROOT, read

AQSOL_SOURCE = 'https://doi.org/10.57745/CZVZIA'
OPERA_SOURCE = 'https://doi.org/10.1186/s13321-018-0263-1'

# Match both the name and structure of each pure solvent.
SOLVENT_DATA = read(ROOT / 'data/source/solvents.tsv')
SOLVENTS = {(row['source_name'], row['smiles']): row['name'] for row in SOLVENT_DATA}


def chebi_keys(obo, bundled):
    by_key = defaultdict(set)
    with gzip.open(obo, 'rt', encoding='utf-8') as file:
        identity = None
        for line in file:
            if line.startswith('['):
                identity = None
            elif line.startswith('id: CHEBI:'):
                identity = line.strip().removeprefix('id: CHEBI:')
            elif identity in bundled and line.startswith('property_value: chemrof:inchi_key_string '):
                match = re.search(r'"([A-Z]{14}-[A-Z]{10}-[A-Z])"', line)
                if match:
                    by_key[match[1]].add(identity)
    return {key: next(iter(ids)) for key, ids in by_key.items() if len(ids) == 1}


def water_molarity(log_s, molar_mass):
    # The compiled water data include 1,000,000 mg/L miscibility placeholders.
    # Allow for rounding of the reported log values and molecular weights.
    if not math.isfinite(log_s) or log_s + math.log10(molar_mass) >= math.log10(999.8):
        return None
    return 10 ** log_s or None


def opera_water(archive, bundled, identify, preferred):
    records = defaultdict(set)
    with ZipFile(archive) as zipped:
        for member in ('OPERA_WS/TR_WS_3158.sdf', 'OPERA_WS/TST_WS_1066.sdf'):
            # The original SMILES preserves identity that the QSAR-ready structure may lose.
            sdf = zipped.read(member).decode('latin-1').replace('\r\n', '\n')
            for block in sdf.split('$$$$'):
                row = {key: value.strip() for key, value in
                       re.findall(r'^> <([^>]+)>[^\S\n]*\n([^\n]*)', block, re.MULTILINE)}
                if (not re.fullmatch(r'\d{2,7}-\d{2}-\d', row.get('CAS', ''))
                        or row.get('Salt_Solvent') not in {'?', ''}
                        or not row.get('WS Reference', '').strip()
                        or not row.get('SMILES') or '.' in row['SMILES']):
                    continue
                try:
                    temperature = float(row['WS Temperature'])
                    log_s = float(row['LogMolar'])
                except (KeyError, ValueError):
                    continue
                if not 20 <= temperature <= 30:
                    continue
                identity = identify(row['SMILES'])
                if (identity is None or (identity, 'Water') in preferred
                        or row['CAS'] not in bundled[identity][1]):
                    continue
                molarity = water_molarity(log_s, bundled[identity][2])
                if molarity is not None:
                    records[identity].add((temperature, molarity))
    # Conflicting source values for an identity require manual review.
    return {identity: next(iter(values)) for identity, values in records.items() if len(values) == 1}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--bigsol', type=Path, required=True)
    parser.add_argument('--aqsol', type=Path, required=True)
    parser.add_argument('--opera', type=Path, required=True)
    parser.add_argument('--chebi-obo', type=Path, required=True)
    parser.add_argument('--mixturesol', type=Path, required=True)
    args = parser.parse_args()
    from rdkit import Chem, RDLogger

    RDLogger.DisableLog('rdApp.*')
    bundled = {row['chebi_id']: (row['formula'], set(json.loads(row['cas_numbers'])), float(row['molar_mass']))
               for row in read(ROOT / 'data/source/chebi/main.en.tsv')}
    keys = chebi_keys(args.chebi_obo, bundled)
    assert keys, 'No InChIKeys in ChEBI OBO'
    curated = read(ROOT / 'data/source/solubility.tsv')
    preferred = {(row['chebi_id'], row['solvent']) for row in curated}

    @cache
    def parse_smiles(smiles):
        return Chem.MolFromSmiles(smiles)

    @cache
    def identify(smiles):
        molecule = parse_smiles(smiles)
        return keys.get(Chem.MolToInchiKey(molecule)) if molecule is not None else None

    reported_water = opera_water(args.opera, bundled, identify, preferred)
    preferred.update((identity, 'Water') for identity in reported_water)

    candidates = defaultdict(set)
    with args.bigsol.open(encoding='utf-8-sig', newline='') as file:
        for row in csv.DictReader(file):
            solvent = SOLVENTS.get((row['Solvent'], row['SMILES_Solvent']))
            temperature = float(row['Temperature_K'])
            if not solvent or not 293.15 <= temperature <= 303.15 or row['solid_form_label'].strip():
                continue
            value = float(row['Solubility(mole_fraction)'])
            doi = row['Source'].strip()
            if not 0 < value < 1 or not re.fullmatch(r'10\.\d{4,9}/\S+', doi):
                continue
            identity = identify(row['SMILES_Solute'])
            if identity is None or (identity, solvent) in preferred:
                continue
            cas_numbers = bundled[identity][1]
            if row['CAS'] and cas_numbers and row['CAS'] not in cas_numbers:
                continue
            candidates[identity, solvent].add((temperature, value, doi))

    aqueous = defaultdict(set)
    for row in read(args.aqsol):
        if (row['Composition'] not in {'None', 'mono-constituent'}
                or row['Origin'] not in {'', 'organic'}
                or row['HasError'] != 'No' or row['AlertAtoms'] != 'False'
                or float(row['DuplicateSD']) > 0.3):
            continue
        identity = keys.get(row['InChIKey'])
        if (identity is None or (identity, 'Water') in preferred or (identity, 'Water') in candidates
                or identify(row['SmilesCurated']) != identity):
            continue
        molecule = parse_smiles(row['SmilesCurated'])
        if len(Chem.GetMolFrags(molecule)) != 1 or all(atom.GetAtomicNum() != 6 for atom in molecule.GetAtoms()):
            continue
        molarity = water_molarity(float(row['ExperimentalLogS']), bundled[identity][2])
        if molarity is not None:
            aqueous[identity].add(molarity)

    mixture_candidates = defaultdict(set)
    if args.mixturesol:
        with args.mixturesol.open(encoding='utf-8-sig', newline='') as file:
            for row in csv.DictReader(file):
                if row['IsPureSolventEndpoint'].lower() != 'true' or '.' in row['SMILES_Solute']:
                    continue
                try:
                    first, second = float(row['Fraction_Solvent1']), float(row['Fraction_Solvent2'])
                    temperature = float(row['Temperature_K'])
                    value = float(row['Solubility(mole_fraction)'])
                except ValueError:
                    continue
                if (first, second) == (1.0, 0.0):
                    solvent = SOLVENTS.get((row['Solvent1'], row['SMILES_Solvent1']))
                elif (first, second) == (0.0, 1.0):
                    solvent = SOLVENTS.get((row['Solvent2'], row['SMILES_Solvent2']))
                else:
                    continue
                doi = row['Source'].strip()
                if not solvent or not 293.15 <= temperature <= 303.15 or not 0 < value < 1:
                    continue
                if not re.fullmatch(r'10\.\d{4,9}/\S+', doi):
                    continue
                identity = identify(row['SMILES_Solute'])
                if identity is None or not row['CAS'] or row['CAS'] not in bundled[identity][1]:
                    continue
                key = (identity, solvent)
                if (key in preferred or key in candidates
                        or (solvent == 'Water' and identity in aqueous)):
                    continue
                if re.search(r'hydrate|anhydrous|polymorph|solvate|amorphous',
                             row['Compound_Name'], re.IGNORECASE):
                    continue
                mixture_candidates[key].add((temperature, value, doi))

    path = ROOT / 'data/source/solubility-reference.tsv'
    with path.open('w', encoding='utf-8', newline='') as file:
        writer = csv.writer(file, delimiter='\t', lineterminator='\n')
        writer.writerow(['chebi_id', 'formula', 'solvent', 'unit', 'value',
                         'temperature_c', 'ph', 'density', 'source'])
        for identity, solvent in sorted(candidates, key=lambda key: (int(key[0]), key[1])):
            measurements = candidates[identity, solvent]
            closest = min(round(abs(t - 298.15), 4) for t, _, _ in measurements)
            nearby = [row for row in measurements if round(abs(row[0] - 298.15), 4) == closest]
            midpoint = median(math.log(value) for _, value, _ in nearby)
            temperature, value, doi = min(nearby, key=lambda row: (abs(math.log(row[1]) - midpoint), row))
            writer.writerow([identity, bundled[identity][0], solvent, 'mole_fraction', value,
                             round(temperature - 273.15, 2), '', '', 'https://doi.org/' + doi])
        for identity, values in sorted(aqueous.items(), key=lambda item: int(item[0])):
            if len(values) == 1:
                value = format(next(iter(values)), '.8g')
                writer.writerow([identity, bundled[identity][0], 'Water', 'mol_l_solution', value,
                                 '', '', '', AQSOL_SOURCE])
        for identity, (temperature, molarity) in sorted(reported_water.items(), key=lambda item: int(item[0])):
            writer.writerow([identity, bundled[identity][0], 'Water', 'mol_l_solution',
                             format(molarity, '.6g'), temperature, '', '', OPERA_SOURCE])
        for (identity, solvent), measurements in sorted(mixture_candidates.items(), key=lambda item: (int(item[0][0]), item[0][1])):
            closest = min(round(abs(t - 298.15), 4) for t, _, _ in measurements)
            nearby = [row for row in measurements if round(abs(row[0] - 298.15), 4) == closest]
            midpoint = median(math.log(value) for _, value, _ in nearby)
            temperature, value, doi = min(nearby, key=lambda row: (abs(math.log(row[1]) - midpoint), row))
            writer.writerow([identity, bundled[identity][0], solvent, 'mole_fraction', value,
                             round(temperature - 273.15, 2), '', '', 'https://doi.org/' + doi])
    print(f'Selected {len(candidates)} BigSolDB references covering '
          f'{len({identity for identity, _ in candidates})} ChEBI substances')
    print(f'Selected {sum(len(values) == 1 for values in aqueous.values())} additional AqSolDBc references')
    print(f'Selected {len(reported_water)} comparable OPERA experimental water limits')
    print(f'Selected {len(mixture_candidates)} MixtureSolDB pure-solvent endpoints')


if __name__ == '__main__':
    main()
