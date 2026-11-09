"""Recognize fixed chemical formulas and compare hydrate atom counts."""
from collections import Counter
import re

ELEMENTS = set('H He Li Be B C N O F Ne Na Mg Al Si P S Cl Ar K Ca Sc Ti V Cr Mn Fe Co Ni Cu Zn Ga Ge As Se Br Kr Rb Sr Y Zr Nb Mo Tc Ru Rh Pd Ag Cd In Sn Sb Te I Xe Cs Ba La Ce Pr Nd Pm Sm Eu Gd Tb Dy Ho Er Tm Yb Lu Hf Ta W Re Os Ir Pt Au Hg Tl Pb Bi Po At Rn Fr Ra Ac Th Pa U Np Pu Am Cm Bk Cf Es Fm Md No Lr Rf Db Sg Bh Hs Mt Ds Rg Cn Nh Fl Mc Lv Ts Og D T'.split())


def fixed_formula(value):
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
