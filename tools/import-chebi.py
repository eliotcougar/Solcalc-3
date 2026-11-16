"""Build language-separated Solcalc sources from an official ChEBI flat-file release.

No network access; pass a directory containing the downloaded gzip files,
README and LICENSE. English chemical data comes exclusively from ChEBI.
"""
import argparse
import csv
import gzip
import hashlib
from html import unescape
from html.parser import HTMLParser
import json
import math
from pathlib import Path
import re
from collections import Counter, defaultdict
from chemical_formula import atom_counts, fixed_formula

ROOT = Path(__file__).resolve().parents[1]
ACTIVE = {'1', '3'}  # ChEBI CHECKED / OK; not SUBMITTED or DEPRECATED.
csv.field_size_limit(16 * 1024 * 1024)


class PlainText(HTMLParser):
    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.parts = []

    def handle_data(self, data):
        self.parts.append(data)


def clean(value):
    parser = PlainText()
    parser.feed(value or '')
    parser.close()
    return ' '.join(unescape(''.join(parser.parts)).split())


def valid_cas(value):
    if not re.fullmatch(r'[1-9][0-9]{1,6}-[0-9]{2}-[0-9]', value):
        return False
    digits = value.replace('-', '')
    return sum(int(c) * i for i, c in enumerate(digits[-2::-1], 1)) % 10 == int(digits[-1])


def rows(directory, name):
    with gzip.open(directory / (name + '.tsv.gz'), 'rt', encoding='utf-8', newline='') as file:
        yield from csv.DictReader(file, delimiter='\t')


def write_tsv(path, columns, records):
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open('w', encoding='utf-8', newline='') as file:
        writer = csv.writer(file, delimiter='\t', lineterminator='\n')
        writer.writerow(columns)
        writer.writerows(records)


def ontology(directory, release):
    """Read ChEBI's group hierarchy and standard structures for this release."""
    parents = defaultdict(list)
    groups = {'24433'}  # CHEBI:24433 is the group (part-molecular entity) class.
    structures = {}
    with gzip.open(directory / 'chebi_core.obo.gz', 'rt', encoding='utf-8') as file:
        identity = None
        version = None
        for raw in file:
            if raw.startswith('data-version:'):
                version = raw.strip().split()[-1]
            elif raw.startswith('['):
                identity = None
            elif raw.startswith('id: CHEBI:'):
                identity = raw.split()[1].removeprefix('CHEBI:')
            elif identity and raw.startswith('is_a: CHEBI:'):
                parents[raw.split()[1].removeprefix('CHEBI:')].append(identity)
            elif identity and raw.startswith('relationship: RO:0018037 '):
                groups.add(identity)  # is substituent group from
            elif identity and raw.startswith('property_value: chemrof:inchi_string "InChI=1S/'):
                structures[identity] = raw.split('"', 2)[1]
    if version != f'chebi/{release}':
        raise ValueError(f'ChEBI CORE ontology {version} does not match flat-file release {release}')
    pending = list(groups)
    while pending:
        for child in parents[pending.pop()]:
            if child not in groups:
                groups.add(child)
                pending.append(child)
    return groups, structures


def stereo_groups(records, structures):
    """Share connectivity, composition, charge and isotopes; vary only stereo."""
    candidates = defaultdict(list)
    for identity, _, _, formula, _, _, _ in records:
        inchi = structures.get(identity)
        if not inchi:
            continue
        _, inchi_formula, *layers = inchi.split('/')
        if '.' in inchi_formula:  # Leave salts and other multi-component forms independent.
            continue
        nonstereo = tuple(layer for layer in layers if layer[:1] not in {'b', 't', 'm', 's'})
        if not any(layer.startswith('c') for layer in nonstereo):
            continue
        composition = atom_counts(formula)
        if composition is None:
            continue
        key = (inchi_formula, nonstereo, tuple(sorted(composition.items())))
        candidates[key].append((identity, inchi))
    for members in candidates.values():
        if len({inchi for _, inchi in members}) > 1:
            group = min((identity for identity, _ in members), key=int)
            yield from ((identity, group) for identity, _ in members)


def generate(directory, output):
    readme = (directory / 'README').read_text(encoding='utf-8')
    release = re.search(r'ChEBI Release:\s*(\d+)', readme).group(1)
    date = re.search(r'Date of last update:\s*([0-9-]+)', readme).group(1)
    groups, structures = ontology(directory, release)
    compounds = {r['id']: r for r in rows(directory, 'compounds') if r['status_id'] in ACTIVE}
    chemical = {r['compound_id']: r for r in rows(directory, 'chemical_data') if r['status_id'] in ACTIVE}
    names = defaultdict(lambda: defaultdict(set))
    for row in rows(directory, 'names'):
        if row['status_id'] not in ACTIVE or row['compound_id'] not in compounds:
            continue
        locale = (row.get('language_code') or 'en').strip().replace('_', '-').lower()
        if not re.fullmatch(r'[a-z]{2,3}(?:-[a-z0-9]{2,8})*', locale):
            continue
        name = clean(row['name'])
        if name:
            names[row['compound_id']][locale].add(name)
    cas = defaultdict(set)
    rejected_cas = 0
    for row in rows(directory, 'database_accession'):
        if row['type'] == 'CAS' and row['status_id'] in ACTIVE:
            value = row['accession_number'].strip()
            if valid_cas(value):
                cas[row['compound_id']].add(value)
            else:
                rejected_cas += 1
    records = []
    locales = defaultdict(list)
    excluded = Counter()
    hidden = {}
    for identity, compound in sorted(compounds.items(), key=lambda item: int(item[0])):
        data = chemical.get(identity)
        if data is None:
            excluded['no_chemical_data'] += 1
            continue
        try:
            mass = float(data['mass'])
        except (TypeError, ValueError):
            mass = 0.0
        formula = clean(data['formula'])
        if not math.isfinite(mass) or mass <= 0 or not fixed_formula(formula):
            excluded['undefined_formula_or_mass'] += 1
            continue
        name = clean(compound['name'])
        if not name:
            excluded['missing_name'] += 1
            continue
        if identity in groups:
            hidden[identity] = 'group_or_residue'
        elif data['charge'] and int(data['charge']) != 0:
            hidden[identity] = 'ion'
        synonyms = sorted(names[identity]['en'] - {name}, key=lambda s: (s.casefold(), s))
        references = sorted(cas[identity])
        records.append([identity, references[0] if references else '', json.dumps(references, ensure_ascii=False), formula, name, repr(mass), json.dumps(synonyms, ensure_ascii=False)])
        for locale, alternatives in sorted(names[identity].items()):
            if locale == 'en' or not alternatives:
                continue
            alternatives = sorted(alternatives, key=lambda s: (s.casefold(), s))
            locales[locale].append(['chebi:' + identity, alternatives[0], json.dumps(alternatives[1:], ensure_ascii=False)])
    write_tsv(output / 'main.en.tsv', ['chebi_id', 'cas', 'cas_numbers', 'formula', 'name', 'molar_mass', 'synonyms'], records)
    write_tsv(output / 'excluded-entities.tsv', ['chebi_id', 'reason'], hidden.items())
    stereo = sorted(stereo_groups((r for r in records if r[0] not in hidden), structures),
                    key=lambda row: int(row[0]))
    write_tsv(output / 'stereoisomers.tsv', ['chebi_id', 'group_id'], stereo)
    for locale, records_locale in sorted(locales.items()):
        combined = defaultdict(set)
        for identity, name, synonyms in records_locale:
            combined[identity].update([name, *json.loads(synonyms)])
        records_locale[:] = [[identity, values[0], json.dumps(values[1:], ensure_ascii=False)]
                            for identity, names_locale in sorted(combined.items())
                            for values in [sorted(names_locale, key=lambda s: (s.casefold(), s))]]
        write_tsv(output / 'locales' / (locale + '.tsv'), ['substance_id', 'name', 'synonyms'], records_locale)
    manifest = {
        'source': 'ChEBI', 'release': release, 'release_date': date,
        'url': 'https://www.ebi.ac.uk/chebi', 'license': 'CC-BY-4.0',
        'records': len(records), 'locales': {key: len(value) for key, value in sorted(locales.items())},
        'selectable_records': len(records) - len(hidden),
        'picker_exclusions': dict(sorted(Counter(hidden.values()).items())),
        'stereoisomer_groups': len({group for _, group in stereo}),
        'policy': 'CHECKED/OK entries with fixed formulas and positive finite average mass; ChEBI is the sole bundled English chemical source.',
        'excluded': dict(sorted(excluded.items())), 'invalid_cas_references_ignored': rejected_cas,
        'inputs': {name: {'url': 'https://ftp.ebi.ac.uk/pub/databases/chebi/flat_files/' + name,
                          'sha256': hashlib.sha256((directory / name).read_bytes()).hexdigest()}
                   for name in ['compounds.tsv.gz', 'chemical_data.tsv.gz', 'names.tsv.gz', 'database_accession.tsv.gz', 'README', 'LICENSE']},
    }
    manifest['inputs']['chebi_core.obo.gz'] = {
        'url': 'https://www.ebi.ac.uk/chebi/downloads',
        'sha256': hashlib.sha256((directory / 'chebi_core.obo.gz').read_bytes()).hexdigest(),
    }
    (output / 'manifest.json').write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + '\n',
                                         encoding='utf-8', newline='\n')
    (output / 'LICENSE.txt').write_bytes((directory / 'LICENSE').read_bytes())
    print(json.dumps(manifest, ensure_ascii=False, indent=2))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--input-dir', type=Path, required=True)
    parser.add_argument('--output-dir', type=Path, default=ROOT / 'data/source/chebi')
    args = parser.parse_args()
    generate(args.input_dir, args.output_dir)
