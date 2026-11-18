# Documentation consistency review

Implementation and re-review of the 2026-11-17 audit, against baseline `3e3ca9167feebab92da833fcd8d148f361d29a00`, following [the repository Diátaxis instructions](../../AGENTS.md).

## Original request and scope

Catalog every source page and taxonomy, make the organization consistent, then review currency (including circuit breakers/assertions) and readability. Apply the findings and review against the same criteria.

- [Originating request and implementation follow-ups](https://phylaxsystems.slack.com/archives/C06T1DLBHGR/p1794880767080659)
- [Integration discussion supplied with the request](https://phylaxsystems.slack.com/archives/C06SNPA488J/p1794860382330089)
- The source inventory retains **116 content routes**, with **115 sidebar entries**, **40 navigation nodes**, **63 glossary terms**, **25 snippets** (22 upstream, three local), and **16 redirects**. The original tree was checked against live GitHub; this snapshot describes the branch, not a deployed-site crawl.

## Inventory and maintenance

- [Pages](pages.csv): every page, primary reader need, current navigation, findings and whether this PR edits it. The last column preserves the **baseline assessment before fixes**; it is not a list of unresolved findings.
- [Navigation](navigation.csv), [glossary](glossary.csv), and [redirects](redirects.csv): current structure, definitions and route compatibility.
- [Snippets](snippets.csv): current consumers, source pins and SHA-256 values, including retained unused upstream examples.
- [Incident sources](incident-sources.json): the reviewed evidence for all 19 cases.
- [Example provenance](maintaining-examples.md): import ownership, upgrades and local fixture maintenance.

Run `uv run python scripts/update-docs-inventory.py` to refresh snapshots. New pages require a human Diátaxis classification before regeneration. Baseline notes are retained; update current classifications when a page's reader need changes.

See the [follow-up Diátaxis review](diataxis-review.md) for the navigation, page-cohesion, and reader-journey passes, including corrections discovered after the initial audit.

The [terminology coherence follow-up](terminology-review.md) records the later vocabulary, lifecycle, and cross-page consistency passes, with their own verification boundaries.

The subsequent [glossary disambiguation review](glossary-review.md) evaluates all glossary entries and adds a [usage register](glossary-usage.csv) with one row per term, its confusion risks, and supporting reference. It adds five missing concepts and preserves anchors for two clarified headings.

## Taxonomy conventions

| Dimension | Rule |
| --- | --- |
| Reader need | One primary tutorial, how-to, reference or explanation need per article; hubs route readers to the appropriate form. |
| Terminology | Policy is intent; assertion functions implement checks; assertion contracts are compiled artifacts; releases group checks. Circuit breaker describes protection; cumulative-flow breaker names the rolling-window subtype. |
| Audience | Protocol-owner and developer tasks are separate from builder/sequencer operation. |
| Scope | Ethereum integration, L2 planning, local tests and historical analogies are labelled separately. |
| Risk and mechanism | Risk domain, protocol family and dispatch mechanism are independent facets. Anomaly dispatch is not a risk domain. |
| Status | Lifecycle, specification and evidence are independent. A source directory named production is not a readiness certification. |
| Evidence | Source review, compilation, local behavior tests, exploit replay and live enforcement establish different things. |
| Operations | Alerting mode is separate from Slack notifications; release state is separate from proof of protection. |

## Finding-by-finding disposition

**Addressed** means the documentation defect is corrected with the stated limits. It does not certify every upstream policy or live deployment. Twenty-six findings are addressed in the repository; F25 retains the live verification boundary below. Priorities are editorial order, not security severity.

| ID | Priority | Finding | Status | Resolution or evidence boundary |
| --- | --- | --- | --- | --- |
| F01 | P1 | Enforcer role is conflated with a sidecar deployment | Addressed | Define the enforcer by its role; describe embedded integration and scope operator sizing. Deployment support remains subject to runtime verification. |
| F02 | P1 | OP Stack / Rollup-Boost remains active guidance | Addressed | Remove OP Stack from active navigation; retain a retirement page and section bookmarks. Mark earlier Linea design historical. |
| F03 | P1 | Capability mappings still deny supported snapshots and key discovery | Addressed | Correct snapshot and mapping-key limitations; put supported key discovery alongside other capabilities. Example compilability is tracked as F27. |
| F04 | P1 | KiloEx recommends an example documented as defective | Addressed | Replace the defective displayed ass28 with an explicit post-call snapshot fixture, including excursion-and-restoration rejection tests. KiloEx links to the candidate mechanism without claiming a replay or demonstrated prevention. |
| F05 | P1 | Bybit balance example creates snapshots but never uses them | Addressed | Remove the defective Bybit balance fragment and align state-inspection prose. A tested native-balance replacement is separate work. |
| F06 | P1 | Policy-control guarantees omit administrative powers | Addressed | Replace absolute control promises with configured signers and documented administrative boundaries; no live role configuration is claimed. |
| F07 | P1 | Circuit breaker has incompatible broad and narrow definitions | Addressed | Use circuit breaker for protection, assertion for executable checks, and cumulative-flow breaker for the rolling-window subtype. This is the consistent editorial convention in this PR, not a claim of a separate product decision. |
| F08 | P2 | Assertion, group, ID, and runtime terminology drift | Addressed | Distinguish assertion contract/function/release; correct ID hash inputs and CLI/runtime terminology. |
| F09 | P2 | Tutorials and reference material sit in misleading sections | Addressed | Give all three local tutorials a Tutorials entry; move contracts/triggers and protocol catalogs into reference-oriented navigation; keep flow semantics under Explanation. |
| F10 | P2 | Pattern categories mix mechanisms, assets, and risk domains | Addressed | Separate risk domain, protocol family, dispatch mechanism, specification, lifecycle and evidence. Align the pattern selector, protocol catalogs and case-study risk labels. |
| F11 | P2 | CLI reference and tutorial versions are not a coherent support policy | Addressed | Check PCL 1.9.0 help and published assets; pin compiler/library/starter revisions and publish executed local combinations. Target enforcer compatibility remains explicitly deployment-specific. |
| F12 | P2 | Snippet importer follows a moving branch while prose cites a pin | Addressed | Pin 22 upstream snippets in a commit/hash manifest; reject dirty source and unmanaged overwrite; make imports manual and dry-run by default. Test local snippet/source equality separately. |
| F13 | P1 | Case-study prevention language exceeds demonstrated scope | Addressed | Rework all 19 case studies around source evidence, candidate policy and coverage limits. Remove categorical prevention and unexecuted code fragments; no exploit replay is claimed. |
| F14 | P2 | Historical amounts, dates, and root causes need a provenance pass | Addressed | Record one evidence source per case; correct Balancer rate direction, Kelp verifier/RPC context, Vestra repeated unstaking, recovery versus loss amounts and other unsupported details. Mark two Abracadabra entries as pinned source analysis rather than verified incident reconstruction. |
| F15 | P1 | Short answer treats platform status as proof of protection | Addressed | Short protection answer now requires active assertions, guard coverage, routing and observed enforcement/recovery checks. |
| F16 | P2 | Fuzz example is excluded by its own suggested filter | Addressed | Correct the displayed fuzz-name filter. Execute the documented CI profile configuration: 14 full, 10 unit and four fuzz tests (256 runs each). The unrelated example on the fuzz overview remains illustrative, not execution-certified. |
| F17 | P2 | CI recipe misses relevant changes and is not reproducible | Addressed | Run the CI recipe on every PR; pin PCL and compiler, remove moving-nightly dependence and correct profile inheritance. Add CI for local observation fixtures and provenance checks; hosted execution is reported separately. |
| F18 | P2 | Alerting and release workflows have multiple names and entry points | Addressed | Use Set Up Slack Notifications for delivery configuration; keep Alerting mode distinct. Make Review and Deploy Policies the canonical deployment procedure and link replacement/removal tasks to it. |
| F19 | P2 | Time-sensitive defaults and service claims lack an owner/date | Addressed | Remove unsupported fixed pricing/timing and universal gas defaults; use workload/operator-specific limits. Distinguish a DA storage signature from a future availability guarantee. |
| F20 | P2 | Installation and profile prerequisites are scattered | Addressed | Publish release-asset OS coverage, Homebrew prerequisites and exact dependency pins. Execute the focused starter and Book tutorials; disclose the unrelated starter full-suite failure. |
| F21 | P2 | Reader-facing overview and reference density obscure next actions | Addressed | Review long CLI/cheatcode reference and FAQ against their reader needs; organize reference by API/command and link tasks. Replace repetitive incident narratives and duplicate release instructions with concise, scoped sections. |
| F22 | P3 | Small copy errors reinforce inconsistent naming | Addressed | Normalize task labels, case names, flow terminology, amounts and grammar across edited pages; retain legacy paths for compatibility. |
| F23 | P2 | Ethereum builder guide is not in the sidebar | Addressed | Add the linked-only Ethereum builder guide to Builder Operations. |
| F24 | P2 | Supported, experimental, legacy and historical are inconsistently signposted | Addressed | Apply specification, lifecycle and evidence labels to imported examples; distinguish experimental Aave fixtures and historical Optimism material. Compilation is not presented as behavior testing or production readiness. |
| F25 | P2 | Live UI and deployment support remain unverified | Live verification pending | Desktop/mobile rendering, links and hosted build are checked. Authenticated platform labels/screenshots, live guard configuration and target-enforcer compatibility require a live session/deployment; this PR does not certify them. |
| F26 | P2 | Group labels describe overlapping work and uneven granularity | Addressed | Align tutorial, authoring, testing, release and operator navigation; use separate pattern-risk and protocol-family catalogs. Preserve every existing route and redirect destination. |
| F27 | P1 | Mapping fragments mutate storage inside view functions | Addressed | Replace storage-mutating view fragments with local call-input and log-query implementations. Exact displayed-source equality plus 14 behavior tests cover valid/invalid updates, log filtering and oracle boundaries. |

## Re-review against the original prompt

| Requirement | Result |
| --- | --- |
| Every source page catalogued | 116 unique rows; current navigation, glossary, redirects and all 25 snippets inventoried. Regeneration checks coverage and import hashes. |
| Diátaxis applied | Tutorials have managed local paths; references describe APIs and catalogs; release how-tos link to one canonical procedure; incident cases explain source evidence and tradeoffs. |
| Consistent taxonomy | Shared policy/assertion/release definitions, separate risk/mechanism/status facets, and explicit operator/historical navigation. |
| Current guidance | PCL 1.9.0 help/assets checked; library/compiler pins and import provenance recorded; snapshots/log queries corrected; all 19 case studies source-reviewed. Unsupported service defaults removed. |
| Readability | Concise case-study structure, searchable command/API reference, clear task titles and selection tables; duplicate release instructions removed. Rendering results are recorded below. |

## Validation and limits

- Node tests: **25 pass**, including navigation/redirect/anchor checks, imported hash integrity and exact equality between displayed and executable local source.
- Pinned importer: **22 upstream snippets verified** against clean `credible-std` `fe49de6dcddb8ea1f77ea2828a0c9d7d5f923db0`.
- PCL 1.9.0 / Solidity 0.8.29 / Linux ARM64: **14 local observation tests pass**, including four fuzz tests with 256 runs each; **three Book ownership tests** and **two focused starter ownership tests** pass. The documented CI profiles also execute successfully (14 full / 10 unit / four fuzz).
- The starter's unrelated full suite is **not certified**: 27 pass, one existing PhyLock test fails (`testMagicNumberDrainOver69UserDeposit`). Onboarding explicitly selects the two tested ownership cases.
- Imported Book sources compile at the pin; most individual examples were not behavior-tested. Defective upstream teaching fragments remain labelled as requiring adaptation. Case studies have source evidence and candidate policies, not executed exploit replays.
- Mintlify local prebuild succeeds using the installed schema and cached client. The ordinary CLI build command currently fails in its network dependency (`got` / `p-cancelable`: onCancel handler attached after settlement); this is not counted as a passing build.
- Hosted [documentation and observation-example jobs](https://github.com/phylaxsystems/phylax-docs/actions/runs/44006668894) pass for implementation commit `68e6a19`, including the ordinary `pnpm test` build and x86-64 PCL tests. The final PR review records checks for the latest head.
- The local cached Mintlify client renders the overview, catalog, Kelp case study, cheatcode reference and oracle fixture. Checked desktop layouts at 1280/1440px and mobile at 390px; the sampled pages fit the viewport, and the oracle source resolves through its snippet import. This is a sampled layout review, not a full interactive or authenticated platform test.
- `git diff --check` passes. Existing accessibility warning: primary/light contrast meets AA (4.83:1), not AAA. The local pnpm version also warns about `packageManagerStrictVersion`.
- **F25 remains a live verification boundary:** no authenticated platform session, deployment-specific roles/guards, target-enforcer compatibility or production incident replay was exercised. Screenshots and deployment instructions are not certified by source checks or local tests.
- Linear association remains pending: the owning live search returned HTTP 401; no ticket is inferred.

## PR review follow-up: relative internal links

The reviews on `224fc92` and `353aa0c` identified root-relative Markdown and Card
links that violated `AGENTS.md`. Converted all 138 remaining root-relative page
links across 41 MDX pages, covering the reported links and older occurrences.
Each page retains its catalogued Diátaxis role, prose, destination paths and
fragments. Regenerated the page and glossary inventories.

Added a repository check for root-relative Markdown links, reference-style links
and component `href` values. It scans tracked MDX, excludes image asset paths,
and reports the file and line. Restoring the reviewed cheatcode overview links
makes the check fail; the corrected tree passes all 26 Node tests. A separate
comparison confirms all 138 edited destinations and fragments are unchanged.
Mintlify accessibility and broken-link checks pass, as does `git diff --check`.
Local `pnpm test` reaches build validation, which cannot download the Mintlify
client in this sandbox (`requires an internet connection`); hosted build status
is recorded in the PR. This link correction does not close F25's authenticated
platform and deployment verification boundary.
