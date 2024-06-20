# Phylax

![phylax cover](https://i.imgur.com/A1SiIfa.png)

Phylax is a powerful tool designed for monitoring blockchains (for now, EVM only). It runs checks against the blockchain state and is tightly coupled with Forge, a slightly forked version of Foundry, used in Phylax.

Phylax is organized into three main task categories:

- **Watchers**: They watch for new blocks on blockchains and emit events when they find a new block.
- **Alerts**: They run assertions against the on-chain state and emit events when the assertions fail.
- **Actions**: They perform off-chain or on-chain actions.

Any task can subscribe to any event using [Triggers](https://graph.phylax.watch/docs/Reference+Documentation/Triggers) and simple user-provided rules. This makes for powerful composability between the tasks.

## Features

- **Integration with Solidity**: Seamlessly work with smart contracts using the phylax-std library. Define alerts (on-chain assertions) and actions (on-chain changes) in Solidity using the familiar framework: [Foundry](https://github.com/foundry-rs/foundry).
- **User Friendly**: Set up monitoring and incident response for your protocol in days rather than weeks. Avoid learning new and proprietary SDKs that could lock you into a micro-SaaS environment. Instead, utilize open protocols and file formats.
- **Highly Expandable**: Phylax is designed from the ground up for integration and expansion. Use the API and Prometheus endpoint to automate or export live blockchain data on the fly. Adding support for a new chain can be done in weeks rather than months.
- **Open Source Software**: Developed and maintained collaboratively by a community dedicated to transparency and innovation. Phylax will always remain free, as in beer, not freedom.

## What you can build

- An alert for the protocol that is triggered when certain protocol invariants don't hold
- An action to pause the protocol if the above alert triggers
- A webhook to Slack so that you can get notified when the alert is triggered
- A [simple data export](https://publish.obsidian.md/phylax/docs/Guides/Monitor/How+to+export+live+blockchain+data) job to quickly get some data into a chart. Phylax makes it trivial to expose blockchain state through it's Prometheus endpoint. Run Prometheus to gather the data and visualize them using Grafana!
- Financial or DeFi alerts for move of important funds
- On-chain assertions about whatever use case you find useful (let us know!)


## Getting Started

Everything phylax-related lives in the [Graph](https://graph.phylax.watch/docs/Get+Started).

A knowledge graph filled with documentation and references on using Phylax and its best practices.

### Docker

```bash
docker run ghcr.io/phylax-systems/phylax
```

### Install from source

```bash
# Install cargo
curl https://sh.rustup.rs -sSf | sh -s -- -y
# Install phylax to ~/.cargo/bin/phylax
cargo install --git https://github.com/phylax-systems/phylax
```

## Running Phylax

## Usage

Read [Getting Started](https://graph.phylax.watch/docs/Get+Started), which has the most up to date on how to quickly get started, configure, and run Phylax.

Basically:

- Write on-chain assertions as foundry fork tests. What information do you want to assert against every X blocks. Make sure to install every required dependency. The tests should be able to run by simply running `forge test` inside the project's directory.
- Write on-chain actions and configure webhooks. What do you think should be done if some assertion is invalidated?
- Configure all this in a very simple yaml file
- Have the yaml file locally or better commit it to a remote git repository
- Run phylax and either point it to the local file or the remote git repository
- ????
- profit

## Contributing

- Read the How to Contribute page in [the docs](https://graph.phylax.watch/docs/Guides/Develop/How+to+Contribute+to+Phylax), which is kept up to date on what areas we need help with. In short:
  - Contribute as a Protocol Engineer with feedback on improving the tools. What features do you need and why?
  - Contribute as a Rust engineer with improvements to the code itself. Phylax is very early in its development, and there is lots of room to improve on its efficiency.
- Read [CONTRIBUTING](./CONTRIBUTING.md) on how to contribute to the code of conduct
- Find the issues that are labeled as `good first issue`
- If you want to chat, join our [Telegram dev channel](ttps://t.me/+m1u_Qz3M33gyN2Y0)

## Documentation

### Setting up
As part of "docs-driven development", we're going to document code
as we commit it and enforce that in the PR approvals flow.

It's easy to add to the docs! Either add markdown/MDX syntax to an
existing file in /docs, or add a new one (feel free to copy from an
existing one of the same style). 

To add a new file:
 - Create ./docs/mynewfile.mdx
 - If you want it to show in the sidebar menu, add it to Navigation in ./mint.js

### Testing

We don't have a staging site set up for docs yet, so it'll be a bit manual
 - Commit and push the new docs to this repo
 - Go to the [github action](https://github.com/phylaxsystems/phylax-docs/actions/workflows/mintlify-compile-docs.yml) for the docs repo
 - At the top of the table showing past workflow runs, on the right side, hit the "Run Workflow" button and run on the main branch
 - cd to the docs repo, checkout the latest from the `docs-prod` branch, and run `mintlify dev` to see the resulting website

## Support

- If you have a bug, open a [GitHub issue](https://github.com/phylax-systems/phylax/issues/new?assignees=&labels=C-bug,S-needs-triage,plane&projects=&template=bug.yml)
- If you want to chat, join our [Telegram dev channel](ttps://t.me/+m1u_Qz3M33gyN2Y0)
- If you have a feature request, create a topic in [GitHub Discussions](https://github.com/phylax-systems/phylax/discussions/new?category=ideas)

## License

This project is being licensed under [GNUv3](./LICENSE).

## Acknowledgments

A special thanks to the contributors and supporters who have made this project possible.
