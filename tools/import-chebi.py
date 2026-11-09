"""Build language-separated Solcalc sources from an official ChEBI flat-file release.

No network access; pass a directory containing the downloaded gzip files,
README and LICENSE. The original Solcalc sources are never written.
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

ROOT = Path(__file__).resolve().parents[1]
ACTIVE = {'1', '3'}  # ChEBI CHECKED / OK; not SUBMITTED or DEPRECATED.
ELEMENTS = set('H He Li Be B C N O F Ne Na Mg Al Si P S Cl Ar K Ca Sc Ti V Cr Mn Fe Co Ni Cu Zn Ga Ge As Se Br Kr Rb Sr Y Zr Nb Mo Tc Ru Rh Pd Ag Cd In Sn Sb Te I Xe Cs Ba La Ce Pr Nd Pm Sm Eu Gd Tb Dy Ho Er Tm Yb Lu Hf Ta W Re Os Ir Pt Au Hg Tl Pb Bi Po At Rn Fr Ra Ac Th Pa U Np Pu Am Cm Bk Cf Es Fm Md No Lr Rf Db Sg Bh Hs Mt Ds Rg Cn Nh Fl Mc Lv Ts Og D T'.split())
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


def fixed_formula(value):
    """Reject repeat units, R groups, wildcards, and undefined stoichiometry."""
    if not value or len(value) > 200:
        return False
    tokens = re.findall(r'[A-Z][a-z]?|[0-9]+|[()[\].·]', value)
    if ''.join(tokens) != value:
        return False
    stack = []
    atoms = 0
    for token in tokens:
        if token in ('(', '['):
            stack.append(token)
        elif token in (')', ']'):
            if not stack or stack.pop() != {')': '(', ']': '['}[token]:
                return False
        elif token[0].isalpha():
            if token not in ELEMENTS:
                return False
            atoms += 1
        elif token.isdigit() and int(token) <= 0:
            return False
    return not stack and atoms > 0


def atom_counts(formula):
    """Compare curated hydrate notation with ChEBI's total-atom formulas."""
    if not fixed_formula(formula) or '[' in formula or ']' in formula:
        return None
    total = Counter()
    for part in re.split('[.·]', formula):
        tokens = re.findall(r'[A-Z][a-z]?|[0-9]+|[()]', part)
        prefix = int(tokens.pop(0)) if tokens and tokens[0].isdigit() else 1
        stack = [Counter()]
        index = 0
        while index < len(tokens):
            token = tokens[index]
            index += 1
            if token == '(':
                stack.append(Counter())
                continue
            if token == ')':
                group = stack.pop()
            elif token in ELEMENTS:
                group = Counter({token: 1})
            else:
                return None
            count = int(tokens[index]) if index < len(tokens) and tokens[index].isdigit() else 1
            if index < len(tokens) and tokens[index].isdigit():
                index += 1
            stack[-1].update({atom: n * count for atom, n in group.items()})
        if len(stack) != 1:
            return None
        total.update({atom: n * prefix for atom, n in stack[0].items()})
    return total


def rows(directory, name):
    with gzip.open(directory / (name + '.tsv.gz'), 'rt', encoding='utf-8', newline='') as file:
        yield from csv.DictReader(file, delimiter='\t')


def write_tsv(path, columns, records):
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open('w', encoding='utf-8', newline='') as file:
        writer = csv.writer(file, delimiter='\t', lineterminator='\n')
        writer.writerow(columns)
        writer.writerows(records)


def generate(directory, output, curated_path=None):
    readme = (directory / 'README').read_text(encoding='utf-8')
    release = re.search(r'ChEBI Release:\s*(\d+)', readme).group(1)
    date = re.search(r'Date of last update:\s*([0-9-]+)', readme).group(1)
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
    with (curated_path or ROOT / 'data/source/main.en.tsv').open(encoding='utf-8', newline='') as file:
        curated_by_cas = {r['cas']: r for r in csv.DictReader(file, delimiter='\t')}
    curated_cas = set(curated_by_cas)
    records = []
    curated_links = []
    locales = defaultdict(list)
    excluded = Counter()
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
        if cas[identity] & curated_cas:
            # Keep every original identity/name/formula/mass and its RU locale.
            excluded['existing_curated_cas'] += 1
            targets = {curated_by_cas[value]['id']: curated_by_cas[value] for value in cas[identity] & curated_cas}
            if len(targets) == 1:
                target, original = next(iter(targets.items()))
                parsed = atom_counts(formula)
                if parsed is not None and parsed == atom_counts(original['formula']):
                    alternatives = sorted(names[identity]['en'] | {clean(compound['name'])})
                    curated_links.append([target, identity, json.dumps(alternatives, ensure_ascii=False), json.dumps(sorted(cas[identity]), ensure_ascii=False)])
                    for locale, values in names[identity].items():
                        if locale != 'en' and values:
                            translated = sorted(values, key=lambda s: (s.casefold(), s))
                            locales[locale].append([target, translated[0], json.dumps(translated[1:], ensure_ascii=False)])
            continue
        name = clean(compound['name'])
        if not name:
            excluded['missing_name'] += 1
            continue
        synonyms = sorted(names[identity]['en'] - {name}, key=lambda s: (s.casefold(), s))
        references = sorted(cas[identity])
        records.append([identity, references[0] if references else '', json.dumps(references, ensure_ascii=False), formula, name, repr(mass), json.dumps(synonyms, ensure_ascii=False)])
        for locale, alternatives in sorted(names[identity].items()):
            if locale == 'en' or not alternatives:
                continue
            alternatives = sorted(alternatives, key=lambda s: (s.casefold(), s))
            locales[locale].append(['chebi:' + identity, alternatives[0], json.dumps(alternatives[1:], ensure_ascii=False)])
    write_tsv(output / 'main.en.tsv', ['chebi_id', 'cas', 'cas_numbers', 'formula', 'name', 'molar_mass', 'synonyms'], records)
    write_tsv(output / 'curated-links.tsv', ['substance_id', 'chebi_id', 'synonyms', 'cas_numbers'], curated_links)
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
        'curated_alias_links': len(curated_links),
        'policy': 'CHECKED/OK entries with fixed formulas and positive finite average mass; original Solcalc CAS records take precedence.',
        'excluded': dict(sorted(excluded.items())), 'invalid_cas_references_ignored': rejected_cas,
        'inputs': {name: {'url': 'https://ftp.ebi.ac.uk/pub/databases/chebi/flat_files/' + name,
                          'sha256': hashlib.sha256((directory / name).read_bytes()).hexdigest()}
                   for name in ['compounds.tsv.gz', 'chemical_data.tsv.gz', 'names.tsv.gz', 'database_accession.tsv.gz', 'README', 'LICENSE']},
    }
    (output / 'manifest.json').write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    (output / 'LICENSE.txt').write_bytes((directory / 'LICENSE').read_bytes())
    print(json.dumps(manifest, ensure_ascii=False, indent=2))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--input-dir', type=Path, required=True)
    parser.add_argument('--output-dir', type=Path, default=ROOT / 'data/source/chebi')
    args = parser.parse_args()
    generate(args.input_dir, args.output_dir)
