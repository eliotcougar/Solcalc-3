import csv
import gzip
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from contextlib import redirect_stdout
from io import StringIO

spec = importlib.util.spec_from_file_location('chebi_import', Path(__file__).resolve().parents[1] / 'tools/import-chebi.py')
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class ChebiImportTests(unittest.TestCase):
    def test_fixed_composition_and_hydrate_equivalence(self):
        for formula in ['NaCl', 'C6H12O6', '[13C]6H12O6', 'MgSO4·7H2O']:
            self.assertTrue(module.fixed_formula(formula), formula)
        for formula in ['', '(C2H4)n', 'C6H5R', 'X2O', 'C6H12O6)', 'Na(Cl']:
            self.assertFalse(module.fixed_formula(formula), formula)
        self.assertEqual(module.atom_counts('MgSO4·7H2O'), module.atom_counts('H14MgO11S'))
        self.assertNotEqual(module.atom_counts('MgSO4'), module.atom_counts('H14MgO11S'))
        self.assertEqual(module.atom_counts('Na2HPO4·12H2O'), module.atom_counts('H25Na2O16P'))

    def test_import_preserves_curated_sources_separates_locales_and_uses_average_mass(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            source = root / 'input'
            source.mkdir()
            output = root / 'output'
            output.mkdir()
            (source / 'README').write_text('ChEBI Release: 257\nDate of last update: 2026-11-05\n', encoding='utf-8')
            (source / 'LICENSE').write_text('Synthetic fixture, not real ChEBI data.', encoding='utf-8')
            def table(name, columns, rows):
                with gzip.open(source / (name + '.tsv.gz'), 'wt', encoding='utf-8', newline='') as file:
                    writer = csv.writer(file, delimiter='\t')
                    writer.writerow(columns)
                    writer.writerows(rows)
            curated = root / 'curated.tsv'
            module.write_tsv(curated, ['id', 'cas', 'formula', 'name', 'synonyms'],
                             [['epsom', '10034-99-8', 'MgSO4·7H2O', 'Original name', 'original synonym']])
            original = curated.read_bytes()
            table('compounds', ['id', 'name', 'status_id'],
                  [['1', 'Source sulfate', '1'], ['2', 'ethanol', '3'], ['3', 'polymer', '1'],
                   ['4', 'undefined', '1'], ['5', 'unreviewed', '9'], ['6', 'conflicting CAS', '1']])
            table('chemical_data', ['compound_id', 'formula', 'mass', 'monoisotopic_mass', 'status_id'],
                  [['1', 'H14MgO11S', '246.475', '246.01', '1'], ['2', 'C2H6O', '46.069', '46.0419', '3'],
                   ['3', '(C2H4)n', '28.05', '28', '1'], ['4', 'H2O', 'NaN', '18', '1'],
                   ['5', 'C2H6', '30.07', '30', '9'], ['6', 'NaCl', '58.44', '58', '1']])
            table('names', ['compound_id', 'name', 'language_code', 'status_id'],
                  [['1', 'Extra sulfate synonym', 'en', '1'], ['1', 'Имя из ChEBI', 'ru', '1'],
                   ['2', '<i>ethyl</i> alcohol', 'en', '3'], ['2', 'спирт', 'ru', '3']])
            table('database_accession', ['compound_id', 'accession_number', 'type', 'status_id'],
                  [['1', '10034-99-8', 'CAS', '1'], ['2', '64-17-5', 'CAS', '3'],
                   ['2', '64-17-4', 'CAS', '1'], ['6', '10034-99-8', 'CAS', '1']])
            with redirect_stdout(StringIO()):
                module.generate(source, output, curated)
            self.assertEqual(curated.read_bytes(), original)
            def read(path):
                with path.open(encoding='utf-8', newline='') as file:
                    return list(csv.DictReader(file, delimiter='\t'))
            records = read(output / 'main.en.tsv')
            self.assertEqual(len(records), 1)
            self.assertEqual(records[0]['molar_mass'], '46.069')
            self.assertEqual(records[0]['cas'], '64-17-5')
            self.assertIn('ethyl alcohol', json.loads(records[0]['synonyms']))
            self.assertNotIn('спирт', records[0]['synonyms'])
            russian = read(output / 'locales/ru.tsv')
            self.assertEqual({r['substance_id'] for r in russian}, {'epsom', 'chebi:2'})
            links = read(output / 'curated-links.tsv')
            self.assertEqual(len(links), 1)
            self.assertEqual(links[0]['substance_id'], 'epsom')
            manifest = json.loads((output / 'manifest.json').read_text(encoding='utf-8'))
            self.assertEqual(manifest['invalid_cas_references_ignored'], 1)
            self.assertEqual(manifest['excluded']['existing_curated_cas'], 2)


if __name__ == '__main__':
    unittest.main()
