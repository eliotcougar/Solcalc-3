import csv
import gzip
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
import sys
from contextlib import redirect_stdout
from io import StringIO

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'tools'))
from chemical_formula import atom_counts
from chebi_catalog import localizations

spec = importlib.util.spec_from_file_location('chebi_import', Path(__file__).resolve().parents[1] / 'tools/import-chebi.py')
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class ChebiImportTests(unittest.TestCase):
    def test_fixed_composition_and_hydrate_equivalence(self):
        for formula in ['NaCl', 'C6H12O6', '[13C]6H12O6', 'MgSO4·7H2O']:
            self.assertTrue(module.fixed_formula(formula), formula)
        for formula in ['', '(C2H4)n', 'C6H5R', 'X2O', 'C6H12O6)', 'Na(Cl']:
            self.assertFalse(module.fixed_formula(formula), formula)
        self.assertEqual(atom_counts('MgSO4·7H2O'), atom_counts('H14MgO11S'))
        self.assertNotEqual(atom_counts('MgSO4'), atom_counts('H14MgO11S'))
        self.assertEqual(atom_counts('Na2HPO4·12H2O'), atom_counts('H25Na2O16P'))

    def test_import_uses_chebi_for_all_english_data_and_separates_locales(self):
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
                module.generate(source, output)
            def read(path):
                with path.open(encoding='utf-8', newline='') as file:
                    return list(csv.DictReader(file, delimiter='\t'))
            records = read(output / 'main.en.tsv')
            self.assertEqual(len(records), 3)
            self.assertEqual(records[0]['name'], 'Source sulfate')
            self.assertEqual(records[0]['molar_mass'], '246.475')
            self.assertEqual(records[1]['molar_mass'], '46.069')
            self.assertEqual(records[1]['cas'], '64-17-5')
            self.assertIn('ethyl alcohol', json.loads(records[1]['synonyms']))
            self.assertNotIn('спирт', records[1]['synonyms'])
            russian = read(output / 'locales/ru.tsv')
            self.assertEqual({r['substance_id'] for r in russian}, {'chebi:1', 'chebi:2'})
            self.assertFalse((output / 'curated-links.tsv').exists())
            manifest = json.loads((output / 'manifest.json').read_text(encoding='utf-8'))
            self.assertEqual(manifest['invalid_cas_references_ignored'], 1)
            self.assertNotIn('existing_curated_cas', manifest['excluded'])

    def test_locale_text_matches_cas_and_composition_without_changing_chemical_data(self):
        records = [('chebi:1', '10034-99-8', 'H14MgO11S', 'ChEBI name', 246.475, '', '10034-99-8'),
                   ('chebi:2', '10034-99-8', 'MgO4S', 'Other form', 120.368, '', '10034-99-8')]
        rows = [{'cas': '10034-99-8', 'formula': 'MgSO4·7H2O', 'name': 'Магний сернокислый', 'synonyms': 'английская соль'},
                {'cas': '7446-19-7', 'formula': 'ZnSO4·H2O', 'name': 'Несопоставленное имя', 'synonyms': ''}]
        before = [r.copy() for r in rows]
        result, matched = localizations(rows, records)
        self.assertEqual(result, [('chebi:1', 'Магний сернокислый', 'английская соль')])
        self.assertEqual(matched, 1)
        self.assertEqual(rows, before)
        self.assertEqual(records[0][3:5], ('ChEBI name', 246.475))


if __name__ == '__main__':
    unittest.main()
