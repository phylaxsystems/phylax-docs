# Review follow-up — November 18, 2026

This follow-up implements Cindy's inline review and Slack requests, plus the two accuracy findings in review 5959187229. It supersedes earlier audit recommendations to label L2 workflows historical, use circuit breaker as an umbrella term for every policy, and repeat status/specification/evidence banners on example pages. Earlier audit files record the reasoning at their respective revisions.

## Changes by reader need

- **Reference — state-change tracking and cheatcodes:** include the initial value when changes occur, compare adjacent history entries from index 1, distinguish actual writes from returned history, and explain that same-value writes are omitted. The worked table covers no writes, increases, decreases, restoration, and repeated writes.
- **Reference/how-to — PCL apply:** dry-run avoids release creation. Local-only validation requires `project_id`; interactive project selection can authenticate and query the platform before the dry-run return. JSON mode requires the ID.
- **Explanation/reference — overview and glossary:** restore the requested signer-authority paragraph and rolling-window circuit-breaker definition, expand the trigger definition, remove “not an execution network,” and explain assertions as EVM bytecode executed offchain with PhEVM precompiles. Align the introductory terminology and contributor rules.
- **Reference — system requirements:** restore the requested description and builder-network guidance, remove “assertion data endpoints,” and state that assertions can be developed locally and pushed to the platform.
- **Navigation:** restore “Circuit Breakers” and “How Policy Management Works on L2s”; use “Test Policies.” Keep the catalog heading aligned and retain old section anchors.
- **Explanation/how-to — L2 integrations:** remove historical/retired framing from visible titles, navigation, workflow text, and incoming links. Keep network-specific compatibility checks and avoid treating the previous Rollup-Boost setup as verified instructions for a new deployment. L2 integration work remains planned and active in scope.
- **How-to/reference — notifications:** identify Slack and PagerDuty support in the notification guide, platform hub, operator guidance, investigation guide, glossary, and contributor rules. Retain the Slack walkthrough; no unverified PagerDuty UI steps are added.
- **Reference — assertion examples:** remove all 23 repeated Example status / Spec / Evidence paragraphs. Keep shared compilation and fork evidence in the catalog and concrete source limitations in the relevant examples.

## Accuracy sources

- [Pinned runtime implementation and tests](https://github.com/phylaxsystems/credible-sdk/blob/6aca60ab0daa9379d5a9e6dcdd46a064437b44d9/crates/assertion-executor/src/phevm/precompiles/legacy/state_changes.rs): initial value, empty history, and same-value filtering.
- [PCL 1.9.0 apply implementation](https://github.com/phylaxsystems/pcl/blob/1.9.0/crates/pcl/core/src/apply.rs): project resolution precedes the dry-run return.
- [Cindy's inline review](https://github.com/phylaxsystems/phylax-docs/pull/208#pullrequestreview-5965783622) and [originating Slack thread](https://phylaxsystems.slack.com/archives/C06T1DLBHGR/p1794880767080659): product terminology, notification support, L2 framing, and example presentation.

## Validation

All 26 Node checks pass. Inventory regeneration preserves 116 content routes, 115 sidebar entries, 40 navigation nodes, 63 glossary terms, 25 snippets, and 16 redirects. Media accessibility, broken-link checks, and `git diff --check` pass. No assertion source, imported snippet, or executable fixture changes.

Local `pnpm test` reaches Mintlify build validation but cannot download its client (“requires an internet connection”). Hosted checks and any rendered verification are reported in the PR separately. F25's authenticated platform/deployment checks remain outside this follow-up. PagerDuty support is confirmed by the requester; its setup UI was not exercised.
