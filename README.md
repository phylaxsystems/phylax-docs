# Phylax Documentation

This repository contains the official documentation for Phylax's products.

## Documentation Structure

### Credible Layer (`/credible`)

Documentation for the Credible Layer including guides, reference materials, and best practices.

### Assertions Book (`/assertions-book`)

A collection of practical assertion examples and patterns that developers can use as a learning resource and inspiration for building their own assertions.
Each example demonstrates real-world patterns and best practices. The book also includes a collection of previous hacks and detailed explanations of how they could have been prevented using assertions.

## Code Snippet Integration

The documentation integrates code snippets from the [assertions-examples](https://github.com/phylaxsystems/assertions-examples) repository through:

1. Automated GitHub Actions
2. Storage in `/snippets`
3. Mintlify code block references

## Contributing

We welcome contributions to improve our documentation! Here are some ways you can help:

- Add new guides and tutorials
- Improve existing content for clarity
- Add more examples to the Assertions Book

### How to Contribute

1. Fork the repository
2. Create a new branch for your changes
3. Make your changes:
   - Create or modify `.mdx` files in the appropriate directory
   - If adding new pages, add them to `docs.json` to include them in the navigation
   - For code snippets, ensure they are properly referenced from the assertions-examples repo
4. Test your changes:
   - Run `mint dev` to preview locally
   - Check for broken links using `mint check-links`
5. Commit your changes with a clear commit message
6. Push to your fork and create a pull request

## Deployment

Documentation is automatically deployed when changes are pushed to main, compiling all content and integrating code snippets.
