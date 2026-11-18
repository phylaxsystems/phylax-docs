# credible-std reference coverage review

Scope: the checked-out documentation dependency at `fe49de6dcddb8ea1f77ea2828a0c9d7d5f923db0`. GitHub API queries for the current upstream revision failed through the sandbox proxy, so this is a pinned-source review, not a claim about the latest upstream tree.

The edited library and cheatcode pages are reference pages: the reader needs API names, arguments, behavior, constraints, and available reusable components. The testing-strategy page remains a how-to guide for a protocol considering opting into Ethereum enforcement; its title, description, opening, and inventory entry now state that audience.

## Gaps addressed

- Added `PhEvm.anomalyContext`, `AnomalyContext`, `TriggerRecorder.watchAnomaly`, and the `CredibleTest` anomaly-staging interface, preserving the distinction between source interfaces and runtime support.
- Added the `LogUtils` and `IERC20LogUtils` functions, decoding behavior, and constraints directly on the reference page.
- Added the local backtesting entry points, configuration fields, result counters/categories, FFI/script requirements, and replay limitations. This is distinct from automatic platform-release backtesting.
- Added the protection families: access control, ERC4626 vaults, Safe, lending, perpetuals, swaps, and anomaly policies. Documented composition and trigger-registration requirements; this is a family map, not an exhaustive adapter-field specification.
- Added console and sensitivity utilities, call snapshot constructors, snapshot failure-message customization, and precise state-change/delta helper semantics.

## Checks and limits

- Compared function names in `PhEvm`, `TriggerRecorder`, `Assertion`, `StateChanges`, `Sensitivity`, `CredibleTest`, `ForkUtils`, `LogUtils`, and `IERC20LogUtils` with the two reference pages. All non-private function names are represented; excluded names were a comment-only example and private implementation helpers. This check alone does not establish behavioral completeness.
- Read the affected signatures and implementations. No new executable assertion program or runtime behavior was introduced; added Solidity blocks describe existing interfaces.
- Refreshed the documentation inventory: 116 content routes, 115 sidebar entries, 40 navigation nodes, 63 glossary terms, 25 snippets, 16 redirects.
- All 27 repository tests passed. Mintlify accessibility and broken-link checks passed, with existing dependency-document parsing warnings and contrast recommendations.
- The final Mintlify validation step failed in its HTTP client's `p-cancelable`/`got` request handling (`onCancel` handler attached after settlement). A rendered build has not been verified for this change.
