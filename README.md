# Welcome to the Phylax docs
Here you'll find everything you need to get started, for every Phylax product

## Setting up

The first step to world-class documentation is setting up your editing environments.

As part of "docs-driven development", we're going to document code
as we commit it and enforce that in the PR approvals flow.

It's easy to add to the docs! Either add markdown/MDX syntax to an
existing file in this repo, or add a new one (feel free to copy from an
existing one of the same style). 

If you're going to document a specific product, follow instructions from its own repo.
It'll get compiled into this repo's `docs-prod` branch before deployment

To add a new file:
 - Create .mynewfile.mdx
 - If you want it to show in the sidebar menu, add it to [Navigation](./mint.json)

## Testing

We don't have a staging site set up for docs yet, so it'll be a bit manual
 - Commit and push the new docs to this repo or a product-specific repo
 - The [github action](https://github.com/phylaxsystems/phylax-docs/actions/workflows/compile-docs.yml) should auto-run
 - run `mintlify dev`


## Phylax Node (FOSS)

## Phylax Node (Hosted)

## Reference Docs

## Assertion Library

## Support and bug reports