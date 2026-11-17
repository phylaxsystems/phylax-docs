import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';

const root = new URL('../', import.meta.url);
const manifest = JSON.parse(readFileSync(new URL('.github/credible-std-import.json', root)));
test('imported snippets match the reviewed provenance manifest', () => {
  assert.match(manifest.revision, /^[0-9a-f]{40}$/);
  assert.equal(new Set(manifest.files.map(file => file.snippet)).size, manifest.files.length);
  for (const file of manifest.files) {
    const content = readFileSync(new URL(file.snippet, root));
    assert.equal(createHash('sha256').update(content).digest('hex'), file.sha256, file.snippet);
    assert.ok(file.source.startsWith(`${manifest.directory}/`));
  }
});

// Local teaching examples must match the implementation exercised by PCL.
test('observation snippet matches the compiled local fixture', () => {
  const source = readFileSync(new URL('../examples/docs/src/ObservationAssertions.a.sol', import.meta.url), 'utf8').trimEnd();
  const snippet = readFileSync(new URL('../snippets/docs-observation-assertions.mdx', import.meta.url), 'utf8');
  assert.equal(snippet, `\`\`\`solidity\n${source}\n\`\`\`\n`);
});
