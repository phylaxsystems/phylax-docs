#!/usr/bin/env node
import { readFileSync, writeFileSync, readdirSync, existsSync } from 'node:fs';
import { resolve, basename } from 'node:path';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';

const root = resolve(import.meta.dirname, '..');
const manifestPath = resolve(root, '.github/credible-std-import.json');
const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'));
const sourceFlag = process.argv.indexOf('--source');
if (sourceFlag < 0 || !process.argv[sourceFlag + 1]) {
  throw new Error('Usage: node scripts/import-credible-std.mjs --source /path/to/credible-std [--check]');
}
const source = resolve(process.argv[sourceFlag + 1]);
const revision = execFileSync('git', ['rev-parse', 'HEAD'], { cwd: source, encoding: 'utf8' }).trim();
if (revision !== manifest.revision) throw new Error(`Expected ${manifest.revision}, got ${revision}`);
const dirty = execFileSync('git', ['status', '--porcelain', '--untracked-files=all', '--', manifest.directory], { cwd: source, encoding: 'utf8' }).trim();
if (dirty) throw new Error('Source directory has uncommitted changes; import a clean pinned checkout');
const directory = resolve(source, manifest.directory);
const files = readdirSync(directory).filter(name => name.endsWith('.sol')).sort();
const known = new Set(manifest.files.map(file => file.snippet));
const outputs = files.map(name => {
  const snippet = `snippets/${basename(name, '.sol')}.mdx`;
  if (existsSync(resolve(root, snippet)) && !known.has(snippet)) {
    throw new Error(`Refusing to overwrite locally maintained snippet: ${snippet}`);
  }
  const solidity = readFileSync(resolve(directory, name), 'utf8').trimEnd();
  const content = `\`\`\`solidity\n${solidity}\n\`\`\`\n`;
  return { snippet, source: `${manifest.directory}/${name}`, sha256: createHash('sha256').update(content).digest('hex'), content };
});
const removed = [...known].filter(path => !outputs.some(file => file.snippet === path));
if (removed.length) throw new Error(`Review removed upstream snippets and their consumers before importing: ${removed.join(', ')}`);
const next = { ...manifest, files: outputs.map(({ content, ...file }) => file) };
const manifestText = `${JSON.stringify(next, null, 2)}\n`;
if (process.argv.includes('--check')) {
  for (const file of outputs) {
    if (!existsSync(resolve(root, file.snippet)) || readFileSync(resolve(root, file.snippet), 'utf8') !== file.content) {
      throw new Error(`Snippet differs from pinned source: ${file.snippet}`);
    }
  }
  if (readFileSync(manifestPath, 'utf8') !== manifestText) throw new Error('Import manifest needs regeneration');
} else {
  for (const file of outputs) writeFileSync(resolve(root, file.snippet), file.content);
  writeFileSync(manifestPath, manifestText);
}
console.log(`${process.argv.includes('--check') ? 'Verified' : 'Imported'} ${outputs.length} snippets at ${revision}`);
