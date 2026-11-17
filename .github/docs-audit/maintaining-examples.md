# Maintaining example provenance

The upstream Book imports and local documentation fixtures have different owners.

## Upstream imports

`.github/credible-std-import.json` records the source repository, exact commit, directory, and SHA-256 of every generated snippet. The importer checks the checkout's HEAD and refuses to overwrite local snippets or silently remove upstream files.

To review an upgrade:

1. Select an immutable source commit and inspect its changes.
2. Update the manifest revision and check out that exact commit locally.
3. Run `node scripts/import-credible-std.mjs --source /path/to/credible-std`.
4. Review every changed consumer, specification, warning, and version reference. Update evidence only for tests actually run.
5. Run the same command with `--check`, then `pnpm test`.
6. Commit the manifest, generated snippets, and dependent prose together.

The manual import workflow defaults to a dry run. An explicit non-dry run opens a review PR; it does not merge or certify the examples. The workflow no longer follows a moving source branch.

## Locally owned fixtures

`examples/docs` owns the call-input, log-query, and oracle-snapshot fixtures. Their displayed source is mirrored in `snippets/docs-observation-assertions.mdx` and checked for exact equality by the Node tests. The documentation CI runs their PCL tests at the pinned dependency revision.

`.mintignore` excludes `examples/` and `.import-source/` from the docs preview. This prevents recursive library links and build output from entering the site or its file watcher. The checked-in fixture directory contains only source, tests, configuration, and instructions.

A compiled import is not a behavior-tested policy. A synthetic test is not an exploit replay. Neither establishes production runtime compatibility or guard coverage.
