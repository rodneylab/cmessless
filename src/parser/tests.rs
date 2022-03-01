use crate::parser::{
    discard_leading_whitespace, form_code_fragment_component_first_line, form_code_span_line,
    form_html_anchor_element_line, form_inline_wrap_text, form_ordered_list_line,
    parse_heading_text, parse_href_scheme, parse_html_tag_attribute, parse_html_tag_attributes,
    parse_inline_wrap_segment, parse_inline_wrap_text, parse_jsx_component,
    parse_jsx_component_first_line, parse_mdx_line, parse_opening_html_tag,
    parse_ordered_list_text, parse_unordered_list_text, parse_up_to_inline_wrap_segment,
    parse_up_to_opening_html_tag, segment_emphasis_line, segment_strong_emphasis_line, JSXTagType,
    LineType,
};
use nom::{
    error::{Error, ErrorKind},
    Err,
};

#[test]
pub fn test_discard_leading_whitespace() {
    let mdx_line = "   NewTech was first set up to solve the common problem coming up for identifiers in computer science.  ";
    assert_eq!(
            discard_leading_whitespace(mdx_line),
            Ok(("","NewTech was first set up to solve the common problem coming up for identifiers in computer science.  "))
        );
}

#[test]
pub fn test_form_code_fragment_component_first_line() {
    let mdx_line = "<CodeFragment";
    assert_eq!(
        form_code_fragment_component_first_line(mdx_line),
        Ok((
            "",
            (
                String::from("<CodeFragment"),
                LineType::CodeFragmentOpening,
                0
            )
        ))
    );

    let mdx_line = "<CodeFragment count={3} >";
    assert_eq!(
        form_code_fragment_component_first_line(mdx_line),
        Ok((
            "",
            (
                String::from("<CodeFragment count={3} >"),
                LineType::CodeFragmentOpen,
                0
            )
        ))
    );

    let mdx_line = "<CodeFragment count={3} />";
    assert_eq!(
        form_code_fragment_component_first_line(mdx_line),
        Ok((
            "",
            (
                String::from("<CodeFragment count={3} />"),
                LineType::CodeFragment,
                0
            )
        ))
    );
}

#[test]
pub fn test_form_html_anchor_element_line() {
    // adds rel and target attributes for external sites when they are not already there
    let mdx_line = "<a href=\"https://www.example.com\">site</a>.";
    assert_eq!(
            form_html_anchor_element_line(mdx_line),
            Ok((
                "site</a>.",
                String::from(
                    "<a href=\"https://www.example.com\" target=\"_blank\" rel=\"nofollow noopener noreferrer\">"
                )
            ))
        );

    // does not add rel and target attributes to non external sites
    let mdx_line = "<a href=\"/home/contact-us\">site</a>.";
    assert_eq!(
        form_html_anchor_element_line(mdx_line),
        Ok(("site</a>.", String::from("<a href=\"/home/contact-us\">")))
    );
}

#[test]
pub fn test_form_code_span_line() {
    let mdx_line = "NewTech `console.log(\"made it here\")` first set up to solve the common problem coming up for identifiers in computer science.";
    assert_eq!(
            form_code_span_line(mdx_line),
            Ok((" first set up to solve the common problem coming up for identifiers in computer science.",String::from("NewTech <code>console.log(\"made it here\")</code>")))
        );
}

#[test]
pub fn test_form_inline_wrap_text() {
    // does not create paragraph tags for empty line
    let mdx_line = "";
    assert_eq!(
        form_inline_wrap_text(mdx_line),
        Ok(("", (String::from(""), LineType::Paragraph, 0)))
    );

    // adds paragraph tags for non-empty line
    let mdx_line = "NewTech was first set up to solve the common problem coming up for identifiers in computer science.";
    assert_eq!(
        form_inline_wrap_text(mdx_line),
        Ok(("", (String::from("<p>NewTech was first set up to solve the common problem coming up for identifiers in computer science.</p>"), LineType::Paragraph, 0)))
    );
}

#[test]
pub fn test_form_ordered_list_line() {
    // does not create paragraph tags for empty line
    let mdx_line = "1. first things first";
    assert_eq!(
        form_ordered_list_line(mdx_line),
        Ok((
            "",
            (
                String::from("<li>first things first</li>"),
                LineType::OrderedListItem,
                0
            )
        ))
    );

    let mdx_line = "1. first things **before** second things";
    assert_eq!(
        form_ordered_list_line(mdx_line),
        Ok((
            "",
            (
                String::from("<li>first things <strong>before</strong> second things</li>"),
                LineType::OrderedListItem,
                0
            )
        ))
    );
}

#[test]
pub fn test_parse_href_scheme() {
    let href = "https://example.com/home";
    assert_eq!(
        parse_href_scheme(href),
        Ok(("example.com/home", "https://"))
    );

    let href = "/home";
    assert_eq!(
        parse_href_scheme(href),
        Err(Err::Error(Error::new(href, ErrorKind::Tag)))
    );
}

#[test]
pub fn test_parse_html_tag_attribute() {
    let attribute = "href=\"https://example.com\"";
    assert_eq!(
        parse_html_tag_attribute(attribute),
        Ok(("", ("href", "https://example.com")))
    );

    let attribute = "aria-label=\"Open our website homepage\"";
    assert_eq!(
        parse_html_tag_attribute(attribute),
        Ok(("", ("aria-label", "Open our website homepage")))
    );
}

#[test]
pub fn test_parse_html_tag_attributes() {
    let attributes = "href=\"https://example.com\" target=\"_blank\"";
    let (_, result) = parse_html_tag_attributes(attributes).unwrap();
    assert_eq!(result.len(), 2);
    assert_eq!(result[0], ("href", "https://example.com"));
    assert_eq!(result[1], ("target", "_blank"));
}

#[test]
pub fn test_parse_jsx_component() {
    let mdx_line = "<Questions {questions} />";
    assert_eq!(
        parse_jsx_component(mdx_line, "Questions"),
        Ok(("", " {questions} "))
    );
}

#[test]
pub fn test_parse_jsx_component_first_line() {
    let mdx_line = "<CodeFragment";
    assert_eq!(
        parse_jsx_component_first_line(mdx_line, "CodeFragment"),
        Ok(("", ("<CodeFragment", &JSXTagType::Opened)))
    );

    let mdx_line = "<CodeFragment count={3} >";
    assert_eq!(
        parse_jsx_component_first_line(mdx_line, "CodeFragment"),
        Ok(("", ("<CodeFragment count={3} >", &JSXTagType::Closed)))
    );

    let mdx_line = "<CodeFragment count={3} />";
    assert_eq!(
        parse_jsx_component_first_line(mdx_line, "CodeFragment"),
        Ok(("", ("<CodeFragment count={3} />", &JSXTagType::SelfClosed)))
    );
}

#[test]
pub fn test_parse_mdx_line() {
    let mdx_line = "# Getting Started with NewTech  ";
    assert_eq!(
        parse_mdx_line(mdx_line, &None),
        Some((
            String::from("<h1>Getting Started with NewTech  </h1>"),
            LineType::Heading,
            1
        ))
    );

    let mdx_line = "### What Does All This Mean?";
    assert_eq!(
        parse_mdx_line(mdx_line, &None),
        Some((
            String::from("<h3>What Does All This Mean?</h3>"),
            LineType::Heading,
            3
        ))
    );

    let mdx_line = "NewTech was first set up to solve the common problem coming up for identifiers in computer science.";
    assert_eq!(
            parse_mdx_line(mdx_line, &None),
            Some((String::from("<p>NewTech was first set up to solve the common problem coming up for identifiers in computer science.</p>"),
                LineType::Paragraph, 0))
        );
}

#[test]
pub fn test_parse_heading_text() {
    let heading_mdx = "# Getting Started with NewTech  ";
    assert_eq!(
        parse_heading_text(heading_mdx),
        Ok(("Getting Started with NewTech  ", 1))
    );

    let heading_mdx = "### What Does All This Mean?";
    assert_eq!(
        parse_heading_text(heading_mdx),
        Ok(("What Does All This Mean?", 3))
    );
}

#[test]
pub fn test_parse_inline_wrap_segment() {
    let mdx_line =
        "first** set up to solve the common problem coming up for identifiers in computer science.";
    assert_eq!(parse_inline_wrap_segment(mdx_line, "**"), Ok(("", ("first", " set up to solve the common problem coming up for identifiers in computer science."))));
}

#[test]
pub fn test_parse_up_to_inline_wrap_segment() {
    let mdx_line = "NewTech was **first** set up to solve the common problem coming up for identifiers in computer science.";
    assert_eq!(parse_up_to_inline_wrap_segment(mdx_line, "**"), Ok(("", ("NewTech was ", "first** set up to solve the common problem coming up for identifiers in computer science."))));
}

#[test]
pub fn test_parse_inline_wrap_text() {
    let mdx_line = "NewTech was **first** set up to solve the **common problem** coming up for identifiers in computer science.";
    assert_eq!(parse_inline_wrap_text(mdx_line), Ok(("", String::from("NewTech was <strong>first</strong> set up to solve the <strong>common problem</strong> coming up for identifiers in computer science."))));

    let mdx_line = "NewTech was first set up to solve the common problem coming up for identifiers in computer science.";
    assert_eq!(parse_inline_wrap_text(mdx_line), Ok(("", String::from("NewTech was first set up to solve the common problem coming up for identifiers in computer science."))));

    let mdx_line = "NewTech was first set up to *solve* the common problem coming up for identifiers in *computer* science.";
    assert_eq!(parse_inline_wrap_text(mdx_line), Ok(("", String::from("NewTech was first set up to <em>solve</em> the common problem coming up for identifiers in <em>computer</em> science."))));
}

#[test]
pub fn test_parse_opening_html_tag() {
    let mdx_line = "<a href=\"https://www.example.com\">site</a>.";
    assert_eq!(
        parse_opening_html_tag(mdx_line, "a"),
        Ok(("site</a>.", ("href=\"https://www.example.com\"")))
    );
}

#[test]
pub fn test_parse_ordered_list_text() {
    let list_line_mdx = "1. first of all  ";
    assert_eq!(
        parse_ordered_list_text(list_line_mdx),
        Ok(("first of all  ", 0))
    );

    let list_line_mdx = "  1. first of all  ";
    assert_eq!(
        parse_ordered_list_text(list_line_mdx),
        Ok(("first of all  ", 2))
    );
}

#[test]
pub fn test_parse_unordered_list_text() {
    let list_line_mdx = "- first of all  ";
    assert_eq!(
        parse_unordered_list_text(list_line_mdx),
        Ok(("first of all  ", 0))
    );

    let list_line_mdx = "  - first of all  ";
    assert_eq!(
        parse_unordered_list_text(list_line_mdx),
        Ok(("first of all  ", 2))
    );
}

#[test]
pub fn test_parse_up_to_opening_html_tag() {
    let mdx_line = "Visit our new <a href=\"https://www.example.com\">site</a>.";
    assert_eq!(
        parse_up_to_opening_html_tag(mdx_line, "a"),
        Ok((
            "<a href=\"https://www.example.com\">site</a>.",
            ("Visit our new ")
        ))
    );
}

#[test]
pub fn test_segment_strong_emphasis_line() {
    let mdx_line = "NewTech was **first** set up to solve the **common problem** coming up for identifiers in computer science.";
    assert_eq!(segment_strong_emphasis_line(mdx_line), Ok(("", ("NewTech was ", "first", " set up to solve the **common problem** coming up for identifiers in computer science."))));
}

#[test]
pub fn test_segment_emphasis_line() {
    let mdx_line = "NewTech was first set up to *solve* the common problem coming up for identifiers in *computer* science.";
    assert_eq!(
        segment_emphasis_line(mdx_line),
        Ok((
            "",
            (
                "NewTech was first set up to ",
                "solve",
                " the common problem coming up for identifiers in *computer* science."
            )
        ))
    );
}
