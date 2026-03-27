# Mintlify documentation

## Working relationship
- You can push back on ideas-this can lead to better documentation. Cite sources and explain your reasoning when you do so
- ALWAYS ask for clarification rather than making assumptions
- NEVER lie, guess, or make up anything

## Project context
- Format: MDX files with YAML frontmatter
- Config: docs.json for navigation, theme, settings
- Components: Mintlify components
- Documentation model: Diataxis

## Diataxis workflow
- Classify every page before writing or editing it: tutorial, how-to guide, reference, or explanation
- Identify the user need and question before drafting:
  - tutorial: "Can you teach me to...?"
  - how-to guide: "How do I...?"
  - reference: "What is...?" / "What are the facts?"
  - explanation: "Why...?" / "Can you tell me more about...?"
- Use the Diataxis compass when a page is unclear:
  - action + study = tutorial
  - action + work = how-to guide
  - cognition + work = reference
  - cognition + study = explanation
- If the intended page type is still ambiguous after checking the page, neighboring content, and navigation, ask for clarification instead of guessing
- Keep one primary Diataxis form per page. If content mixes forms, split it or link to the other page instead of blending modes
- Apply Diataxis at the page and section level, but do NOT force the entire docs tree into four rigid top-level buckets if that hurts the reader's mental model
- Use Diataxis as a guide to improve docs iteratively, not as a top-down reorganization plan
- Prefer fixing one page, section, or paragraph at a time over sweeping rewrites
- Do NOT create empty tutorial/how-to/reference/explanation sections or placeholder structure just to satisfy the framework

## Diataxis editing loop
- Start with the page or section in front of you instead of redesigning the whole docs set
- Ask:
  - What user need does this serve?
  - Does the language and structure match that need?
  - What is the smallest change that would improve it right now?
- Make the smallest useful change, then stop and reassess
- Treat documentation as complete but never finished: publishable at every stage, always open to further improvement

## Titles and openings
- Make the title reveal the page's Diataxis role and user question
- Tutorial titles should name the concrete thing the reader will build, do, or complete
- How-to titles should usually start with "How to ..."
- Reference titles should match the machinery, command, API, concept, or object being described
- Explanation titles should pass the "About ..." test even if the word "About" is not used
- Open each page with a short lead that confirms what the page helps the reader do, know, or understand

## Organization
- Organize the page around its Diataxis role, not around whatever content happens to exist already
- Tutorials should follow one managed path from start to finish
- How-to guides should be organized around the user's goal, with branching only where real-world conditions require it
- Reference should be organized around the structure of the product or machinery, not the user's journey
- Explanation should be organized around concepts, context, reasons, and tradeoffs rather than procedures
- Use sections and headings to support scanability, but keep the page's mode consistent from top to bottom
- If a section clearly belongs to a different Diataxis form, move it or link out instead of keeping it in place

## Diataxis writing rules

### Tutorials
- Optimize for learning by doing, not coverage
- Keep tutorials concrete, linear, safe, and reliable
- Minimize explanation, options, and branching; link out when deeper context is needed
- Show the learner what they will accomplish
- Deliver visible results early and often, and tell the learner what they should notice
- Ignore alternatives unless they are required for success
- Prefer "In this tutorial, you will build..." over "In this tutorial, you will learn..."
- A good tutorial organization is: goal, prerequisites, step-by-step lesson, expected results, next steps

### How-to guides
- Focus on a specific user goal or problem
- Assume the reader is already competent enough to follow directions
- Keep the page action-oriented; include only the context needed to complete the task
- Title how-to guides explicitly, preferably as "How to..."
- Address real-world conditions and branching where they matter: "if this, then that"
- Describe a logical sequence that reflects both what the user does and how the user thinks through the task
- Seek flow: minimize unnecessary context-switching, repeated setup, and backtracking across tools or concepts
- Omit teaching, long explanation, and exhaustive option lists; link to reference or explanation instead
- A good how-to organization is: goal, prerequisites, steps/decision points, verification, related reference links

### Reference
- Describe the product neutrally and accurately
- Organize reference to mirror the product, API, or system structure
- Use examples to illustrate facts, not to turn reference into a tutorial or how-to guide
- Favor consistency, lists, tables, warnings, signatures, and examples over narrative
- Keep reference austere: describe, do not explain or persuade
- Adopt standard patterns so users can reliably find the same kind of information in the same place
- Use titles that match the machinery being described
- A good reference organization is: object/command/topic, syntax or interface, options/fields, behavior, constraints, examples

### Explanation
- Provide background, reasoning, tradeoffs, and conceptual context
- Answer why questions and connect ideas across the product
- Make connections between related ideas, systems, constraints, and alternatives
- Do not let explanation turn into step-by-step task guidance
- Treat explanation as discussion about a topic; "About ..." is a useful title test
- Include history, rationale, alternatives, constraints, and implications when they help understanding
- Explanation may contain perspective and judgment, but it still needs clear bounds
- A good explanation organization is: topic framing, context/background, reasoning, alternatives/tradeoffs, implications, related links

## Language
- Match the wording to the page type; do not use one generic docs voice for every page
- Tutorials may use light collaborative framing such as "we" in introductions and checkpoints, but keep most instructional language reader-centered and consistent with the repo's default "you" voice
- Tutorials should set expectations explicitly:
  - "In this tutorial, we will ..."
  - "The output should look something like ..."
  - "Notice that ..."
  - "Let's check ..."
- How-to guides should use direct, conditional imperatives:
  - "This guide shows you how to ..."
  - "If you want x, do y."
  - "To achieve w, do z."
- Reference should state facts, lists, constraints, and warnings plainly
- Explanation should use reasoning language, comparisons, and alternatives:
  - "The reason is ..."
  - "X is better than Y when ..."
  - "Some users prefer ..."

## Page design and review
- Keep blur between neighboring forms out of the page:
  - tutorial vs how-to: lesson for study vs directions for work
  - reference vs explanation: facts for use during work vs context for understanding away from the task
- When reviewing a page, check both functional quality and deep quality:
  - functional quality: accuracy, completeness, consistency, precision, usefulness
  - deep quality: flow, clarity of purpose, fit to user needs, anticipation of the reader
- If a page feels awkward, check whether explanation has leaked into action-oriented docs or instruction has leaked into theory docs

## Content strategy
- Document just enough for user success - not too much, not too little
- Prioritize accuracy and usability
- Make content evergreen when possible
- Search for existing content before adding anything new. Avoid duplication unless it is done for a strategic reason
- Check existing patterns for consistency
- For mixed or unfocused pages, make the smallest change that gives the page a clear Diataxis role
- Start by making the smallest reasonable changes
- Prefer links between Diataxis forms over stuffing one page with tutorial + how-to + reference + explanation content

## docs.json

- Refer to the [docs.json schema](https://mintlify.com/docs.json) when building the docs.json file and site navigation
- Use landing pages and groupings to help readers find the right document type, but prefer reader-friendly structure over mechanically forcing Diataxis labels into navigation
- The current repo is organized largely by topic rather than four explicit Diataxis top-level buckets; preserve or refine that only when it improves discoverability
- Landing pages should provide overview and context, not just raw link lists
- If a list of links becomes long, group it into smaller reader-friendly chunks instead of one long flat list
- Aim to keep landing-page lists to a comfortable human-scannable size; around seven items is a good warning threshold
- Use headings plus short introductory text on landing pages so groups of links make sense before the reader clicks

## Frontmatter requirements for pages
- title: Clear, descriptive page title
- description: Concise summary for SEO/navigation

## Writing standards
- Default to second-person voice ("you")
- Tutorials may occasionally use "we" for shared lesson framing, but do not let the page drift into a generic first-person voice
- Prerequisites at start of procedural content
- Test all code examples before publishing
- Match style and formatting of existing pages
- Include both basic and advanced use cases
- Language tags on all code blocks
- Alt text on all images
- Relative paths for internal links

## Git workflow
- NEVER use --no-verify when committing
- Ask how to handle uncommitted changes before starting
- Create a new branch when no clear branch exists for changes
- Commit frequently throughout development
- NEVER skip or disable pre-commit hooks

## Do not
- Skip frontmatter on any MDX file
- Use absolute URLs for internal links
- Include untested code examples
- Make assumptions - always ask for clarification
