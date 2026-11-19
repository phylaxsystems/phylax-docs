# Content restoration and customer review — November 18, 2026

## Reader needs and changes

Case studies remain explanations of an attack mechanism and the corresponding policy, with the complete implementation on the page. Restored all 19 previous-hack pages: 17 now display 13 complete local Reshiram assertions; the two prose-only cases retain their mechanism and policy detail. Sources, configuration assumptions, and integration limits stay beside the examples. Links supplement the answer. Retained the original source and transaction links alongside the inline explanations.

The shared assertion-specification reference now lives in the credible-std overview, outside L2 compatibility. L2 pages use enforcement terminology, retain integration planning, and remove historical-design tags and outdated video walkthroughs. Linea names remain only where identifying the actual Besu architecture case, not as a synonym for L2 compatibility.

The agentic project/deployment how-tos now carry network selection through release preparation. Protocol Manager assignment follows project creation and precedes release creation. The dapp overview states that engineering support for deploying policies is included in enforcement pricing. Slack and PagerDuty support and removal of repeated example-status banners were handled in the preceding review commit.

## Latest inline review

- Restored the complete `Should you use Phylax?` page to the PR base, as requested.
- Restored the FDE introduction in `Work with Phylax` and the circuit-breaker wording for both cumulative-flow trigger descriptions.
- `examples/docs/.gitignore` is repository plumbing: it excludes downloaded libraries and generated build/cache files from commits. It is not a published documentation page.

## Other places where the answer was replaced with a handoff

These are surfaced for editorial review, not represented as repaired by this change:

| Page | What the reader still needs |
| --- | --- |
| `credible/use-case-mappings/storage-lookups.mdx` | The rewrite removed the inline implementation. Restore a complete Reshiram storage-read policy beside the `loadStateAt` explanation instead of relying on the ownership tutorial. |
| `credible/use-case-mappings/call-frame-context.mdx` | The table explains context fields, but the full policy was removed and the reader is sent to the oracle fixture. Put a complete call-context/snapshot example on this page. |
| `credible/use-case-mappings/state-changes.mdx` | The corrected history semantics are useful, but a reader still has no executable policy using them. Add an inline example covering initial values and same-value writes. |

Function-call inputs and log queries already render their shared source inline; these are not link-only handoffs. Protocol catalogs link to larger upstream suites and are a separate content choice.

## Evidence

- All 13 restored case-study implementations compile with PCL 1.9.0 / Solidity 0.8.29 and pinned credible-std `fe49de6dcddb8ea1f77ea2828a0c9d7d5f923db0`.
- 52 local Solidity tests pass: 38 case-study tests and 14 existing observation tests. Five fuzz tests run 256 cases each.
- Exact source/display equality covers all 17 pages, including the metric assertion reused on five pages.
- The full documentation check passes 27 Node tests plus Mintlify accessibility, links, and build checks. The palette produces existing contrast warnings.
- These are synthetic behavior tests, not exploit replays. Deployed ABIs, storage layouts, protocol economics, production dispatch, and authenticated platform behavior remain outside the evidence. Bunni and GMX V2 require protocol-specific reader adapters.

Fresh browser verification was unavailable: the sandbox has no Chrome installation, and the browser installer does not provide a Linux ARM64 build. Local Mintlify build validation passes; no new rendered or authenticated-platform check is claimed.
