## 2.0.0 (2023-08-11)

### Fix

- update CLI logic
- **dependencies**: 💫 update crates
- **dependencies**: 💫 update crates
- **dependencies**: 💫 update crates (#24)
- **dependencies**: 💫 update crates

### Refactor

- 🏄🏽 replace watchexec dependency with notify (#32)
- 🏄🏽 replace watchexec dependency with notify

## 1.0.0 (2023-08-05)

### Fix

- 💫 use IsTerminal introduced in Rust 1.70 to replace is-terminal crate
- 💫 replace is-terminal with Rust features introduced in 1.70, add min Rust version to match (#20)
- 💫 replace is-terminal with Rust features introduced in 1.70, add min Rust version to match
- 💫 replace atty with is-terminal to mitigate risk from vulnerability RUSTSEC-2021-0145 (#19)
- 💫 replace atty with is-terminal to mitigate risk from vulnerability RUSTSEC-2021-0145
- 💫 update crates (#18)
- 💫 update crates
- 💫 update title slugify to replace $ symbol (#5)
- 💫 update title slugify to replace $ symbol
- 💫 update crates (#4)
- 💫 update crates
- **dependencies**: 💫 update crates
- 💫 update usage of Astro directives
- **dependencies**: 💫 update crates
- 💫 update HowTo output
- 💫 update div handling
- **dependencies**: 💫 update crate
- 💫 upgrade to clap version 4
- 💫 improve number range formatting
- **dependencies**: 💫 update crates
- **dependencies**: 💫 update crates
- 💫 update crates
- 💫 update HowTo parsing
- **dependencies**: 💫 update crates
- **dependencies**: 💫 update packages
- **dependencies**: 💫 update crate
- ✅ correct typo
- **dependencies**: 💫 update crate
- 💫 add new component parsing
- 💫 remove no longer needed import from Astro output
- **dependencies**: 💫 update crate
- **dependencies**: 💫 update crates
- **dependencies**: 💫 update crates
- 💫 update image sources for Image component
- 💫 update title formatting to generate code element for inline code spans
- 💫 pretty format apostrophes and quotation marks in titles
- 💫 simplify widow formatting code
- 💫 simplify widow formatting code
- 💫 add non-breaking space where title could create a widow
- 🐞 address issue parsing Markdown where an inline code fragment appears after or within an HTML anchor element
- 🐞 address issue parsing Markdown line where emphasis follows an inline code fragment
- 💫 update title slug generation
- 💫 update title slug generation
- 🐞 address potential issue with oredered lisr markup generation
- 💫 simplify ordered list markup generation from Markdown
- 💫 update Heading related markup
- 💫 output headings with component to facilitate anchor link code
- 💫 update heading slugify
- 💫 add heading formatting
- 💫 update escaping of inline code fragments
- 💫 add link icon to external anchor references
- 💫 add logic for parsing ordered lists which do not start at 1
- 💫 update cade escapes
- 💫 add escape for import.meta
- add logic for parsing HTML description list elements
- 💫 simplify Video JSX element parsing
- 💫 add logic for parsing HTML block level comments
- 💫 ignore additional characters in title slugify
- 💫 simplify escaping on inline and fenced code fragments
- 💫 remove trailing white spce from table cells
- 💫 update markup for ordered lists
- 💫 update Astro frontmatter to include slug when Poll component is present
- **parser**: 💫 address issue generating markup for nested unordered lists
- 💫 add logic for parsing of Video JSX components within HowTo components
- **dependencies**: 💫 update crate
- 💫 add logic for parsing inliine code inside a emphasised text span
- 💫 add logic for parsing fenced code block cpations
- **dependencies**: 💫 update crate
- 💫 add logic for parsing tables
- 💫 update title slugify to avoid leading -
- 💫 change highlightLines attribute on fenced code block markup to avoid change on prettier formatting
- 💫 refine title slugify
- 💫 add version cli flag
- 💫 add sanitising of title id slugs
- 💫 add id attribute to headings
- 💫 add parsing of collapsible tag on fenced code blocks
- ✅ correct typo
- **dependencies**: 💫 update crates
- 🐞 address issue parsing HowTo blocks
- 💫 add logic for parsing HTML blocks
- 💫 address issue in parsing headings which include inline code fragments
- 💫 workaround for astro parsing inline code fragments
- 🐞 address issue of parsing inline code in unordered list elements
- 💫 update imports for posts with images
- 💫 reinforce Astro script in fenced code block workaround
- 💫 address issue with Astro workaround from parsing script tags in fenced code blocks
- 💫 add logic for handling HowTo JSX blocks
- 💫 extend workaround for Astro parsgin script tags in fenced code blocks
- 💫 add workaround for Astro parsing import statements in fenced code blocks
- 💫 add workaround for Astro parsing script tags in fenced code blocks
- 💫 add guard for empty inputs list
- 💫 add watch mode monitoring for multiple inputs
- ✅ correct typos
- 💫 add check mode and logic for parsing multiple inputs in single command
- 🐞 address issue parsing html tags not
- 💫 change componentused to render code blocks to bypass Astro code parsing issue
- 💫 add TwitterMessageLink component to imports
- 💫 add question JSON import to posts with Questions component
- 🐞 address issue with imageDate import not being included with Video component
- 💫 update status message
- 🐞 address issue parsing non-acnhor tags commencing a
- 💫 add parsig of first line number for Markdown fenced code block
- 💫 add panic if anchor without href attribute is encountered
- 💫 add logic for Markdown frontmatter recognition
- **dependencies**: 💫 update crates
- 🧑🏽 improve ux when using cli switches
- 💫 improve UX by adding cli options
- 💫 add processing meta display on completion
- 📚 update usage instructions

### Refactor

- 🏄🏽 create new JSXComponentRegister struct and move JSX to new module
- 🏄🏽 simplify block parsing logic
- 🏄🏽 update to facilitate parsing of blocks going forwards
- 🏄🏽 make intesnt int parse inline wrap text clearer
- 🏄🏽 refator parser module to simplift mdx parsing function
- 🏄🏽 replace custom stack components in parser with generic one
- 🏄🏽 simplify mdx file parse function
- 🏄🏽 simplify mdx file parse function
- 🏄🏽 simplify cli watch mode code

### Perf

- 🔥 inline wrap text parse optimisation
- 🔥 add optimisation to title slugify

## 0.1.0 (2022-03-01)

### Feat

- 🌟 add watch mode

### Fix

- 💫 Astro code parsing issue workaround and general Markdown fixes
- 💫 address issue parsing multiline self-closing Poll JSX component tags
- 💫 add logic for parsing Markdown fenced code blocks
- ✅ correct typo
- 🐞 address issue with parsing of multiline self-closing components
- 💫 improve sophistication of JSX component opening tag parsing
- 🐞 address issue of not correctly parsing markdown within list item
- 💫 add MDX component parsing
- 💫 add logic for parsing ordered lists
- 💫 add rudimentary list item support
- 💫 add parsing of anchor tags and insertion of security attributes for external links
- 💫 add logic for parsing inline code wraps and inline emphasis
- 💫 add logic for parsing bold text

### Refactor

- 🏄🏽 simplify JSX component opening tag parsing
- 🏄🏽 simplify JSX component opening tag parsing
- 🏄🏽‍♂️ refactor unordered list handling to pave way for ordered lists
- 🏄🏽 refactor heading and paragraph parsing
- 🏄🏽 separate parser tests to separate file
- 🏄🏽 simplify bold text parsing
