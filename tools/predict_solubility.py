"""Generate offline aqueous estimates from a release-matched ChEBI Core OBO."""
import argparse
import csv
import gzip
import json
import math
from pathlib import Path
import re

from chebi_catalog import ROOT, read


def structures(path, release):
    with gzip.open(path, 'rt', encoding='utf-8') as file:
        assert next(file).startswith('format-version:')
        assert next(file).strip() == f'data-version: chebi/{release}'
        identity = None
        for line in file:
            if line.startswith('['):
                identity = None
            elif line.startswith('id: CHEBI:'):
                identity = line.strip().removeprefix('id: CHEBI:')
            elif identity and line.startswith('property_value: chemrof:smiles_string '):
                match = re.search(r'"((?:\\.|[^"\\])*)"', line)
                if match:
                    yield identity, match[1].replace('\\\\', '\\')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('chebi_obo', type=Path)
    args = parser.parse_args()
    from rdkit import Chem, RDLogger
    from rdkit.Chem import Crippen, Descriptors, rdMolDescriptors

    RDLogger.DisableLog('rdApp.*')
    source = ROOT / 'data/source'
    chebi = source / 'chebi'
    release = json.loads((chebi / 'manifest.json').read_text(encoding='utf-8'))['release']
    bundled = {row['chebi_id']: row for row in read(chebi / 'main.en.tsv')}
    estimates = {}
    for identity, smiles in structures(args.chebi_obo, release):
        row = bundled.get(identity)
        if row is None or '.' in smiles or not 100 <= float(row['molar_mass']) <= 600:
            continue
        molecule = Chem.MolFromSmiles(smiles)
        if molecule is None or rdMolDescriptors.CalcMolFormula(molecule) != row['formula']:
            continue
        atoms = list(molecule.GetAtoms())
        if not any(atom.GetAtomicNum() == 6 for atom in atoms):
            continue
        if any(atom.GetFormalCharge() or atom.GetIsotope() or atom.GetNumRadicalElectrons()
               or atom.GetSymbol() not in {'C', 'H', 'N', 'O', 'F', 'P', 'S', 'Cl', 'Br', 'I'}
               for atom in atoms):
            continue
        log_p = Crippen.MolLogP(molecule)
        if not -2 <= log_p <= 6:
            continue
        mass = Descriptors.MolWt(molecule)
        rotors = rdMolDescriptors.CalcNumRotatableBonds(molecule)
        aromatic_fraction = sum(atom.GetIsAromatic() for atom in atoms) / molecule.GetNumHeavyAtoms()
        # Delaney's published ESOL equation with RDKit molecular descriptors.
        log_s = 0.16 - 0.63 * log_p - 0.0062 * mass + 0.066 * rotors - 0.74 * aromatic_fraction
        if not math.isfinite(log_s) or log_s + math.log10(mass) >= 3:
            continue
        assert identity not in estimates, identity
        estimates[identity] = (row['formula'], format(10 ** log_s, '.2g'))

    with (source / 'solubility-prediction.tsv').open('w', encoding='utf-8', newline='') as file:
        writer = csv.writer(file, delimiter='\t', lineterminator='\n')
        writer.writerow(['chebi_id', 'formula', 'mol_l'])
        for identity in sorted(estimates, key=int):
            writer.writerow([identity, *estimates[identity]])
    print(f'Generated {len(estimates)} ESOL water-solubility estimates')


if __name__ == '__main__':
    main()
