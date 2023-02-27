<img src="./images/rodneylab-github-cmessless.png" alt="Rodney Lab c mess less Github banner">

<p align="center">
  <a aria-label="Open Rodney Lab site" href="https://rodneylab.com" rel="nofollow noopener noreferrer">
    <img alt="Rodney Lab logo" src="https://rodneylab.com/assets/icon.png" width="60" />
  </a>
</p>
<h1 align="center">
  cmessless
</h1>

Basic MDX parser written in Rust.

- adds an id to each heading for easy linking,
- reformats headings, replacing hyphens with non-breaking hyphens,
- uses a parser combinator for improved parsing performance: outputs parsed
  output in a dozen milliseconds for input mdx file of ~25 KB
- watch mode to update Astro output as you save markdown,
- escapes code in inline fragments and fenced code blocks.

⛔️ **full Markdown spec not yet implemented!**

Credit to tutorial by Jesse Lawson for initial inspiration:
[https://jesselawson.org/rust/getting-started-with-rust-by-building-a-tiny-markdown-compiler/](https://jesselawson.org/rust/getting-started-with-rust-by-building-a-tiny-markdown-compiler/)
