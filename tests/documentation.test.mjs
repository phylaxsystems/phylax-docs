import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import test from 'node:test';

const root = new URL('../', import.meta.url);
const config = JSON.parse(readFileSync(new URL('docs.json', root), 'utf8'));
const pageExists = (route) => existsSync(new URL(`${route.replace(/^\//, '')}.mdx`, root));
const readPage = (route) => readFileSync(new URL(`${route.replace(/^\//, '')}.mdx`, root), 'utf8');

function* navigationPages(node) {
  if (!node || typeof node !== 'object') return;
  if (typeof node.root === 'string') yield node.root;
  for (const page of node.pages ?? []) {
    if (typeof page === 'string') yield page;
  }
  for (const value of Object.values(node)) {
    if (typeof value === 'object') yield* navigationPages(value);
  }
}

test('navigation pages and group landing pages exist and are listed only once', () => {
  const pages = [...navigationPages(config.navigation)];
  assert.ok(pages.length > 0, 'No navigation pages found');
  const seen = new Set();
  for (const page of pages) {
    assert.ok(pageExists(page), `Missing navigation page: ${page}`);
    assert.ok(!seen.has(page), `Duplicate navigation page: ${page}`);
    seen.add(page);
  }
});

test('redirects have unique sources and point directly to existing pages', () => {
  const sources = new Set();
  for (const { source, destination } of config.redirects) {
    assert.ok(!sources.has(source), `Duplicate redirect: ${source}`);
    assert.ok(!pageExists(source), `Redirect shadows an existing page: ${source}`);
    assert.ok(pageExists(destination), `Missing redirect destination: ${source} -> ${destination}`);
    sources.add(source);
  }
});

// These published routes must survive the Ethereum/L2 split and guide consolidation.
const legacyRoutes = {
  '/credible/network-integration': '/credible/l2-integrations/network-integration',
  '/credible/network-integrations/architecture-linea': '/credible/l2-integrations/architecture-linea',
  '/credible/network-integrations/architecture-op-stack': '/credible/l2-integrations/architecture-op-stack',
  '/credible/manual-verification': '/credible/l2-integrations/manual-verification',
  '/credible/ownership-verification': '/credible/managing-policy-deployments',
  '/credible/write-first-assertion': '/credible/writing-your-own-policies',
  '/credible/invariant-to-assertion': '/credible/writing-your-own-policies',
  '/credible/explorer-security-posture': '/credible/dapp-explorer',
  '/credible/dapp-projects': '/credible/managing-your-circuit-breakers',
  '/credible/dapp-incidents': '/credible/investigate-rejected-transaction',
  '/credible/dapp-invalidations': '/credible/investigate-rejected-transaction',
  '/credible/dapp-integrations': '/credible/setting-up-alerting',
  '/credible/release-review-checks': '/credible/deploy-assertions-dapp',
};

for (const [source, destination] of Object.entries(legacyRoutes)) {
  test(`published route ${source} still reaches its replacement`, () => {
    assert.equal(config.redirects.find((redirect) => redirect.source === source)?.destination, destination);
  });
}

// Preserve bookmarks when a heading changes or its content moves into another guide.
const legacySections = [
  ['credible/deploy-assertions-dapp', 'step-4-deploy-the-assertion', /<span id="step-4-deploy-the-assertion"\s*\/>/],
  ['credible/deploy-assertions-dapp', 'step-5-monitor-your-assertions', /<span id="step-5-monitor-your-assertions"\s*\/>/],
  ['credible/deploy-assertions-dapp', 'assertion-verification', /^### Assertion verification$/m],
  ['credible/deploy-assertions-dapp', 'source-code', /^### Source code$/m],
  ['credible/deploy-assertions-dapp', 'backtesting', /^### Backtesting$/m],
  ['credible/managing-your-circuit-breakers', 'update-or-remove-a-circuit-breaker', /<span id="update-or-remove-a-circuit-breaker"\s*\/>/],
  ['credible/managing-your-circuit-breakers', 'investigate-a-rejected-transaction', /<span id="investigate-a-rejected-transaction"\s*\/>/],
];

for (const [page, anchor, pattern] of legacySections) {
  test(`published section ${page}#${anchor} remains available`, () => {
    assert.match(readPage(page), pattern);
  });
}

test('the former investigation section links to the standalone guide', () => {
  const page = readPage('credible/managing-your-circuit-breakers');
  const section = page.slice(page.indexOf('<span id="investigate-a-rejected-transaction"'));
  assert.match(section, /\]\(\.\/investigate-rejected-transaction\)/);
});
