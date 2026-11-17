# Follow-up Diátaxis review

Review date: November 17, 2026. Starting head: `7d8bb41`. Applied [the repository's Diátaxis guidance](../../AGENTS.md) in three successive passes, with fixes followed by another reader-journey check. This supplements the [original finding tracker](README.md).

## Pass 1: Information architecture

Compared the page inventory, sidebar, landing pages, and catalog facets. Kept the existing site sections and URLs while correcting local inconsistencies.

| Finding | Change |
| --- | --- |
| Pattern navigation mixed risks, protocol families, asset types, and emergency modes | Group the 23 pattern entries into five risk domains matching the catalog. Keep protocol compositions and dispatch mechanisms separate. |
| The 19 case studies formed a flat list despite a categorized index | Use the index's five risk groups in the sidebar, with the index as its landing page. |
| Landing pages appeared as ordinary list entries | Make the release, transaction-data, pattern, and case-study hubs group roots. |
| Release backtesting was separated from release review | Put backtesting beside deployment and replacement tasks. |
| Deployment authority explanation lived among integration procedures | Move it beside the trust model. |
| The vault reference was classified as a how-to; authoring and DA classifications also drifted | Classify vault composition as Reference, policy authoring as How-to, and DA as Explanation. Classify the historical manual-registration procedure as How-to. |
| The Book introduction repeated a second, incompatible “Part 2/Part 3” outline | Replace that outline with reader-need cards and a short explanation of the sidebar. |

The refreshed inventory contains 116 content routes, 115 sidebar entries, 40 navigation nodes, 58 glossary terms, 25 snippets, and 16 redirects. No page or redirect destination was removed.

## Pass 2: Page cohesion and reader contracts

| Finding | Change |
| --- | --- |
| Deployment linked to replacement-page anchors that pointed back to deployment | Keep release checks and authorization in the deployment guide; route replacement/removal there without claiming a second screenshot walkthrough. |
| Reviewers were required to complete developer CLI preparation | Add an explicit entry at Step 4 for an already-prepared release; retain preparation steps for its author. |
| Platform cards conflated release review, replacements, and notification setup | Route each task to its owning guide; distinguish Slack delivery from Alerting mode. |
| The Book tutorial's ending switched from learning to production adaptation | Give it the distinct goal “Test the Limits of an Ownership Policy,” conclude with the three observed results, and link to reference and authoring tasks. Remove the unused standalone Foundry prerequisite. |
| Reference navigation said “Use Case Mappings” without identifying the subject | Name it “Transaction Data APIs” and update incoming labels. |
| Storage/call-context references mixed incomplete code, broad guarantees, and future work | Replace incomplete fragments with API, snapshot, interpretation, and coverage tables; link to existing tested fixtures and remove unsupported roadmap promises. |
| State-change sample subtracted one from an empty array length and omitted the initial-state comparison | Remove the faulty executable-looking fragment. Explain baseline-first iteration and show empty, single-write, restored-state, and equal-value outcomes; explicitly identify these as reasoning examples, not tested code. |
| Aave reference repeated approach, features, testing, bugs, and usage without executed evidence | Consolidate into maturity, catalog scope, collections, and validation limits. Keep the intentional-defect warning and identify moving fork links. |
| Historical L2 registration looked like current onboarding | Label the workflow historical and route new integrations through planning. Put experimental/historical status in the Aave and Optimism catalog titles. |

The reference/API descriptions were cross-checked against `PhEvm.sol` and `StateChanges.sol` in credible-std `fe49de6dcddb8ea1f77ea2828a0c9d7d5f923db0`. No executable fixture, imported snippet, or runtime implementation changed in this follow-up.

## Pass 3: Fresh consistency and journey check

A second search after the edits found another obsolete release-review link in the manager guide and old transaction-data labels in the cheatcode overview/reference. Corrected those before validation. Reordered platform cards to follow authorization, review, backtesting, maintenance, investigation, and notification tasks.

Checked these reader journeys:

- New learner: Book introduction → ownership exercise → observed limits → state-change reference or policy authoring.
- Policy author: pattern catalog → risk group → pattern limits; protocol composition remains a separate reference facet.
- Incident reader: case-study hub → risk group → historical mechanism and proposed policy.
- Release reviewer: platform hub → deployment checks → backtesting → authorization; reviewer entry skips CLI preparation.
- Operator: active-policy inspection → replacement/removal; investigation and Slack notifications remain distinct tasks.
- L2 integrator: planning first; historical registration is explicitly conditional.

All 42 categorized pattern/case routes match the links in their corresponding catalog sections. Existing routes and legacy section aliases remain available. Titles and primary reader needs are reflected in the regenerated page inventory.

## Validation and boundaries

- 25 existing Node tests pass: unique navigation, route/redirect compatibility, preserved sections, and snippet provenance.
- Mintlify broken-link and media-alt checks pass. Existing color contrast warning remains: primary/light is AA at 4.83:1, not AAA.
- Prebuild succeeds using the installed schema and cached Mintlify client. Ordinary local validation still fails in the existing `got` / `p-cancelable` network path; it is not recorded as a passing build. The PR review records hosted checks for the pushed head.
- Sampled rendered desktop (1440px) and mobile (390px) layouts include the catalog, platform hub, ownership tutorial, and state-change reference; no page-width overflow was observed. Expanded pattern navigation and canonical release-card destinations were checked in the browser.
- Authenticated platform UI, live deployment compatibility, and historical exploit replay remain outside this source/IA review. Existing F25 still applies. The Aave fork was not executed or newly certified; its moving source links are disclosed.
