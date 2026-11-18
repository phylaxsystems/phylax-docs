# Diátaxis information architecture and journey review

Reviewed November 18, 2026, starting from `98d7f34`. Applied the repository's [Diátaxis instructions](../../AGENTS.md): classify the reader need, make a local improvement, then retrace the journey. This is a navigation and task-flow review, not a new certification of runtime behavior or every example.

## Pass 1: navigation, page roles, and entry points

Inspected the full navigation and 116-page inventory, then read the landing pages and linked procedures for learners, policy authors, release reviewers, operators, and infrastructure integrators. Retained the existing topic structure. All 116 content routes, 115 sidebar entries, 40 navigation nodes, and 16 redirects remain.

Page roles were checked before editing:

- **Tutorial:** starter quickstart, allowed/rejected-case lesson, Book ownership exercise. Keep concrete outcomes and a bounded exercise.
- **How-to:** authoring, local testing, release preparation, manager assignment, release review/deployment, backtesting, platform operation, AI deployment, and L2 planning. State prerequisites, completion evidence, and the next task.
- **Reference:** CLI and glossary. Update the release-preparation link labels and environment examples without changing their role.
- **Explanation:** introduction and evaluation discussion. Offer relevant task entry points without turning the conceptual overview into a walkthrough.
- **Navigation hubs:** pattern catalog and AI workflow selector. These select documents by reader need; they do not need to imitate a tutorial or force the entire site into four top-level buckets.

| Finding | Resolution |
| --- | --- |
| The front page offered evaluation and a tutorial but no direct path for returning authors or operators | Add authoring and platform-operation cards alongside the existing entry points. |
| The AI setup page appeared as a peer of its two tasks | Make it the group's landing page, consistent with other task groups. |
| Catalog links promised “compose” and “configure” walkthroughs but landed on references | Label the destinations as composition/configuration references. |
| L2 planning ended with Ethereum architecture, Ethereum trust assumptions, and an Ethereum platform walkthrough | Link to sequencer pipelines and the L2 enforcement boundary; require network-specific management and investigation workflows in the completed plan. |

## Pass 2: prerequisites, completion criteria, and handoffs

| Finding | Resolution |
| --- | --- |
| The starter and Book ownership lessons looked interchangeable | Explain the different repositories and outcomes. The starter runs existing tests; the Book adds three tests and exposes an endpoint limitation. Add explicit onward links. |
| The allowed/rejected-case lesson ended by asking the learner to solve a real governance-policy design | End with a concrete requirement and a list of missing inputs. Move implementation decisions into the later adaptation task. |
| Authoring/testing guides lacked a clear entry contract and test-to-release handoff | Add prerequisites, test completion criteria, and direct links to release preparation. Existing Solidity examples and executable fixtures are unchanged. |
| “Apply Assertions” blurred preparation and deployment and linked readers back to the beginning of setup | Rename it “How to Prepare a Policy Release,” verify the pending release, and hand off directly to release checks. Preserve the old Step 4 fragment. The deployment guide links to this owner for configuration. |
| The general platform journey starts with Alerting, but manual configuration examples selected Production | Use Alerting in the how-to examples and explain the explicit Production choice. The CLI reference continues to describe both values; the existing AI deployment example already uses Alerting. |
| Manager assignment required a project, but deployment prerequisites implied the manager must already exist before project creation | Scope the manager requirement to signing and link assignment prerequisites to the existing project-creation section. |
| Alerting and AI deployment readers were sent into production enforcement verification | Add a separate Alerting observation section, retain the existing Production anchor, and link each workflow to the appropriate section. |
| The AI setup hub required authentication before the local authoring workflow | Limit authentication and signer prerequisites to deployment, matching the local-project guide. |

## Pass 3: fresh journey and consistency check

Retraced the edited routes and searched incoming links. Corrected a remaining release-check redirect in the evaluation explanation, updated preparation-card labels in the CLI/backtesting/glossary, and made backtesting return to the remaining release checks. Refreshed the page, navigation, and glossary inventories. No glossary definitions or runtime identifiers changed.

| Reader and entry point | Route through the docs | Completion boundary |
| --- | --- | --- |
| Evaluator: introduction | suitability → integration preparation → integration planning | Named protection need, technical contact, and coverage questions |
| New learner: starter | install → quickstart → allowed/rejected cases | Two existing test outcomes and one recorded exception requirement |
| Learner exploring limits: Book | ownership exercise → state-change reference | Three local outcomes, including a temporary change escaping an endpoint check |
| Policy author: authoring guide | define exceptions → implement → test → prepare release | Pending release with the reviewed environment, targets, and complete policy set |
| Reviewer: platform hub | existing release → checks → backtesting → remaining checks → signing | Reviewed release; no CLI setup required for an already-prepared release |
| Alerting operator | authorize → activation → Alerting observation → investigate | Evidence of exercised policies; no claim of live rejection |
| Production operator | separate Production release → review → authorize → activation → enforcement checks | Integrated guard, routing, assertion-result, and state evidence |
| Maintainer | inspect → complete replacement or removal → review/authorization → delayed effect | Existing complete-release lifecycle remains explicit |
| AI-assisted author | workflow hub → local project → local test evidence | Local work without platform login or signing credentials |
| L2 integrator | L2 overview → plan → sequencer pipelines/boundaries | Network-specific coverage and failure-policy plan, not an Ethereum walkthrough |

The final source pass found no additional blocking handoff issue in these traced journeys. This does not assert that every page or live workflow is defect-free.

## Validation and boundaries

- All 26 existing Node checks pass, including navigation uniqueness, route/redirect compatibility, relative links, preserved sections, and snippet provenance.
- Inventory regeneration succeeds with 116 pages, 115 sidebar entries, 40 navigation nodes, 63 glossary terms, 25 snippets, and 16 redirects.
- Mintlify broken-link and media-alt checks pass. The existing AA/AAA color contrast warning remains; no color changes were made.
- `git diff --check` passes.
- Local `pnpm test` reaches Mintlify build validation but fails because the CLI cannot download its client (`requires an internet connection`). `pnpm dev` fails at the same prerequisite. Fresh rendered desktop/mobile checks could not run; prior rendered checks do not validate this revision. Current-head hosted build results are recorded on the PR.
- No Solidity fixture, assertion implementation, dependency pin, or platform action changed. No new local execution of Solidity examples is claimed.
- F25 remains open: authenticated platform labels, deployment authority, target-runtime compatibility, and live enforcement were not exercised. The new verification sections organize existing documented expectations; they do not establish that a deployment passed them.
- Linear association remains pending after the live owning-source lookup returned HTTP 401. No duplicate issue was created.
