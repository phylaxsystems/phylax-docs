# Terminology coherence follow-up

Reviewed on 2026-11-17 against PR #208 head [224fc925ee](https://github.com/phylaxsystems/phylax-docs/commit/224fc925ee36e366f818f8cab4f72ebdb78483ad), following the [request thread](https://phylaxsystems.slack.com/archives/C06T1DLBHGR/p1794880767080659) and repository Diátaxis guidance.

## Scope and naming history

Scanned the reader-facing MDX corpus and navigation, then read the affected explanation, how-to, and reference pages together. The inventory remains 116 content routes. No routes, command names, Solidity identifiers, or upstream skill names are renamed.

Repository history records the shift to Ethereum-first guidance in [ebc5888a3e](https://github.com/phylaxsystems/phylax-docs/commit/ebc5888a3e), policies with AI in [1709166d1c](https://github.com/phylaxsystems/phylax-docs/commit/1709166d1c), circuit-breaker navigation in [e64b82a733](https://github.com/phylaxsystems/phylax-docs/commit/e64b82a733), and Use Cases Book in [c81bcf8e23](https://github.com/phylaxsystems/phylax-docs/commit/c81bcf8e23). The [integration discussion](https://phylaxsystems.slack.com/archives/C06SNPA488J/p1794880151717169) supports distinguishing embedded integration from older sidecar guidance. These are evidence of evolving documentation, not proof of a formal product-wide naming decision.

The broad circuit-breaker definition remains this branch's editorial convention: policy is intent, assertions implement checks, circuit breaker names the protection, and cumulative-flow breaker names its rolling-window subtype.

## Corrections across three passes

| Pass | Drift found | Correction |
| --- | --- | --- |
| Vocabulary and scope | AI authoring and effort estimates treated all requirements as invariants. | Describe failure scenarios, policy requirements, observation boundaries, and permitted exceptions. Keep invariants as one kind of requirement. |
| Vocabulary and scope | The management guide implied every circuit breaker has a window and threshold. | Scope those controls and the screenshot caption to cumulative-flow breakers. |
| Vocabulary and scope | Component prose alternated between enforcement engine, enforcer, and sidecar. | Use Assertion Enforcer for validation, PhEVM for execution, and sidecar for a separate-process integration. |
| Lifecycle and evidence | The enforcer diagram selected deployed assertions; deployment prose blurred confirmation and activation. | Select applicable active assertions; separate deployment confirmation, delay, activation, Production enforcement, and Alerting observation. |
| Lifecycle and evidence | Removal described Alerting assertions as enforced. | Distinguish continued observation from continued enforcement during deactivation delay. |
| Lifecycle and evidence | The overview claimed the linked Balancer candidate demonstrated an exploit replay. | Match the case study's actual evidence boundary: candidate policy, no pinned Phylax replay. |
| Cohesion and reader journeys | Names and distinctions were scattered across explanatory pages. | Add a shared glossary comparison table, link it from introductions, and add contributor terminology rules. |
| Cohesion and reader journeys | Architecture described all L2 material as earlier designs. | Separate current integration planning from explicitly historical examples. |

## Diátaxis and compatibility

- Explanation pages introduce the conceptual distinctions and link to reference definitions.
- The glossary owns the cross-term comparison and observation-boundary definition of invariant.
- Authoring and management how-tos describe tasks and status checks without becoming a second glossary.
- Existing tutorial and reference classifications remain appropriate; no new sidebar category is needed.
- Historical routes, including `/assertions-book/` and `/dapp-*`, remain valid. Renamed effort and AI review headings retain explicit compatibility anchors.
- The upstream `mapping-invariants` skill name, executable identifiers, and examples that actually express invariants retain that wording. This is a semantic review, not a global word replacement.

## Verification

- Inventory regeneration succeeds: 116 pages, 115 sidebar entries, 40 navigation nodes, 58 glossary terms, 25 snippets, 16 redirects.
- All 25 Node tests pass, including navigation, redirects, anchors, and source-integrity checks.
- Mintlify reports no broken links and no image/video attribute issues across 118 MDX files. The existing palette warning remains (primary/light contrast 4.83:1, AA).
- `git diff --check` passes. Executable examples are unchanged.
- Local `pnpm test` reaches the Mintlify build step but cannot complete it: the CLI reports that downloading its validation client requires an internet connection. The preview has the same limitation; this follow-up does not claim local rendered validation. Hosted checks are recorded on the PR after pushing.
- Authenticated platform labels, deployment behavior, and live enforcement remain outside this source review. Earlier rendering results in the main audit describe earlier commits, not this follow-up.
