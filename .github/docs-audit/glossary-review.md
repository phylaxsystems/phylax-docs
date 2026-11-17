# Glossary disambiguation review

This follow-up evaluates every glossary entry and its related usage, after the [terminology pass](terminology-review.md). It starts from PR #208 commit [6e30ffb](https://github.com/phylaxsystems/phylax-docs/commit/6e30ffb1bc6ac65c13e7ac0f67e81d40ee405932).

## Coverage and decisions

The [usage register](glossary-usage.csv) covers all **63 current terms**: the original 58 entries, with two headings clarified, plus five missing concepts. Each row records the intended usage, the concepts it must not be confused with, and a supporting reference. The generated [glossary snapshot](glossary.csv) retains the actual definitions. These files serve different purposes; update the usage register when glossary terms or their intended meanings change.

The five added entries are **Project**, **Assertion Specification**, **Snapshot**, **Assertion Activation Window**, and **Rolling Window**. **Network Validator (Node)** becomes **Network Validator**, and **Callstack** becomes **Call Trace**. Both old heading anchors remain available.

| Ambiguity | Decision and affected reader path |
| --- | --- |
| Assertion can mean a check or an artifact | Use function, contract, ID, adopter, and release when their units matter. Backtesting now identifies the artifact first and uses the trace to find the failing function. |
| Adopter can sound like every observed contract | Define the assigned address separately from other inspected contracts, the transaction recipient, and guard coverage. Align the API description with that meaning. |
| Window has two unrelated meanings | Separate the State Oracle's activation interval from the rolling flow-measurement window in the glossary and contract reference. |
| Pre/post-state hides its observation boundary | Explicitly name transaction or call boundaries. Align the glossary, snapshot API, and cheatcodes explanation. |
| Trigger match sounds like a rejection | The trigger dispatches a function; the function checks the policy. This preserves the difference between flow/anomaly dispatch and effect checks. |
| Invalidation sounds like an attack or confirmed policy violation | A runtime failure such as out-of-gas can invalidate a candidate too. Align the glossary, enforcement overview, backtesting, and investigation task. |
| Exclusion sounds network-wide | The decision belongs to a participating builder. Exclusion itself uses no onchain gas but does not establish another builder's result. |
| DA signature sounds like permanent availability or proof of execution | Define a stored-artifact attestation at signing time; keep future availability, policy correctness, and builder execution separate. |
| Development tooling sounds like production runtime | Phorge is the Forge-derived tooling behind local PCL build/test; PhEVM is the assertion execution environment. |
| Specification sounds like readiness or lifecycle status | Legacy/Reshiram/Experimental describe API capability selection, independently of activation, tests, and deployment support. |
| Invariant taxonomy mixes scope and purpose | Describe contract scope and observation boundary separately; remove the overlapping single-contract/protocol-wide/security-specific list. |

The broad **circuit breaker** convention remains an editorial choice of this PR, not evidence of a formal product rename. Exact CLI commands, Solidity identifiers, UI field names, imported snippets, and historical integration examples retain their names. The glossary does not declare actual UI labels obsolete.

## Sources and Diátaxis

Definitions were reconciled against the linked docs, then checked against relevant primary implementation sources:

- [State Oracle](https://github.com/phylaxsystems/credible-layer-contracts/blob/4f2723352afa1934af0a2a04b7294767226fbd50/src/StateOracle.sol): adopter assignments and `AssertionWindow` activation/deactivation fields.
- [PhEVM interface at the documented library pin](https://github.com/phylaxsystems/credible-std/blob/fe49de6dcddb8ea1f77ea2828a0c9d7d5f923db0/src/PhEvm.sol): snapshot boundaries and the adopter returned for the current evaluation.
- [PCL 1.9.0 Phoundry dependency](https://github.com/phylaxsystems/pcl/blob/1.9.0/crates/pcl/phoundry/Cargo.toml): Forge-derived build/test tooling.
- [Human integration discussion](https://phylaxsystems.slack.com/archives/C06SNPA488J/p1794880151717169): embedded library direction versus the earlier sidecar approach. Indexed naming searches did not establish any additional formal naming decision.

The glossary and API/contract pages remain **reference**. Conceptual distinctions are reflected in **explanation** pages. Backtesting and investigation remain **how-to guides**, with only the distinctions needed to interpret results. Existing page classifications remain unchanged; there is no new navigation hierarchy.

## Verification boundary

Review passes covered glossary definitions, cross-page term uses, and a final scan for contradictory aliases or scope claims across the 116 content routes. The usage register was checked for exact one-to-one coverage of the 63 glossary headings, and all supporting paths exist. Legacy glossary anchors and all routes are preserved. This is a source consistency review, not a claim that every future sentence will remain consistent.

Build, link, and test results for the resulting commit are recorded on the PR. Authenticated platform labels and runtime behavior remain unverified; implementation source does not establish the deployed revision or configuration. No executable examples or imported snippets change in this follow-up.
