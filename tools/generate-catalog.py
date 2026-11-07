"""Generate the offline molar-mass catalog. No network or third-party packages.

Atomic weights: CIAAW Abridged Standard Atomic Weights 2024,
https://www.ciaaw.org/abridged-atomic-weights.htm (retrieved 2026-11-07).
Formula masses are sums for the specified stoichiometry, rounded to 0.001 g/mol.
"""
from decimal import Decimal, ROUND_HALF_UP
from pathlib import Path
import csv
import sqlite3
import re
import sys

ROOT = Path(__file__).resolve().parents[1]
WEIGHTS = dict(item.split(':') for item in '''H:1.0080 Li:6.94 B:10.81 C:12.011
N:14.007 O:15.999 Na:22.990 Mg:24.305 Al:26.982 P:30.974 S:32.06 Cl:35.45
K:39.098 Ca:40.078 Mn:54.938 Fe:55.845 Co:58.933 Ni:58.693 Cu:63.546
Zn:65.38 Br:79.904 Ag:107.87 I:126.90 Ba:137.33'''.split())
WEIGHTS = {key: Decimal(value) for key, value in WEIGHTS.items()}


def mass(formula):
    def group(tokens, pos):
        total = Decimal(0)
        while pos < len(tokens) and tokens[pos] != ')':
            token = tokens[pos]
            pos += 1
            if token == '(':
                value, pos = group(tokens, pos)
                assert tokens[pos] == ')'
                pos += 1
            else:
                value = WEIGHTS[token]
            count = 1
            if pos < len(tokens) and tokens[pos].isdigit():
                count = int(tokens[pos])
                pos += 1
            total += value * count
        return total, pos

    total = Decimal(0)
    for part in formula.split('·'):
        prefix = re.match(r'^(\d+)', part)
        multiplier = int(prefix[0]) if prefix else 1
        part = part[len(prefix[0]):] if prefix else part
        tokens = re.findall(r'[A-Z][a-z]?|\d+|[()]', part)
        assert ''.join(tokens) == part, formula
        value, end = group(tokens, 0)
        assert end == len(tokens), formula
        total += multiplier * value
    return total.quantize(Decimal('.001'), rounding=ROUND_HALF_UP)


# Independently familiar reference cases and hydrate increments.
assert mass('NaCl') == Decimal('58.440')
assert mass('C6H12O6') == Decimal('180.156')
assert mass('MgSO4·7H2O') == Decimal('246.466')
assert mass('Na2HPO4·12H2O') == Decimal('358.138')
assert mass('C10H14N2Na2O8·2H2O') == Decimal('372.238')
assert mass('(NH4)2Fe(SO4)2·6H2O') == Decimal('392.125')

def read_tsv(path, columns):
    with path.open(encoding='utf-8', newline='') as source:
        reader = csv.DictReader(source, delimiter='\t')
        assert reader.fieldnames == columns, path
        rows = list(reader)
        assert all(None not in row and all(v is not None for v in row.values()) for row in rows), path
        return rows


def valid_cas(value):
    assert re.fullmatch(r'[1-9][0-9]{1,6}-[0-9]{2}-[0-9]', value), value
    digits = value.replace('-', '')
    assert sum(int(c) * i for i, c in enumerate(digits[-2::-1], 1)) % 10 == int(digits[-1]), value


def database(path, app_id, schema, table, columns, records, locale):
    expected = sqlite3.connect(':memory:')
    expected.executescript(f'''PRAGMA application_id={app_id}; PRAGMA user_version=1;
        CREATE TABLE metadata (key TEXT PRIMARY KEY, value TEXT NOT NULL);
        INSERT INTO metadata VALUES ('locale', '{locale}'), ('catalog_version', '2');
        {schema}''')
    expected.executemany(f'INSERT INTO {table} VALUES ({",".join("?" for _ in columns)})', records)
    expected.commit()
    if '--check' in sys.argv:
        with sqlite3.connect(path.as_uri() + '?mode=ro', uri=True) as actual:
            assert actual.execute('PRAGMA integrity_check').fetchone() == ('ok',), path
            assert actual.execute('PRAGMA application_id').fetchone() == (app_id,), path
            assert actual.execute('PRAGMA user_version').fetchone() == (1,), path
            for query in ['SELECT type,name,sql FROM sqlite_master ORDER BY name',
                          'SELECT * FROM metadata ORDER BY key',
                          f'SELECT * FROM {table} ORDER BY {columns[0]}']:
                assert actual.execute(query).fetchall() == expected.execute(query).fetchall(), path
    else:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(expected.serialize())
    expected.close()


base = read_tsv(ROOT / 'data/source/main.en.tsv', ['id', 'cas', 'formula', 'name', 'synonyms'])
assert len({r['id'] for r in base}) == len(base)
assert len({r['cas'] for r in base}) == len(base)
for row in base:
    valid_cas(row['cas'])
    assert row['name'] and row['formula']
    assert not re.search('[А-Яа-яЁё]', row['name'] + row['synonyms']), row['id']
database(ROOT / 'data/bundled/main.en.sqlite', 0x534F4C43,
         '''CREATE TABLE substances (id TEXT PRIMARY KEY, cas TEXT NOT NULL UNIQUE,
         formula TEXT NOT NULL, name TEXT NOT NULL, molar_mass REAL NOT NULL CHECK(molar_mass>0),
         synonyms TEXT NOT NULL);''', 'substances',
         ['id', 'cas', 'formula', 'name', 'molar_mass', 'synonyms'],
         [(r['id'], r['cas'], r['formula'], r['name'], float(mass(r['formula'])), r['synonyms']) for r in base], 'en')
ids = {r['id'] for r in base}
index = ['// Generated by tools/generate-catalog.py.',
         'pub const LOCALES: &[(&str, &[u8])] = &[']
locales = sorted((ROOT / 'data/source/locales').glob('*.tsv'))
for path in locales:
    code = path.stem
    assert re.fullmatch(r'[a-z]{2,3}(?:-[a-z0-9]{2,8})*', code) and code != 'en', code
    rows = read_tsv(path, ['substance_id', 'name', 'synonyms'])
    assert len({r['substance_id'] for r in rows}) == len(rows), path
    assert all(r['substance_id'] in ids for r in rows), path
    database(ROOT / f'data/bundled/locales/{code}.sqlite', 0x534F4C4C,
             '''CREATE TABLE translations (substance_id TEXT PRIMARY KEY,
             name TEXT NOT NULL, synonyms TEXT NOT NULL);''', 'translations',
             ['substance_id', 'name', 'synonyms'],
             [(r['substance_id'], r['name'], r['synonyms']) for r in rows], code)
    index.append(f'    ("{code}", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/data/bundled/locales/{code}.sqlite"))),')
index.append('];')
index = '\n'.join(index) + '\n'
index_path = ROOT / 'data/bundled/locale_index.rs'
if '--check' in sys.argv:
    assert index_path.read_text(encoding='utf-8') == index, 'Regenerate locale index'
else:
    index_path.write_text(index, encoding='utf-8', newline='\n')
print(f'{len(base)} substances with valid CAS identifiers; {len(locales)} locale database(s); all formula masses validated')
