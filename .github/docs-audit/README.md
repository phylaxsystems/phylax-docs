# Documentation consistency review

This is the first implementation pass from the 2026-11-17 audit, against baseline `3e3ca9167feebab92da833fcd8d148f361d29a00`. The aim is to improve current guidance in reviewable steps, following [the repository Diátaxis instructions](../../AGENTS.md).

## Original request and scope

Catalog every source page and taxonomy, make the organization consistent, then review currency (including circuit breakers/assertions) and readability. Review the changes against those same criteria before calling the work complete.

- [Originating request and implementation follow-up](https://phylaxsystems.slack.com/archives/C06T1DLBHGR/p1794880767080659)
- [Integration discussion supplied with the request](https://phylaxsystems.slack.com/archives/C06SNPA488J/p1794860382330089)
- The baseline covers 116 content pages, 24 snippet files and 16 redirects. It was checked against the live GitHub tree, not a deployed-site crawl.
- This pass retains 116 content routes. The 115 sidebar entries now include the builder guide and exclude the retired OP Stack route. The glossary now has 58 definitions.

## Inventory

- [Pages](pages.csv): every page, primary reader need, current navigation, findings and whether this pass edits it. The last column deliberately preserves the **baseline assessment before fixes**; use the finding status below for current work.
- [Navigation](navigation.csv): all current groups, anchors and landing routes.
- [Glossary](glossary.csv): current terms and full definitions.
- [Redirects](redirects.csv): all published redirects; destinations remain unchanged.
- [Snippets](snippets.csv): all imports and local snippets. Consumer/integrity results are the baseline snapshot; no snippet content changes in this pass.

These inventories are a review snapshot, not generated runtime configuration. Update affected rows when subsequent PRs change the docs.

## Taxonomy rules used in this pass

| Dimension | Rule |
| --- | --- |
| Reader need | One primary tutorial, how-to, reference or explanation need per article; navigation hubs route to several needs. |
| Terminology | Policy describes intent; assertion functions implement checks; assertion contracts are compiled artifacts; releases group deployed checks. The broader circuit-breaker product meaning and cumulative-flow subtype are proposed for review. |
| Audience | Keep protocol-owner and developer tasks distinct from builder/sequencer operation. |
| Scope | Label Ethereum integration, L2 integration planning, local test and historical analogy separately. |
| Risk/mechanism | A risk domain, an asset class and a dispatch mechanism are separate facets; broader catalog alignment remains F10. |
| Status | Historical design, conceptual fragment, correction required, assertion spec and tested version are different properties. |
| Evidence | Local tests, exploit replay and deployed enforcement demonstrate different things. |
| Operations | Alerting environment is distinct from Slack notifications; release state is distinct from proof of protection. |

## Finding-by-finding progress

**Addressed** means the documented inconsistency is corrected in this pass, with the listed limits. **Contained** means misleading guidance is removed or prominently qualified but a working replacement remains. **Partial**, **For review**, and **Open** are unfinished work. Priorities are editorial order, not security severity.

| ID | Priority | Finding | Status | Change or next step |
| --- | --- | --- | --- | --- |
| F01 | P1 | Enforcer role is conflated with a sidecar deployment | Addressed | Define the enforcer by its role; describe embedded integration and scope operator sizing. Deployment support remains subject to runtime verification. |
| F02 | P1 | OP Stack / Rollup-Boost remains active guidance | Addressed | Remove OP Stack from active navigation; retain a retirement page and section bookmarks. Mark earlier Linea design historical. |
| F03 | P1 | Capability mappings still deny supported snapshots and key discovery | Addressed | Correct snapshot and mapping-key limitations; put supported key discovery alongside other capabilities. Example compilability is tracked as F27. |
| F04 | P1 | KiloEx recommends an example documented as defective | Contained | KiloEx now inherits the defective-example warning and conditional policy language. Correct and replay ass28 before presenting it as demonstrated prevention. |
| F05 | P1 | Bybit balance example creates snapshots but never uses them | Addressed | Remove the defective Bybit balance fragment and align state-inspection prose. A tested native-balance replacement is separate work. |
| F06 | P1 | Policy-control guarantees omit administrative powers | Addressed | Replace absolute control promises with configured signers and documented administrative boundaries; no live role configuration is claimed. |
| F07 | P1 | Circuit breaker has incompatible broad and narrow definitions | For review | Align the glossary with the existing introductory product meaning: circuit breaker as protection, assertion as executable check, cumulative-flow breaker as subtype. Confirm naming during PR review. |
| F08 | P2 | Assertion, group, ID, and runtime terminology drift | Addressed | Distinguish assertion contract/function/release; correct ID hash inputs and CLI/runtime terminology. |
| F09 | P2 | Tutorials and reference material sit in misleading sections | Addressed | Give all three local tutorials a Tutorials entry; move contracts/triggers and protocol catalogs into reference-oriented navigation; keep flow semantics under Explanation. |
| F10 | P2 | Pattern categories mix mechanisms, assets, and risk domains | Partial | Separate flow limits from anomaly dispatch. Broader alignment of risk-domain catalogs, protocol families and incident categories remains. |
| F11 | P2 | CLI reference and tutorial versions are not a coherent support policy | Open | Select and exercise a supported CLI/library/spec/runtime matrix. Do not change reference version 1.8.0 to 1.9.0 without checking commands. |
| F12 | P2 | Snippet importer follows a moving branch while prose cites a pin | Open | Pin/import source provenance and update dependent prose in one review. Baseline comparison found no current drift in 22 upstream imports. |
| F13 | P1 | Case-study prevention language exceeds demonstrated scope | Partial | Qualify Book landing/index and KiloEx, Bybit, Bunni, Radiant and UxLink claims. Remaining case-by-case evidence/maturity and pinned replays still needed. |
| F14 | P2 | Historical amounts, dates, and root causes need a provenance pass | Open | Verify incident dates, amounts, metric definitions, primary sources and Balancer price direction before factual replacements. |
| F15 | P1 | Short answer treats platform status as proof of protection | Addressed | Short protection answer now requires active assertions, guard coverage, routing and observed enforcement/recovery checks. |
| F16 | P2 | Fuzz example is excluded by its own suggested filter | Addressed | Match the fuzz command filter to the displayed function name. Source-level discovery mismatch corrected; example execution was not performed. |
| F17 | P2 | CI recipe misses relevant changes and is not reproducible | Open | Repair CI path coverage and pin a validated toolchain; test the profile configuration with that toolchain. |
| F18 | P2 | Alerting and release workflows have multiple names and entry points | Partial | Rename the Slack notification task and separate notification delivery from Alerting mode. Canonical release-procedure consolidation remains. |
| F19 | P2 | Time-sensitive defaults and service claims lack an owner/date | Partial | Replace universal sidecar sizing and fixed backtest range with workload/report-specific guidance. Verify gas limits, commercial claims and timelines with their owners. |
| F20 | P2 | Installation and profile prerequisites are scattered | Partial | State Homebrew prerequisite and project-defined profile selection. Confirm supported OS/install paths and execute onboarding fixtures. |
| F21 | P2 | Reader-facing overview and reference density obscure next actions | Partial | Give the introduction a descriptive title, replace repetitive mapping prose with a selection table, and clarify Book entry paths. Long reference/FAQ and duplicate-release review remains. |
| F22 | P3 | Small copy errors reinforce inconsistent naming | Partial | Fix inflow/outflow copy, counts, Cream grammar and Bybit relative wording. A full prose-casing sweep remains. |
| F23 | P2 | Ethereum builder guide is not in the sidebar | Addressed | Add the linked-only Ethereum builder guide to Builder Operations. |
| F24 | P2 | Supported, experimental, legacy and historical are inconsistently signposted | Partial | Mark historical integrations and defective fragments explicitly. Apply consistent lifecycle, maturity, spec and evidence labels across remaining examples. |
| F25 | P2 | Live UI and deployment support remain unverified | Open | Verify rendered pages, authenticated UI/screenshots and target deployment compatibility. Source validation does not establish these. |
| F26 | P2 | Group labels describe overlapping work and uneven granularity | Partial | Separate tutorials, authoring, testing, release management and operator entry points. Revisit remaining mixed hubs as their contents are consolidated. |
| F27 | P1 | Mapping fragments mutate storage inside view functions | Contained | New review finding: Function Call Inputs and Read Logs mutate storage bookkeeping in view functions. Label as non-compilable fragments; replace with compiled fixtures and behavior tests in a follow-up. |

## Re-review against the original prompt

| Requirement | Result of this pass | Remaining evidence/work |
| --- | --- | --- |
| Every page catalogued | 116 unique page rows; navigation, glossary, redirects and snippets also inventoried | Keep snapshots synchronized with later changes |
| Diátaxis applied | Three tutorials discoverable; contract/trigger references and protocol catalogs correctly placed; explanatory flow semantics separated | Remaining mixed pages need section-level treatment, not a wholesale rewrite |
| Consistent taxonomy | Policy/assertion/artifact/release distinctions and operator/historical navigation clarified | Confirm product umbrella; finish risk-domain and status facets |
| Current guidance | Retire OP Stack setup, describe embedded integration, correct API contradictions and qualify defective examples | CLI/runtime matrix, importer pinning, complete incident provenance and replay validation |
| Readable and legible | Shorter mapping selector, clearer titles, exact task labels, removed broken balance fragment and duplicated copy | Long reference/FAQ cleanup and rendered desktop/mobile review |

## Validation and limits

- `pnpm test`: repository navigation/redirect/anchor tests, Mintlify alt-attribute checks, broken links and build validation.
- `git diff --check`: whitespace validation.
- Inventory coverage: all 116 source routes appear once; all 115 sidebar entries exist; retained redirects and snippet paths exist.
- Existing warnings: primary/light contrast meets AA (4.83:1), not AAA; pnpm ignores `packageManagerStrictVersion` in workspace configuration.
- Source inspection only for assertion examples. This pass adds no new Solidity implementation; it removes or qualifies defective fragments. The corrected fuzz filter is checked against the displayed function name, not an executed fixture.
- Live rendering, authenticated platform labels/screenshots, deployed roles/specification support, historical incident reconstruction and all-example execution remain unverified. These are not silently marked complete by a successful docs build.
- Linear association is pending: the owning live search returned HTTP 401. Continue repository work; do not infer or invent a ticket.
