# Executable documentation fixtures

These synthetic fixtures validate the displayed call-input, log, and oracle-snapshot examples. They do not reproduce a historical exploit or validate production trigger routing.

## Reproduce

Use PCL 1.9.0. From this directory, install the exact dependency revision:

```bash
git clone https://github.com/phylaxsystems/credible-std.git lib/credible-std
git -C lib/credible-std checkout --detach fe49de6dcddb8ea1f77ea2828a0c9d7d5f923db0
git -C lib/credible-std submodule update --init --recursive
pcl test
```

Solidity 0.8.29 and 256 fuzz runs are configured in `foundry.toml`. Validated on Linux ARM64, November 17, 2026: 14 tests pass, including four fuzz tests. The counter tests use bounded uint128 inputs; the oracle tests exercise the 10% boundaries, zero baseline, and temporary excursions. Reverted child-call semantics and production dispatch are outside this fixture's evidence.

The local source is mirrored in `snippets/docs-observation-assertions.mdx`; the Node provenance test checks exact equality so the displayed implementation is the one executed. Update both in the same change.

## Restored case-study examples

`src/cases/` mirrors the complete inline Solidity examples on 17 case-study pages.
`case-pages.json` maps each page to its source; the Node provenance check requires
exact equality. The shared metric assertion is displayed on five pages with
case-specific configuration and limits.

`CaseAssertions.t.sol` adds synthetic checks for temporary implementation changes,
debt/share consistency, per-call metric bounds, cumulative withdrawal error,
recoverable collateral, share dilution, zero-share mints, and repeated unstaking.
On November 18, 2026, PCL 1.9.0 and Solidity 0.8.29 compiled all 13 case-study
implementations and passed 52 tests: 38 case-study tests plus the existing 14
observation tests. Five tests use 256 fuzz runs each. This run includes
`CaseAuthoritiesAndSolvency.t.sol` for authority, threshold, cook, donation, and
reward-payout checks.

All case-study checks dispatch once at transaction end. Checks concerning
individual operations scan the recorded calls and read each call's snapshots;
they do not rely only on the transaction endpoints. This also avoids executing
the same whole-transaction scan once per matching call.

Protocol-specific reader implementations, production dispatch, and historical
exploit replay are outside these synthetic tests. Bunni and GMX V2 examples
require readers adapted to the actual protocol; the fixtures do not supply those
integrations. A successful fixture does not independently verify a deployed
contract's ABI, storage layout, or economics.
