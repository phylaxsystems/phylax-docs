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
