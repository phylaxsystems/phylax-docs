"""Refresh audit snapshots; preserve human Diataxis classifications and baseline notes.

Run from any directory with: uv run python scripts/update-docs-inventory.py
New content pages require a human classification in pages.csv before regeneration.
"""
from pathlib import Path
import csv
import hashlib
import json
import re
import subprocess

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / '.github/docs-audit'
BASELINE = '3e3ca9167feebab92da833fcd8d148f361d29a00'
config = json.loads((ROOT / 'docs.json').read_text())


def write_csv(name, fields, rows):
    with (OUT / name).open('w', newline='') as stream:
        writer = csv.writer(stream, lineterminator='\n')
        writer.writerow(fields)
        writer.writerows(rows)


navigation, nodes = {}, []


def walk(node, parents=()):
    if not isinstance(node, dict):
        return
    label = node.get('anchor') or node.get('group')
    chain = parents + ((label,) if label else ())
    hierarchy = ' > '.join(chain)
    if label:
        kind = 'external' if node.get('href', '').startswith('https://') else ('anchor' if 'anchor' in node else 'group')
        nodes.append([hierarchy, kind, node.get('root', ''), node.get('href', '')])
    if node.get('root'):
        navigation[node['root']] = hierarchy
    for item in node.get('pages', []):
        if isinstance(item, str):
            navigation[item] = hierarchy
        else:
            walk(item, chain)
    for key in ('anchors', 'groups'):
        for item in node.get(key, []):
            walk(item, chain)
    if 'global' in node:
        walk(node['global'], chain)


walk(config['navigation'])
with (OUT / 'pages.csv').open() as stream:
    pages = list(csv.DictReader(stream))
paths = sorted(str(p.relative_to(ROOT)) for directory in ('credible', 'assertions-book') for p in (ROOT / directory).rglob('*.mdx'))
if set(paths) != {row['path'] for row in pages} or len(paths) != len(pages):
    raise SystemExit('Update page classifications in pages.csv for new or removed routes first')
changed = set(subprocess.check_output(['git', 'diff', '--name-only', BASELINE], cwd=ROOT, text=True).splitlines())
for row in pages:
    text = (ROOT / row['path']).read_text()
    row['current_title'] = re.search(r'^title:\s*(.+)$', text, re.M)[1].strip('\"\'')
    row['current_navigation'] = navigation.get(row['path'][:-4], 'Retired route; retained outside sidebar')
    row.pop('edited_in_first_pass', None)
    row['edited_in_pr'] = row['path'] in changed
    if row['path'].startswith('assertions-book/previous-hacks/') and not row['path'].endswith('prev-hacks-index.mdx'):
        row['primary_reader_need'] = 'Explanation'
fields = ['id', 'path', 'current_title', 'primary_reader_need', 'current_navigation', 'edited_in_pr', 'findings', 'baseline_assessment_before_fixes']
write_csv('pages.csv', fields, [[row[k] for k in fields] for row in pages])
write_csv('navigation.csv', ['current_hierarchy', 'kind', 'landing_route', 'external_destination'], nodes)
write_csv('redirects.csv', ['source', 'destination'], [[r['source'], r['destination']] for r in config['redirects']])
glossary, section = [], ''
for match in re.finditer(r'^(##|###) (.+)\n([\s\S]*?)(?=^## |^### |\Z)', (ROOT / 'credible/glossary.mdx').read_text(), re.M):
    if match[1] == '##':
        section = match[2]
    else:
        glossary.append([section, match[2], match[3].strip()])
write_csv('glossary.csv', ['section', 'term', 'current_definition'], glossary)
manifest = json.loads((ROOT / '.github/credible-std-import.json').read_text())
imports = {f['snippet']: f for f in manifest['files']}
snippets = []
for path in sorted((ROOT / 'snippets').glob('*.mdx')):
    relative = str(path.relative_to(ROOT))
    consumers = [page for page in paths if f'/{relative}' in (ROOT / page).read_text()]
    imported = imports.get(relative)
    digest = hashlib.sha256(path.read_bytes()).hexdigest()
    if imported and digest != imported['sha256']:
        raise SystemExit(f'Provenance mismatch: {relative}')
    snippets.append([relative, 'Pinned upstream Solidity' if imported else 'Locally maintained', '; '.join(consumers), manifest['revision'] if imported else '', imported['source'] if imported else '', digest])
write_csv('snippets.csv', ['path', 'origin', 'current_consumers', 'source_revision', 'source_path', 'sha256'], snippets)
missing = set(navigation) - {p[:-4] for p in paths}
if missing:
    raise SystemExit(f'Missing navigation routes: {missing}')
print(f'{len(pages)} pages; {len(navigation)} sidebar entries; {len(nodes)} navigation nodes; {len(glossary)} glossary terms; {len(snippets)} snippets; {len(config["redirects"])} redirects')
