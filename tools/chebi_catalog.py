"""Generate/check the read-only ChEBI catalog from committed normalized sources."""
import csv
import json
import math
from pathlib import Path
import sqlite3

ROOT = Path(__file__).resolve().parents[1]


def read(path):
    with path.open(encoding='utf-8', newline='') as file:
        return list(csv.DictReader(file, delimiter='\t'))


def save(path, schema, metadata, table, columns, records, check, application=0x534F4C43):
    expected = sqlite3.connect(':memory:')
    expected.executescript(f'PRAGMA application_id={application};PRAGMA user_version=1;'
                           'CREATE TABLE metadata(key TEXT PRIMARY KEY,value TEXT NOT NULL);' + schema)
    expected.executemany('INSERT INTO metadata VALUES(?,?)', sorted(metadata.items()))
    expected.executemany(f'INSERT INTO {table} VALUES({",".join("?" for _ in columns)})', records)
    expected.commit()
    if check:
        with sqlite3.connect(path.as_uri() + '?mode=ro', uri=True) as actual:
            for query in ['PRAGMA integrity_check', 'PRAGMA application_id', 'PRAGMA user_version',
                          'SELECT type,name,sql FROM sqlite_master ORDER BY name',
                          'SELECT * FROM metadata ORDER BY key',
                          f'SELECT * FROM {table} ORDER BY {columns[0]}']:
                assert actual.execute(query).fetchall() == expected.execute(query).fetchall(), (path, query)
    else:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(expected.serialize())
    expected.close()


def build(check, valid_cas, curated_cas):
    source = ROOT / 'data/source/chebi'
    manifest = json.loads((source / 'manifest.json').read_text(encoding='utf-8'))
    rows = read(source / 'main.en.tsv')
    assert len(rows) == manifest['records'] and len(rows) > 1000
    identities = set()
    records = []
    for row in rows:
        identity = row['chebi_id']
        assert identity.isdigit() and identity not in identities
        identities.add(identity)
        cas_numbers = json.loads(row['cas_numbers'])
        assert isinstance(cas_numbers, list) and len(cas_numbers) == len(set(cas_numbers))
        for cas in cas_numbers:
            valid_cas(cas)
            assert cas not in curated_cas, (identity, cas)
        assert row['cas'] == (cas_numbers[0] if cas_numbers else '')
        synonyms = json.loads(row['synonyms'])
        assert isinstance(synonyms, list) and all(isinstance(s, str) and s for s in synonyms)
        mass = float(row['molar_mass'])
        assert row['name'] and row['formula'] and math.isfinite(mass) and mass > 0
        records.append(('chebi:' + identity, row['cas'], row['formula'], row['name'], mass,
                        ';'.join(synonyms), ';'.join(cas_numbers)))
    metadata = {'locale': 'en', 'catalog_version': manifest['release'], 'source': 'ChEBI', 'license': 'CC-BY-4.0'}
    save(ROOT / 'data/bundled/chebi.en.sqlite',
         'CREATE TABLE substances(id TEXT PRIMARY KEY,cas TEXT NOT NULL,formula TEXT NOT NULL,'
         'name TEXT NOT NULL,molar_mass REAL NOT NULL CHECK(molar_mass>0),synonyms TEXT NOT NULL,cas_numbers TEXT NOT NULL);',
         metadata, 'substances', ['id', 'cas', 'formula', 'name', 'molar_mass', 'synonyms', 'cas_numbers'], records, check)
    curated = {r['id'] for r in read(ROOT / 'data/source/main.en.tsv')}
    links = read(source / 'curated-links.tsv')
    assert len(links) == manifest['curated_alias_links']
    alias_records = []
    for row in links:
        assert row['substance_id'] in curated and row['chebi_id'].isdigit()
        numbers = json.loads(row['cas_numbers'])
        for cas in numbers: valid_cas(cas)
        synonyms = json.loads(row['synonyms'])
        assert all(isinstance(s, str) and s for s in synonyms)
        alias_records.append((row['substance_id'], 'CHEBI:' + row['chebi_id'], ';'.join(synonyms), ';'.join(numbers)))
    save(ROOT / 'data/bundled/chebi.aliases.sqlite',
         'CREATE TABLE aliases(substance_id TEXT NOT NULL,source_id TEXT PRIMARY KEY,synonyms TEXT NOT NULL,cas_numbers TEXT NOT NULL);',
         metadata, 'aliases', ['substance_id', 'source_id', 'synonyms', 'cas_numbers'], alias_records, check)
    index = ['pub const CHEBI_LOCALES: &[(&str, &[u8])] = &[']
    for path in sorted((source / 'locales').glob('*.tsv')):
        locale = path.stem
        translated = read(path)
        assert len(translated) == manifest['locales'][locale]
        assert len({r['substance_id'] for r in translated}) == len(translated)
        values = []
        for row in translated:
            target = row['substance_id']
            assert (target in curated or target.removeprefix('chebi:') in identities) and row['name']
            synonyms = json.loads(row['synonyms'])
            assert all(isinstance(s, str) and s for s in synonyms)
            values.append((target, row['name'], ';'.join(synonyms)))
        save(ROOT / f'data/bundled/chebi-locales/{locale}.sqlite',
             'CREATE TABLE translations(substance_id TEXT PRIMARY KEY,name TEXT NOT NULL,synonyms TEXT NOT NULL);',
             dict(metadata, locale=locale), 'translations', ['substance_id', 'name', 'synonyms'], values, check, 0x534F4C4C)
        index.append(f'    ("{locale}", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/data/bundled/chebi-locales/{locale}.sqlite"))),')
    index.append('];')
    print(f"ChEBI release {manifest['release']}: {len(records)} fixed-composition substances; {len(manifest['locales'])} locale(s)")
    return index
