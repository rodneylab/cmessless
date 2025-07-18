#[cfg(test)]
mod tests;

pub mod jsx;
use crate::{
    parser::jsx::{
        form_code_fragment_component_first_line, form_gatsby_not_maintained_component,
        form_image_component, form_poll_component_first_line, form_questions_component,
        form_tweet_component, form_video_component_first_line, parse_open_jsx_block,
        JSXComponentRegister, JSXComponentType,
    },
    utility::stack::Stack,
};
use deunicode::deunicode;
use markup_fmt::{config::FormatOptions, format_text, Language};
use nom::{
    branch::alt,
    bytes::complete::{is_not, tag, tag_no_case, take_until},
    character::complete::{alpha1, alphanumeric1, digit1, multispace0, multispace1},
    combinator::{opt, peek, recognize, rest, value},
    error::{Error, ErrorKind},
    multi::{many0, many0_count, many1, many1_count},
    sequence::{delimited, pair, preceded, separated_pair, terminated},
    Err, IResult, Parser,
};
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    fs::File,
    io::{BufRead, BufReader, Cursor, Read, Seek, Write},
    path::Path,
    time::Instant,
};

type ParsedFencedCodeBlockMeta<'a> = (
    Option<&'a str>, // language
    Option<&'a str>, // first line number
    Option<&'a str>, // highlight line numbers
    Option<&'a str>, // title
    Option<&'a str>, // caption
    Option<bool>,    //collapse
);

#[derive(PartialEq)]
enum HTMLBlockElementType {
    Comment,
    DescriptionList,
    Div,
    Figure,
    TableBody,
    TableHead,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum HTMLTagType {
    Opening,
    OpeningStart,
    SelfClosing,
    Closing,
}

#[derive(Debug, Eq, PartialEq)]
pub enum LineType {
    CodeFragment,
    CodeFragmentOpen,
    CodeFragmentOpening,
    FencedCodeBlock,
    FencedCodeBlockOpen,
    Frontmatter,
    FrontmatterDelimiter,
    GatsbyNotMaintained,
    JSXComponent,
    Heading,
    HTMLBlockLevelComment,
    HTMLBlockLevelCommentOpen,
    HTMLDescriptionList,
    HTMLDescriptionListOpen,
    HTMLDivBlockOpen,
    HTMLDivBlock,
    HTMLFigureBlockOpen,
    HTMLFigureBlock,
    HTMLTableBodyOpen,
    HTMLTableBody,
    HTMLTableHeadOpen,
    HowTo,
    HowToOpen,
    HowToOpening,
    HowToSection,
    HowToSectionOpen,
    HowToSectionOpening,
    HowToStep,
    HowToStepOpen,
    HowToStepOpening,
    HowToDirection,
    HowToDirectionOpen,
    HowToDirectionOpening,
    Image,
    OrderedList,
    OrderedListItemOpen,
    Paragraph,
    Poll,
    PollOpen,
    PollOpening,
    Questions,
    Tweet,
    UnorderedListItem,
    Video,
    VideoOpen,
    VideoOpening,
}

#[derive(Debug, PartialEq)]
enum ListType {
    Ordered,
    Unordered,
}

#[derive(Debug, PartialEq)]
enum MarkdownBlock {
    OrderedList,
}

#[derive(Clone, Debug, PartialEq)]
enum TableAlign {
    Centre,
    Left,
    Right,
}

#[allow(dead_code)]
fn discard_leading_whitespace(line: &str) -> IResult<&str, &str> {
    preceded(multispace0, rest).parse(line)
}

fn escape_code(line: &str) -> String {
    line.replace('<', "\\u003C")
        .replace('>', "\\u003E")
        .replace('`', "\\u0060")
        .replace('{', "\\u007B")
        .replace('}', "\\u007D")
        .replace("import.", "import..")
        .replace("process.env", "process..env")
}

fn remove_html_tags(line: &str) -> IResult<&str, &str> {
    let (remaining_line, initial_segment) = take_until("<")(line)?;
    let (final_segment, _) = parse_self_closing_html_tag(remaining_line)?;
    Ok((final_segment, initial_segment))
}

fn form_code_span_html_string(input: &str) -> String {
    match segment_code_span_line(input) {
        Ok((_, (initial_segment, code_segment, final_segment))) => {
            format!(
                "{initial_segment}<code>{code_segment}</code>{}",
                form_code_span_html_string(final_segment)
            )
        }
        Err(_) => String::from(input),
    }
}

/* if the last word of the title is shorter than 6 characters, replaces the last space with a
 * non-breaking space
 */
fn format_heading_widows(heading: &str) -> String {
    match heading.rsplit_once(' ') {
        Some((before_space, after_space)) => {
            if after_space.len() < 5 {
                format!(
                    "{}\\u00a0{}",
                    format_heading(before_space),
                    format_heading(after_space)
                )
            } else {
                format_heading(heading).to_string()
            }
        }
        None => format_heading(heading).to_string(),
    }
}

fn format_heading<'a, I: Into<Cow<'a, str>>>(heading: I) -> Cow<'a, str> {
    fn is_replace_character(c: char) -> bool {
        c == '-' || c == '\'' || c == '"'
    }

    let heading = heading.into();
    let first = heading.find(is_replace_character);
    if let Some(first) = first {
        let (mut result, rest) = match first {
            0 => match &heading[0..1] {
                "\"" => (String::from("\\u201c"), heading[1..].chars()),
                "'" => (String::from("\\u2018"), heading[1..].chars()),
                _ => (String::from(&heading[0..first]), heading[first..].chars()),
            },
            _ => {
                if &heading[(first - 1)..first] == " " {
                    (
                        String::from(&heading[0..(first - 1)]),
                        heading[(first - 1)..].chars(),
                    )
                } else {
                    (String::from(&heading[0..first]), heading[first..].chars())
                }
            }
        };
        result.reserve(heading.len() - first);

        let mut preceded_by_space = false;
        for c in rest {
            match c {
                '-' => result.push_str("&#x2011;"), // non-breaking hyphen
                ' ' => {
                    preceded_by_space = true;
                    result.push(c);
                }
                '\'' => {
                    if preceded_by_space {
                        preceded_by_space = false;
                        result.push_str("\\u2018");
                    } else {
                        result.push_str("\\u2019");
                    }
                }
                '"' => {
                    if preceded_by_space {
                        preceded_by_space = false;
                        result.push_str("\\u201c");
                    } else {
                        result.push_str("\\u201d");
                    }
                }
                _ => {
                    preceded_by_space = false;
                    result.push(c);
                }
            }
        }
        Cow::Owned(result)
    } else {
        heading
    }
}

fn slugify_title(title: &str) -> String {
    if let Ok((final_value, initial_value)) = remove_html_tags(title) {
        format!(
            "{}{}",
            slugify_title(initial_value),
            slugify_title(final_value)
        )
    } else {
        let deunicoded_title = deunicode(title);
        let mut result = String::with_capacity(deunicoded_title.len());
        let mut last_was_replaced = true;
        let remove_characters = "?'`:[]()";
        let replace_characters = " -/.,$"; // include '-' here to avoid "--" in result
        for chars in deunicoded_title.chars() {
            if replace_characters.contains(chars) {
                if !last_was_replaced {
                    last_was_replaced = true;
                    result.push('-');
                }
            } else if !remove_characters.contains(chars) {
                last_was_replaced = false;
                result.push_str(&chars.to_lowercase().to_string());
            }
        }
        result
    }
}

fn parse_author_name_from_cargo_pkg_authors(cargo_pkg_authors: &str) -> IResult<&str, &str> {
    take_until(" <")(cargo_pkg_authors)
}

pub fn author_name_from_cargo_pkg_authors() -> &'static str {
    let (_, result) = parse_author_name_from_cargo_pkg_authors(env!("CARGO_PKG_AUTHORS"))
        .expect("[ ERROR ] Authors should be defined!");
    result
}

// consumes delimiter
fn parse_up_to_inline_wrap_segment<'a>(
    line: &'a str,
    delimiter: &'a str,
) -> IResult<&'a str, (&'a str, &'a str)> {
    separated_pair(take_until(delimiter), tag(delimiter), rest).parse(line)
}

fn parse_html_tag_attributes_str(line: &str) -> IResult<&str, &str> {
    is_not(">/")(line)
}

fn parse_html_tag_content(line: &str) -> IResult<&str, (&str, &str)> {
    let (remainder, tag_content) = is_not(">/")(line)?;
    let (attributes, (tag_name, _space)) = pair(alphanumeric1, multispace0).parse(tag_content)?;
    Ok((remainder, (tag_name, attributes)))
}

fn parse_closing_html_tag(line: &str) -> IResult<&str, (&str, &str, HTMLTagType)> {
    let (remaining_line, (tag_name, tag_attributes)) =
        delimited(tag("</"), parse_html_tag_content, tag(">")).parse(line)?;
    Ok((
        remaining_line,
        (tag_name, tag_attributes, HTMLTagType::Closing),
    ))
}

fn parse_opening_html_tag(line: &str) -> IResult<&str, (&str, &str, HTMLTagType)> {
    let (remaining_line, (tag_name, tag_attributes)) =
        delimited(tag("<"), parse_html_tag_content, tag(">")).parse(line)?;
    Ok((
        remaining_line,
        (tag_name, tag_attributes, HTMLTagType::Opening),
    ))
}

fn parse_opening_html_tag_start(line: &str) -> IResult<&str, (&str, &str, HTMLTagType)> {
    let (remaining_line, (tag_name, tag_attributes)) =
        preceded(tag("<"), parse_html_tag_content).parse(line)?;
    Ok((
        remaining_line,
        (tag_name, tag_attributes, HTMLTagType::OpeningStart),
    ))
}

fn parse_opening_html_tag_end(line: &str) -> IResult<&str, (&str, HTMLTagType)> {
    if let Ok((remaining_line, tag_attributes)) = alt((
        delimited(multispace0, parse_html_tag_attributes_str, tag(">")),
        terminated(multispace0, tag(">")),
    ))
    .parse(line)
    {
        Ok((remaining_line, (tag_attributes, HTMLTagType::Opening)))
    } else {
        let (_, attributes) = preceded(multispace0, rest).parse(line)?;
        Ok(("", (attributes, HTMLTagType::OpeningStart)))
    }
}

fn parse_self_closing_html_tag(line: &str) -> IResult<&str, (&str, &str, HTMLTagType)> {
    let (remaining_line, (tag_name, tag_attributes)) =
        delimited(tag("<"), parse_html_tag_content, tag("/>")).parse(line)?;
    Ok((
        remaining_line,
        (tag_name, tag_attributes, HTMLTagType::SelfClosing),
    ))
}

fn parse_self_closing_html_tag_end(line: &str) -> IResult<&str, (&str, HTMLTagType)> {
    let (remaining_line, tag_attributes) = alt((
        delimited(multispace0, parse_html_tag_attributes_str, tag("/>")),
        terminated(multispace0, tag("/>")),
    ))
    .parse(line)?;
    Ok((remaining_line, (tag_attributes, HTMLTagType::SelfClosing)))
}

fn parse_up_to_opening_html_tag<'a>(
    line: &'a str,
    element_tag: &'a str,
) -> IResult<&'a str, &'a str> {
    let delimiter = &mut String::from("<");
    delimiter.push_str(element_tag);
    let result = take_until(delimiter.as_str())(line);
    result
}

fn parse_opening_html_tag_no_attributes<'a>(
    line: &'a str,
    element_tag: &'a str,
) -> IResult<&'a str, &'a str> {
    let closed_delimiter = &mut String::from("<");
    closed_delimiter.push_str(element_tag);
    closed_delimiter.push('>');
    let (tag_close, _attributes) = tag(closed_delimiter.as_str())(line)?;
    Ok((tag_close, ""))
}

fn parse_opening_html_tag_with_attributes<'a>(
    line: &'a str,
    element_tag: &'a str,
) -> IResult<&'a str, &'a str> {
    let delimiter = &mut String::from("<");
    delimiter.push_str(element_tag);
    let (tag_close, attributes) = delimited(
        tag(delimiter.as_str()),
        delimited(multispace1, take_until(">"), multispace0),
        tag(">"),
    )
    .parse(line)?;
    Ok((tag_close, attributes))
}

fn segment_anchor_element_with_attributes_line(line: &str) -> IResult<&str, (&str, &str, &str)> {
    let delimiter = "a";
    let (remainder, initial_segment) = parse_up_to_opening_html_tag(line, delimiter)?;
    let (final_segment, anchor_attributes_segment) =
        parse_opening_html_tag_with_attributes(remainder, delimiter)?;
    Ok((
        "",
        (initial_segment, anchor_attributes_segment, final_segment),
    ))
}

fn segment_anchor_element_no_attributes_line(line: &str) -> IResult<&str, (&str, &str, &str)> {
    let delimiter = "a";
    let (remainder, initial_segment) = parse_up_to_opening_html_tag(line, delimiter)?;
    let (final_segment, anchor_attributes_segment) =
        parse_opening_html_tag_no_attributes(remainder, delimiter)?;
    Ok((
        "",
        (initial_segment, anchor_attributes_segment, final_segment),
    ))
}

fn segment_code_span_line(line: &str) -> IResult<&str, (&str, &str, &str)> {
    let delimiter = "`";
    let (_, (initial_segment, remainder)) = parse_up_to_inline_wrap_segment(line, delimiter)?;
    let (_, (bold_segment, final_segment)) = parse_inline_wrap_segment(remainder, delimiter)?;
    Ok(("", (initial_segment, bold_segment, final_segment)))
}

fn segment_emphasis_line(line: &str) -> IResult<&str, (&str, &str, &str)> {
    let delimiter = "*";
    let (_, (initial_segment, remainder)) = parse_up_to_inline_wrap_segment(line, delimiter)?;
    let (_, (bold_segment, final_segment)) = parse_inline_wrap_segment(remainder, delimiter)?;
    Ok(("", (initial_segment, bold_segment, final_segment)))
}

fn segment_strong_emphasis_line(line: &str) -> IResult<&str, (&str, &str, &str)> {
    let delimiter = "**";
    let (_, (initial_segment, remainder)) = parse_up_to_inline_wrap_segment(line, delimiter)?;
    let (_, (bold_segment, final_segment)) = parse_inline_wrap_segment(remainder, delimiter)?;
    Ok(("", (initial_segment, bold_segment, final_segment)))
}

fn parse_html_tag_attribute(line: &str) -> IResult<&str, (&str, &str)> {
    alt((
        (
            preceded(multispace0, take_until("=")),
            delimited(tag("=\""), take_until("\""), tag("\"")),
        ),
        (
            preceded(multispace0, take_until("=")),
            delimited(tag("={`"), take_until("`}"), tag("`}")),
        ),
    ))
    .parse(line)
}

fn parse_astro_client_directive(line: &str) -> IResult<&str, (&str, &str)> {
    preceded(
        multispace0,
        separated_pair(alphanumeric1, tag(":"), alphanumeric1),
    )
    .parse(line)
}

fn parse_html_tag_attributes(attributes: &str) -> IResult<&str, Vec<(&str, &str)>> {
    many0(alt((
        parse_astro_client_directive,
        parse_html_tag_attribute,
    )))
    .parse(attributes)
}

fn parse_href_scheme(href: &str) -> IResult<&str, &str> {
    alt((tag_no_case("HTTP://"), tag_no_case("HTTPS://"))).parse(href)
}

fn form_html_anchor_element_line(line: &str) -> IResult<&str, String> {
    let (_, (initial_segment, anchor_attributes_segment, final_segment)) = alt((
        segment_anchor_element_with_attributes_line,
        segment_anchor_element_no_attributes_line,
    ))
    .parse(line)?;
    let (_, attributes_vector) = parse_html_tag_attributes(anchor_attributes_segment)?;
    let (remaining_line, link_content) = take_until("</a>")(final_segment)?;

    let attributes_hash_map: HashMap<&str, &str> = attributes_vector.into_iter().collect();
    let href = attributes_hash_map
        .get("href")
        .unwrap_or_else(|| panic!("[ ERROR ] Anchor tag missing href: {line}"));
    let external_site = parse_href_scheme(href).is_ok();
    let mut additional_attributes = String::new();

    if external_site {
        if !attributes_hash_map.contains_key("target") {
            additional_attributes.push_str(" target=\"_blank\"");
        }
        if !attributes_hash_map.contains_key("rel") {
            additional_attributes.push_str(" rel=\"nofollow noopener noreferrer\"");
        }
    }
    let icon = if external_site {
        "&nbsp;<LinkIcon />"
    } else {
        ""
    };
    let (remaining_line, (tag_name, _, _)) = parse_closing_html_tag(remaining_line)?;
    match tag_name {
        "a" => {
            let (_, link_content) = parse_inline_wrap_text(link_content)?;
            Ok((
        remaining_line,
        format!("{initial_segment}<a {anchor_attributes_segment}{additional_attributes}>{link_content}{icon}</a>"),
    ))
        }
        _ => Err(Err::Error(Error::new(line, ErrorKind::Tag))),
    }
}

fn form_code_span_line(line: &str) -> IResult<&str, String> {
    let (_, (initial_segment, code_segment, final_segment)) = segment_code_span_line(line)?;
    Ok((
        final_segment,
        format!(
            "{initial_segment}<InlineCodeFragment code={{`{}`}} />",
            escape_code(code_segment)
        ),
    ))
}

fn parse_fenced_code_block_first_line(line: &str) -> IResult<&str, ParsedFencedCodeBlockMeta<'_>> {
    let (meta, _) = tag("```")(line)?;
    let (remaining_meta, language_option) =
        opt(alt((terminated(take_until(" "), tag(" ")), alpha1))).parse(meta)?;
    let (remaining_meta, first_line_number_option) =
        opt(alt((terminated(digit1, tag(" ")), digit1))).parse(remaining_meta)?;
    let (remaining_meta, highlight_lines_option) = opt(alt((
        delimited(peek(tag("{")), is_not(" \t\r\n"), tag(" ")),
        preceded(peek(tag("{")), is_not(" \t\r\n")),
    )))
    .parse(remaining_meta)?;
    let (remaining_meta, title_option) = opt(alt((
        delimited(tag("\""), take_until("\" "), tag("\" ")),
        delimited(tag("\""), take_until("\""), tag("\"")),
    )))
    .parse(remaining_meta)?;
    let (remaining_meta, caption_option) = opt(alt((
        delimited(tag("["), take_until("] "), tag("] ")),
        delimited(tag("["), take_until("]"), tag("]")),
    )))
    .parse(remaining_meta)?;
    let (_, collapse_option_tag) = opt(tag("<>")).parse(remaining_meta)?;
    let collapse_option = match collapse_option_tag {
        Some("<>") => Some(true),
        _ => Some(false),
    };
    Ok((
        "",
        (
            language_option,
            first_line_number_option,
            highlight_lines_option,
            title_option,
            caption_option,
            collapse_option,
        ),
    ))
}

fn parse_fenced_code_block_last_line(line: &str) -> IResult<&str, &str> {
    tag("```")(line)
}

fn parse_html_block_level_comment_first_line(line: &str) -> IResult<&str, &str> {
    tag("<!--")(line)
}

fn parse_html_block_level_comment_last_line(line: &str) -> IResult<&str, &str> {
    terminated(take_until("-->"), tag("-->")).parse(line)
}

fn parse_table_column_alignment(line: &str) -> IResult<&str, TableAlign> {
    let (remaining_line, cell) =
        terminated(take_until("|"), pair(tag("|"), multispace0)).parse(line)?;
    let (_, alignment) = alt((
        value(
            TableAlign::Centre,
            delimited(tag(":"), tag("---"), tag(":")),
        ),
        value(TableAlign::Left, preceded(tag(":"), tag("---"))),
        value(TableAlign::Right, terminated(tag("---"), tag(":"))),
    ))
    .parse(cell)?;
    Ok((remaining_line, alignment))
}

fn parse_table_cell(line: &str) -> IResult<&str, &str> {
    terminated(take_until("|"), pair(tag("|"), multispace0)).parse(line)
}

// parses row separating header and body containing alignment markers
fn parse_table_header_row(line: &str) -> IResult<&str, Vec<TableAlign>> {
    let (headings, _) = preceded(tag("|"), multispace1).parse(line)?;
    many1(parse_table_column_alignment).parse(headings)
}

fn parse_table_line(line: &str) -> IResult<&str, Vec<&str>> {
    let (headings, _) = preceded(tag("|"), multispace1).parse(line)?;
    many1(parse_table_cell).parse(headings)
}

fn form_html_block_element_first_line(line: &str) -> IResult<&str, (String, LineType, usize)> {
    let (_remaining_line, (tag_name, _tag_attributes, _tag_type)) = parse_opening_html_tag(line)?;
    match tag_name {
        "dl" => Ok((
            "",
            (String::from(line), LineType::HTMLDescriptionListOpen, 0),
        )),
        "div" => Ok(("", (String::from(line), LineType::HTMLDivBlockOpen, 0))),
        "figure" => Ok(("", (String::from(line), LineType::HTMLFigureBlockOpen, 0))),
        _ => panic!("[ ERROR ] Unrecognised HTML block element: {tag_name}"),
    }
}

fn form_html_block_element_last_line(line: &str) -> IResult<&str, (String, LineType, usize)> {
    let (_remaining_line, (tag_name, _tag_attributes, _tag_type)) = parse_closing_html_tag(line)?;
    match tag_name {
        "dl" => Ok(("", (String::from(line), LineType::HTMLDescriptionList, 0))),
        "div" => Ok(("", (String::from(line), LineType::HTMLDivBlock, 0))),
        "figure" => Ok(("", (String::from(line), LineType::HTMLFigureBlock, 0))),
        _ => Err(Err::Error(Error::new(line, ErrorKind::Tag))),
    }
}

fn form_fenced_code_block_first_line(line: &str) -> IResult<&str, (String, LineType, usize)> {
    let (
        _,
        (
            language_option,
            first_line_number_option,
            highlight_line_numbers_option,
            title_option,
            caption_option,
            collapse_option,
        ),
    ) = parse_fenced_code_block_first_line(line)?;

    let mut markup = String::from("<CodeFragment\n  client:visible");
    if let Some(value) = language_option {
        markup.push_str("\n  language=\"");
        markup.push_str(value);
        markup.push('\"');
    };
    if let Some(value) = first_line_number_option {
        markup.push_str("\n  firstLine={");
        markup.push_str(value);
        markup.push('}');
    };
    if let Some(value) = highlight_line_numbers_option {
        markup.push_str("\n  highlightLines={`");
        markup.push_str(value);
        markup.push_str("`}");
    };
    if let Some(value) = title_option {
        markup.push_str("\n  title=\"");
        markup.push_str(value);
        markup.push('\"');
    };
    if let Some(value) = caption_option {
        markup.push_str("\n  caption=\"");
        markup.push_str(value);
        markup.push('\"');
    };
    if let Some(true) = collapse_option {
        markup.push_str("\n  collapse");
    };
    markup.push_str("\n  code={`");
    Ok(("", (markup, LineType::FencedCodeBlockOpen, 0)))
}

fn form_table_body_row(line: &str) -> IResult<&str, (String, LineType, usize)> {
    let (_, cells) = parse_table_line(line)?;
    let mut markup = String::from("    <tr>");
    for cell in cells {
        markup.push_str("\n      <td>");
        markup.push_str(cell.trim_end());
        markup.push_str("</td>");
    }
    markup.push_str("\n    </tr>");
    Ok(("", (markup, LineType::HTMLTableBodyOpen, 0)))
}

// regular row in table head
fn form_table_head_row(line: &str) -> IResult<&str, (String, LineType, usize)> {
    let (_, cells) = parse_table_line(line)?;
    let mut markup = String::from("    <tr>");
    for cell in cells {
        markup.push_str("\n      <th scope=\"col\">");
        markup.push_str(cell);
        markup.push_str("</th>");
    }
    markup.push_str("\n    </tr>");
    Ok(("", (markup, LineType::HTMLTableHeadOpen, 0)))
}

// special row between head and body with alignment markers
fn form_table_header_row(line: &str) -> IResult<&str, (String, LineType, usize)> {
    parse_table_header_row(line)?;
    Ok((
        "",
        (
            String::from("  </thead>\n  <tbody>"),
            LineType::HTMLTableBodyOpen,
            0,
        ),
    ))
}

fn form_table_body_last_line(line: &str) -> IResult<&str, (String, LineType, usize)> {
    match form_table_body_row(line) {
        Ok(value) => Ok(value),
        Err(_) => Ok((
            "",
            (
                String::from("  </tbody>\n</table>"),
                LineType::HTMLTableBody,
                0,
            ),
        )),
    }
}

fn form_table_head_first_line(line: &str) -> IResult<&str, (String, LineType, usize)> {
    let (_, (row_body, line_type, indentation)) = form_table_head_row(line)?;
    let markup = String::from("<table>\n  <thead>");
    Ok((
        "",
        (format!("{markup}\n{row_body}"), line_type, indentation),
    ))
}

// optimistically try to end the head section or alternatively add additional head line
fn form_table_head_last_line(line: &str) -> IResult<&str, (String, LineType, usize)> {
    alt((form_table_header_row, form_table_head_row)).parse(line)
}

fn form_fenced_code_block_last_line(line: &str) -> IResult<&str, (String, LineType, usize)> {
    let (_final_segment, _initial_segment) = parse_fenced_code_block_last_line(line)?;
    Ok(("", (String::from("  `} />"), LineType::FencedCodeBlock, 0)))
}

fn form_emphasis_line(line: &str) -> IResult<&str, String> {
    let (_, (initial_segment, bold_segment, final_segment)) = segment_emphasis_line(line)?;
    Ok((
        final_segment,
        format!("{initial_segment}<em>{bold_segment}</em>"),
    ))
}

fn form_strong_emphasis_line(line: &str) -> IResult<&str, String> {
    let (_, (initial_segment, bold_segment, final_segment)) = segment_strong_emphasis_line(line)?;
    match form_code_span_line(bold_segment) {
        Ok((_, code_segment)) => Ok((
            final_segment,
            format!("{initial_segment}<strong>{code_segment}</strong>"),
        )),
        Err(_) => Ok((
            final_segment,
            format!("{initial_segment}<strong>{bold_segment}</strong>"),
        )),
    }
}

fn form_inline_wrap_text_number_range(line: &str) -> IResult<&str, String> {
    let (remaining_line, first_tag) = recognize(separated_pair(
        tag("<InlineCodeFragment code={`"),
        digit1,
        tag("`} />"),
    ))
    .parse(line)?;
    let (remaining_line, _) = alt((tag("&ndash;"), tag("-"))).parse(remaining_line)?;
    let (remaining_line, second_tag) = recognize(separated_pair(
        tag("<InlineCodeFragment code={`"),
        digit1,
        tag("`} />"),
    ))
    .parse(remaining_line)?;

    Ok((
        remaining_line,
        format!("{first_tag}&thinsp;&ndash;&thinsp;{second_tag}"),
    ))
}

fn form_inline_wrap_inline_code_fragment(line: &str) -> IResult<&str, String> {
    let (remaining_line, tag_content) = delimited(
        tag("<InlineCodeFragment code={`"),
        take_until("/>"),
        tag("/>"),
    )
    .parse(line)?;

    Ok((
        remaining_line,
        format!("<InlineCodeFragment code={{`{tag_content}/>"),
    ))
}

fn format_inline_wrap_text_number_range(line: &str) -> IResult<&str, String> {
    let (remaining_line, initial_part) = take_until("<InlineCodeFragment")(line)?;
    let (remaining_line, inline_code_fragment_part) = alt((
        form_inline_wrap_text_number_range,
        form_inline_wrap_inline_code_fragment,
    ))
    .parse(remaining_line)?;

    match format_inline_wrap_text_number_range(remaining_line) {
        Ok((_, value)) => Ok((
            "",
            format!("{initial_part}{inline_code_fragment_part}{value}"),
        )),
        Err(_) => Ok((
            "",
            format!("{initial_part}{inline_code_fragment_part}{remaining_line}"),
        )),
    }
}

fn parse_inline_wrap_text(line: &str) -> IResult<&str, String> {
    fn is_wrap_tag(c: char) -> bool {
        c == '`' || c == '*' || c == '<'
    }

    let first_tag = line.find(is_wrap_tag);
    if let Some(first_tag) = first_tag {
        let line_from_tag = &line[first_tag..];
        let parsed_result = match &line_from_tag[0..1] {
            "`" => form_code_span_line(line_from_tag),
            "<" => form_html_anchor_element_line(line_from_tag),
            "*" => alt((form_strong_emphasis_line, form_emphasis_line)).parse(line_from_tag),
            _ => return Ok(("", line.to_string())),
        };
        let Ok((final_segment, initial_segment)) = parsed_result else {
            return Ok(("", line.to_string()));
        };
        let (_, final_final_segment) = parse_inline_wrap_text(final_segment)?;
        let line_before_tag = &line[..first_tag];
        Ok((
            "",
            format!("{line_before_tag}{initial_segment}{final_final_segment}"),
        ))
    } else {
        Ok(("", line.to_string()))
    }
}

fn parse_heading_text(line: &str) -> IResult<&str, usize> {
    let (heading, level) = terminated(many1_count(tag("#")), multispace1).parse(line)?;
    Ok((heading, level))
}

// consumes delimiter
fn parse_inline_wrap_segment<'a>(
    line: &'a str,
    delimiter: &'a str,
) -> IResult<&'a str, (&'a str, &'a str)> {
    separated_pair(take_until(delimiter), tag(delimiter), rest).parse(line)
}

fn parse_ordered_list_text(line: &str) -> IResult<&str, (usize, &str)> {
    let (content_text, (indentation, start, _full_stop_tag)) =
        (many0_count(tag(" ")), digit1, tag(". ")).parse(line)?;
    Ok((content_text.trim(), (indentation, start)))
}

fn parse_unordered_list_text(line: &str) -> IResult<&str, usize> {
    let (heading, indentation) = terminated(many0_count(tag(" ")), tag("- ")).parse(line)?;
    Ok((heading, indentation))
}

fn form_heading_line(line: &str) -> IResult<&str, (String, LineType, usize)> {
    let (value, level) = parse_heading_text(line)?;
    let parsed_text = form_code_span_html_string(value);
    let id = slugify_title(value);
    Ok((
        "",
        (
            format!(
                "<h{level} id=\"{id}\"><Heading client:visible id=\"{id}\" text=\"{}\"/></h{level}>",
                format_heading_widows(parsed_text.trim_end())
            ),
            LineType::Heading,
            level,
        ),
    ))
}

fn form_html_block_level_comment_first_line(
    line: &str,
) -> IResult<&str, (String, LineType, usize)> {
    parse_html_block_level_comment_first_line(line)?;
    Ok((
        "",
        (
            line.trim_end().to_string(),
            LineType::HTMLBlockLevelCommentOpen,
            0,
        ),
    ))
}

fn form_html_block_level_comment_last_line(line: &str) -> IResult<&str, (String, LineType, usize)> {
    match parse_html_block_level_comment_last_line(line) {
        Ok((after_comment, end_of_comment)) => {
            let (_, after_comment) = parse_inline_wrap_text(after_comment)?;
            let markup = format!("{end_of_comment}-->{}", after_comment.trim_end());
            Ok(("", (markup, LineType::HTMLBlockLevelComment, 0)))
        }
        Err(_) => Ok((
            "",
            (
                line.trim_end().to_string(),
                LineType::HTMLBlockLevelCommentOpen,
                0,
            ),
        )),
    }
}

fn form_ordered_list_first_line(line: &str) -> IResult<&str, (String, LineType, usize)> {
    let (list_text, (indentation, start)) = parse_ordered_list_text(line)?;
    let (_, parsed_list_text) = parse_inline_wrap_text(list_text)?;
    let markup = match start {
        "1" => format!("<ol>\n  <li>{parsed_list_text}"),
        _ => format!("<ol start=\"{start}\">\n  <li>{parsed_list_text}"),
    };
    Ok(("", (markup, LineType::OrderedListItemOpen, indentation)))
}

fn form_ordered_list_line(line: &str) -> IResult<&str, (String, LineType, usize)> {
    let (list_text, (indentation, _start)) = parse_ordered_list_text(line)?;
    let (_, parsed_list_text) = parse_inline_wrap_text(list_text)?;
    Ok((
        "",
        (
            format!("  <li>{parsed_list_text}"),
            LineType::OrderedListItemOpen,
            indentation,
        ),
    ))
}

fn form_unordered_list_line(line: &str) -> IResult<&str, (String, LineType, usize)> {
    let (list_text, indentation) = parse_unordered_list_text(line)?;
    let (_, parsed_list_text) = parse_inline_wrap_text(list_text)?;
    Ok((
        "",
        (
            format!("<li>\n  {parsed_list_text}\n</li>"),
            LineType::UnorderedListItem,
            indentation,
        ),
    ))
}

fn form_inline_wrap_text(line: &str) -> IResult<&str, (String, LineType, usize)> {
    let (_, parsed_line) = parse_inline_wrap_text(line)?;
    let parsed_line = if let Ok((_, value)) = format_inline_wrap_text_number_range(&parsed_line) {
        value
    } else {
        parsed_line
    };
    if parsed_line.is_empty() {
        Ok(("", (String::new(), LineType::Paragraph, 0)))
    } else {
        Ok((
            "",
            (format!("<p>{parsed_line}</p>"), LineType::Paragraph, 0),
        ))
    }
}

fn form_astro_frontmatter(
    components: &HashSet<JSXComponentType>,
    prepared_markup: &[String],
    slug: &str,
) -> Vec<String> {
    let mut result: Vec<String> = Vec::new();
    let mut define_slug = false;
    let mut image_data_imports: Vec<String> = Vec::new();

    result.push(String::from("---"));
    if components.contains(&JSXComponentType::CodeFragment) {
        result.push(String::from(
            "import CodeFragment from '~components/CodeFragment.svelte';",
        ));
    }
    result.push(String::from(
        "import Heading from '~components/Heading.svelte';",
    ));
    if components.contains(&JSXComponentType::HowTo) {
        define_slug = true;
        result.push(String::from(
            "import HowTo from '~components/HowTo/index.svelte';
import HowToSection from '~components/HowTo/HowToSection.svelte';
import HowToStep from '~components/HowTo/HowToStep.svelte';
import HowToDirection from '~components/HowTo/HowToDirection.svelte';",
        ));
    }
    if components.contains(&JSXComponentType::GatsbyNotMaintained) {
        result.push(String::from(
            "import GatsbyNotMaintained from '~components/BlogPost/GatsbyNotMaintained.svelte';",
        ));
    }
    if components.contains(&JSXComponentType::Image)
        || components.contains(&JSXComponentType::Video)
        || components.contains(&JSXComponentType::Tweet)
    {
        result.push(String::from("import { getEntry } from 'astro:content';"));
    }
    if components.contains(&JSXComponentType::Image) {
        define_slug = true;
        image_data_imports.push(String::from("images"));
        result.push(String::from(
            "import Image from '~components/BlogPost/Image.svelte';",
        ));
    }
    result.push(String::from(
        "import LinkIcon from '~components/Icons/Link.svelte';
import InlineCodeFragment from '~components/InlineCodeFragment.svelte';",
    ));
    if components.contains(&JSXComponentType::Poll) {
        define_slug = true;
        result.push(String::from("import Poll from '~components/Poll.svelte';"));
    }
    if components.contains(&JSXComponentType::Image) {
        result.push(String::from(
            "import type { NebulaPicture, PostPagePictures } from '~types/image';",
        ));
    } else if components.contains(&JSXComponentType::Video) {
        result.push(String::from(
            "import type { PostPagePictures } from '~types/image';",
        ));
    }
    if components.contains(&JSXComponentType::Questions) {
        result.push(String::from(
            "import Questions from '~components/Questions.svelte';",
        ));
        result.push(format!(
            "import questions from '~content-raw/blog/{slug}/questions.json';"
        ));
    }
    if components.contains(&JSXComponentType::Tweet) {
        result.push(String::from(
            "import Tweet from '~components/Tweet.svelte';",
        ));
    }
    result.push(String::from(
        "import TwitterMessageLink from '~components/Link/TwitterMessageLink.svelte';",
    ));
    if components.contains(&JSXComponentType::Video) {
        define_slug = true;
        image_data_imports.push(String::from("poster"));
        result.push(String::from(
            "import Video from '~components/Video.svelte';",
        ));
    }
    if define_slug {
        result.push("import website from '~configuration/website';".to_string());
        result.push("\nconst { newsletterUrl } = website;".to_string());
        result.push(format!("const slug = '{slug}';"));
        if components.contains(&JSXComponentType::Image)
            && components.contains(&JSXComponentType::Video)
        {
            result.push(
                "const postImagesContentCollectionEntry = await getEntry('post-images', slug);
const {
  data: { pagePictures, pictures },
}: { data: { pagePictures: PostPagePictures; pictures: NebulaPicture[] } } =
  postImagesContentCollectionEntry;
const {
  poster: { src: poster },
} = pagePictures;"
                    .to_string(),
            );
            result.push(
                "const imageProps = pictures.map((element, index) => ({
  index,
  ...element,
  slug,
}));"
                    .to_string(),
            );
        } else if components.contains(&JSXComponentType::Image) {
            result.push(
                "const postImagesContentCollectionEntry = await getEntry('post-images', slug);
const {
  data: { pictures },
}: { data: { pictures: NebulaPicture[] } } = postImagesContentCollectionEntry;"
                    .to_string(),
            );
            result.push(
                "const imageProps = pictures.map((element, index) => ({
  index,
  ...element,
  slug,
}));"
                    .to_string(),
            );
        } else if components.contains(&JSXComponentType::Video) {
            result.push(
                "const postImagesContentCollectionEntry = await getEntry('post-images', slug);
const {
  data: { pagePictures },
}: { data: { pagePictures: PostPagePictures; } } =
  postImagesContentCollectionEntry;
const {
  poster: { src: poster },
} = pagePictures;"
                    .to_string(),
            );
        }
    } else {
        result.push("import website from '~configuration/website';".to_string());
        result.push("\nconst { newsletterUrl } = website;".to_string());
    }
    if components.contains(&JSXComponentType::Tweet) {
        result.push(
            "const pageImagesContentCollectionEntry = await getEntry('page-images', 'blog');
const {
  data: { pagePictures: blogPagePictures },
}: { data: { pagePictures: PostPagePictures } } = pageImagesContentCollectionEntry;
const {
  twitterAvatar: { src: avatarSrc, placeholder: avatarPlaceholder },
} = blogPagePictures;"
                .to_string(),
        );
    }
    for line in prepared_markup {
        result.push(line.to_string());
    }
    result.push(String::from("---\n"));
    result
}

fn parse_frontmatter_delimiter(line: &str) -> IResult<&str, &str> {
    let (line, _) = tag("---")(line)?;
    Ok((line, ""))
}

fn parse_frontmatter_line(line: &str) -> (Option<String>, LineType) {
    match parse_frontmatter_delimiter(line) {
        Ok((_frontmatter_line, _)) => (None, LineType::FrontmatterDelimiter),
        Err(_) => (Some(String::from(line)), LineType::Frontmatter),
    }
}

fn parse_open_markdown_block(
    line: &str,
    open_markdown_block: Option<&MarkdownBlock>,
) -> Option<(String, LineType, usize)> {
    match open_markdown_block {
        Some(MarkdownBlock::OrderedList) => match form_ordered_list_line(line) {
            Ok((_, (line, line_type, level))) => {
                if line.is_empty() {
                    Some((String::from("</ol>"), LineType::OrderedList, level))
                } else {
                    let markup = format!("</li>{line}");
                    Some((markup, line_type, level))
                }
            }
            Err(_) => None,
        },
        None => None,
    }
}

fn parse_open_html_block(
    line: &str,
    open_html_block_elements: Option<&HTMLBlockElementType>,
) -> Option<(String, LineType, usize)> {
    match open_html_block_elements {
        Some(HTMLBlockElementType::Div) => match form_html_block_element_last_line(line) {
            Ok((_, (line, line_type, level))) => {
                if line.is_empty() {
                    None
                } else {
                    Some((line, line_type, level))
                }
            }
            Err(_) => Some((line.to_string(), LineType::HTMLDivBlockOpen, 0)),
        },
        Some(HTMLBlockElementType::Figure) => match form_html_block_element_last_line(line) {
            Ok((_, (line, line_type, level))) => {
                if line.is_empty() {
                    None
                } else {
                    Some((line, line_type, level))
                }
            }
            Err(_) => Some((line.to_string(), LineType::HTMLFigureBlockOpen, 0)),
        },
        Some(HTMLBlockElementType::DescriptionList) => {
            match form_html_block_element_last_line(line) {
                Ok((_, (line, line_type, level))) => {
                    if line.is_empty() {
                        None
                    } else {
                        Some((line, line_type, level))
                    }
                }
                Err(_) => Some((line.to_string(), LineType::HTMLDescriptionListOpen, 0)),
            }
        }
        Some(HTMLBlockElementType::TableBody) => match form_table_body_last_line(line) {
            Ok((_, value)) => Some(value),
            Err(_) => None,
        },
        Some(HTMLBlockElementType::TableHead) => match form_table_head_last_line(line) {
            Ok((_, value)) => Some(value),
            Err(_) => None,
        },
        Some(HTMLBlockElementType::Comment) => {
            match form_html_block_level_comment_last_line(line) {
                Ok((_, value)) => Some(value),
                Err(_) => None,
            }
        }
        None => None,
    }
}

fn parse_mdx_lines<B>(
    line: &str,
    lines_iterator: std::io::Lines<B>,
    open_markdown_block: Option<&MarkdownBlock>,
    open_html_block_elements: Option<&HTMLBlockElementType>,
    open_jsx_component_register: &mut JSXComponentRegister,
) -> (std::io::Lines<B>, Option<(String, LineType, usize)>)
where
    B: BufRead,
{
    match parse_open_markdown_block(line, open_markdown_block) {
        Some(value) => (lines_iterator, Some(value)),
        None => match parse_open_html_block(line, open_html_block_elements) {
            Some((_parsed_line, LineType::HTMLDivBlockOpen, _indentation)) => {
                (lines_iterator, parse_mdx_line(line))
            }
            Some(value) => (lines_iterator, Some(value)),
            None => match parse_open_jsx_block(line, open_jsx_component_register) {
                Some(value) => (lines_iterator, Some(value)),
                None => (lines_iterator, parse_mdx_line(line)),
            },
        },
    }
}

fn parse_mdx_line(line: &str) -> Option<(String, LineType, usize)> {
    match alt((
        form_code_fragment_component_first_line,
        form_fenced_code_block_first_line,
        // form_how_to_component_first_line,
        form_html_block_level_comment_first_line,
        form_html_block_element_first_line,
        form_table_head_first_line,
        form_image_component,
        form_poll_component_first_line,
        form_questions_component,
        form_tweet_component,
        form_gatsby_not_maintained_component,
        form_video_component_first_line,
        form_heading_line,
        form_ordered_list_first_line,
        form_unordered_list_line,
        form_inline_wrap_text,
    ))
    .parse(line)
    {
        Ok((_, (line, line_type, level))) => {
            if line.is_empty() {
                None
            } else {
                Some((line, line_type, level))
            }
        }
        Err(_) => None,
    }
}

pub fn parse_frontmatter(file: &File) -> usize {
    let reader = BufReader::new(file);
    let mut frontmatter_open = false;
    let mut line_number = 1;

    for line in reader.lines() {
        let line_content = line.unwrap();
        let (_frontmatter_line_option, line_type) = parse_frontmatter_line(&line_content);
        if line_type == LineType::FrontmatterDelimiter {
            frontmatter_open = !frontmatter_open;
            if !frontmatter_open {
                return line_number;
            }
        } else if !frontmatter_open {
            /* first line of file (with content) is not frontmatter delimiter so assume there is no
             * frontmatter
             */
            return 0;
        };
        line_number += 1;
    }
    line_number
}

pub fn slug_from_input_file_path<P: AsRef<Path>>(path: &P) -> &str {
    match path
        .as_ref()
        .file_stem()
        .expect("[ ERROR ] Couldn't open that file!")
        .to_str()
    {
        Some(value) => match value {
            "index" => path
                .as_ref()
                .parent()
                .expect("[ ERROR ] Couldn't open that file!")
                .file_name()
                .expect("[ ERROR ] Couldn't open that file!")
                .to_str()
                .expect("[ ERROR ] Couldn't open that file!"),
            other => other,
        },
        None => panic!("[ ERROR ] Couldn't open that file!"),
    }
}

pub fn parse_mdx_file<P1: AsRef<Path>, P2: AsRef<Path>>(
    input_path: &P1,
    output_path: &P2,
    verbose: bool,
) {
    println!(
        "[ INFO ] Parsing {:?}...",
        input_path.as_ref().display().to_string()
    );
    let start = Instant::now();

    let file = File::open(input_path).expect("[ ERROR ] Couldn't open that file!");
    let frontmatter_end_line_number = parse_frontmatter(&file);
    let file = File::open(input_path).expect("[ ERROR ] Couldn't open that file!");

    let slug = slug_from_input_file_path(input_path);
    let mut tokens: Vec<String> = Vec::new();
    let reader = BufReader::new(&file);

    let mut current_indentation: usize = 0;
    let mut open_lists = Stack::new();

    // used to keep a track of open blocks
    let mut open_jsx_component_register = JSXComponentRegister::new();
    let mut open_html_block_element_stack: Stack<HTMLBlockElementType> = Stack::new();
    let mut open_markdown_block_stack: Stack<MarkdownBlock> = Stack::new();
    let mut astro_frontmatter_markup: Vec<String> = Vec::new();

    let mut present_jsx_component_types: HashSet<JSXComponentType> = HashSet::new();

    let mut lines_iterator = reader.lines();
    if frontmatter_end_line_number > 0 {
        lines_iterator.nth(frontmatter_end_line_number - 1); // discard frontmatter
    }
    while let Some(line) = lines_iterator.next() {
        let line_content = line.unwrap();

        let (lines_iterator_current, parsed_line) = parse_mdx_lines(
            &line_content,
            lines_iterator,
            open_markdown_block_stack.peek(),
            open_html_block_element_stack.peek(),
            &mut open_jsx_component_register,
        );
        lines_iterator = lines_iterator_current;
        match parsed_line {
            Some((line, line_type, indentation)) => match line_type {
                LineType::OrderedList => {
                    open_markdown_block_stack.pop();
                    open_lists.pop();
                    tokens.push(line);
                }
                LineType::OrderedListItemOpen => {
                    let open_markdown_block = open_markdown_block_stack.peek();
                    if open_markdown_block != Some(&MarkdownBlock::OrderedList) {
                        open_markdown_block_stack.push(MarkdownBlock::OrderedList);
                    }
                    if open_lists.is_empty() {
                        open_lists.push(ListType::Ordered);
                        tokens.push(line);
                    } else if indentation > current_indentation {
                        open_lists.push(ListType::Ordered);
                        let list_item_indentation = " ".repeat(2 * open_lists.len());
                        tokens.push(format!("<ol>\n  {list_item_indentation}{line}"));
                    } else if indentation == current_indentation {
                        let list_item_indentation = " ".repeat(2 * open_lists.len());
                        tokens.push(format!("{list_item_indentation}{line}"));
                    } else {
                        while open_lists.pop() != Some(ListType::Ordered) {
                            tokens.push(String::from("</ul>"));
                        }
                        let list_item_indentation = " ".repeat(2 * open_lists.len());
                        tokens.push(format!("</ol>\n{list_item_indentation}{line}"));
                        open_markdown_block_stack.pop();
                    }
                    current_indentation = indentation;
                }
                LineType::UnorderedListItem => {
                    if open_lists.is_empty() {
                        open_lists.push(ListType::Unordered);
                        tokens.push(format!("<ul>\n  {line}"));
                    } else if indentation > current_indentation {
                        open_lists.push(ListType::Unordered);
                        let list_item_indentation = " ".repeat(2 * open_lists.len());
                        tokens.push(format!("<ul>\n{list_item_indentation}{line}"));
                    } else if indentation == current_indentation {
                        let list_item_indentation = " ".repeat(2 * open_lists.len());
                        tokens.push(format!("{list_item_indentation}{line}"));
                    } else {
                        while open_lists.pop() != Some(ListType::Unordered) {
                            tokens.push(String::from("</ol>"));
                            open_markdown_block_stack.pop();
                        }
                        let list_item_indentation = " ".repeat(2 * open_lists.len());
                        tokens.push(format!("</ul>\n{list_item_indentation}{line}"));
                    }
                    current_indentation = indentation;
                }
                LineType::Poll => {
                    present_jsx_component_types.insert(JSXComponentType::Poll);
                    open_jsx_component_register.pop();
                    tokens.push(line);
                }
                LineType::Video => {
                    present_jsx_component_types.insert(JSXComponentType::Video);
                    open_jsx_component_register.pop();
                    tokens.push(line);
                }
                LineType::FencedCodeBlock | LineType::CodeFragment => {
                    present_jsx_component_types.insert(JSXComponentType::CodeFragment);
                    open_jsx_component_register.pop();
                    tokens.push(line);
                }
                LineType::HowTo => {
                    present_jsx_component_types.insert(JSXComponentType::HowTo);
                    open_jsx_component_register.pop();
                    tokens.push(line);

                    if let Some(value) = open_jsx_component_register.how_to() {
                        astro_frontmatter_markup.append(&mut value.astro_frontmatter_markup());
                    };
                }
                LineType::HowToSection => {
                    present_jsx_component_types.insert(JSXComponentType::HowToSection);
                    open_jsx_component_register.pop();
                    tokens.push(line);
                }
                LineType::HowToStep => {
                    present_jsx_component_types.insert(JSXComponentType::HowToStep);
                    open_jsx_component_register.pop();
                    tokens.push(line);
                }
                LineType::HowToDirection => {
                    present_jsx_component_types.insert(JSXComponentType::HowToDirection);
                    open_jsx_component_register.pop();
                    tokens.push(line);
                }
                LineType::Image => {
                    present_jsx_component_types.insert(JSXComponentType::Image);
                    tokens.push(line);
                }
                LineType::Questions => {
                    present_jsx_component_types.insert(JSXComponentType::Questions);
                    tokens.push(line);
                }
                LineType::GatsbyNotMaintained => {
                    present_jsx_component_types.insert(JSXComponentType::GatsbyNotMaintained);
                    tokens.push(line);
                }
                LineType::Tweet => {
                    present_jsx_component_types.insert(JSXComponentType::Tweet);
                    tokens.push(line);
                }
                LineType::HTMLBlockLevelComment
                | LineType::HTMLDescriptionList
                | LineType::HTMLDivBlock
                | LineType::HTMLFigureBlock
                | LineType::HTMLTableBody => {
                    open_html_block_element_stack.pop();
                    tokens.push(line);
                }
                LineType::FencedCodeBlockOpen => {
                    if open_jsx_component_register.peek()
                        != Some(&JSXComponentType::FencedCodeBlock)
                    {
                        open_jsx_component_register.push(JSXComponentType::FencedCodeBlock);
                    }
                    tokens.push(line);
                }
                LineType::CodeFragmentOpen => {
                    if open_jsx_component_register.peek() != Some(&JSXComponentType::CodeFragment) {
                        open_jsx_component_register.push(JSXComponentType::CodeFragment);
                    }
                    tokens.push(line);
                }
                LineType::CodeFragmentOpening => {
                    if open_jsx_component_register.peek()
                        != Some(&JSXComponentType::CodeFragmentOpening)
                    {
                        open_jsx_component_register.push(JSXComponentType::CodeFragmentOpening);
                    }
                    tokens.push(line);
                }
                LineType::HowToOpen => {
                    let current_open_jsx_component = open_jsx_component_register.peek();
                    if current_open_jsx_component == Some(&JSXComponentType::HowToOpening) {
                        open_jsx_component_register.pop();
                        open_jsx_component_register.push(JSXComponentType::HowTo);
                    } else if current_open_jsx_component != Some(&JSXComponentType::HowTo) {
                        open_jsx_component_register.push(JSXComponentType::HowTo);
                    }
                    tokens.push(line);
                }
                LineType::HowToOpening => {
                    if open_jsx_component_register.peek() != Some(&JSXComponentType::HowToOpening) {
                        open_jsx_component_register.push(JSXComponentType::HowToOpening);
                    }
                    tokens.push(line);
                }
                LineType::HowToSectionOpen => {
                    let current_open_jsx_component = open_jsx_component_register.peek();
                    if current_open_jsx_component == Some(&JSXComponentType::HowToSectionOpening) {
                        open_jsx_component_register.pop();
                        open_jsx_component_register.push(JSXComponentType::HowToSection);
                    } else if current_open_jsx_component != Some(&JSXComponentType::HowToSection) {
                        open_jsx_component_register.push(JSXComponentType::HowToSection);
                    }
                    tokens.push(line);
                }
                LineType::HowToSectionOpening => {
                    if open_jsx_component_register.peek()
                        != Some(&JSXComponentType::HowToSectionOpening)
                    {
                        open_jsx_component_register.push(JSXComponentType::HowToSectionOpening);
                    }
                    tokens.push(line);
                }
                LineType::HowToStepOpen => {
                    let current_open_jsx_component = open_jsx_component_register.peek();
                    if current_open_jsx_component == Some(&JSXComponentType::HowToStepOpening) {
                        open_jsx_component_register.pop();
                        open_jsx_component_register.push(JSXComponentType::HowToStep);
                    } else if current_open_jsx_component != Some(&JSXComponentType::HowToStep) {
                        open_jsx_component_register.push(JSXComponentType::HowToStep);
                    }
                    tokens.push(line);
                }
                LineType::HowToStepOpening => {
                    if open_jsx_component_register.peek()
                        != Some(&JSXComponentType::HowToStepOpening)
                    {
                        open_jsx_component_register.push(JSXComponentType::HowToStepOpening);
                    }
                    tokens.push(line);
                }
                LineType::HowToDirectionOpen => {
                    let current_open_jsx_component = open_jsx_component_register.peek();
                    if current_open_jsx_component == Some(&JSXComponentType::HowToDirectionOpening)
                    {
                        open_jsx_component_register.pop();
                        open_jsx_component_register.push(JSXComponentType::HowToDirection);
                    } else if current_open_jsx_component != Some(&JSXComponentType::HowToDirection)
                    {
                        open_jsx_component_register.push(JSXComponentType::HowToDirection);
                    }
                    tokens.push(line);
                }
                LineType::HowToDirectionOpening => {
                    if open_jsx_component_register.peek()
                        != Some(&JSXComponentType::HowToDirectionOpening)
                    {
                        open_jsx_component_register.push(JSXComponentType::HowToDirectionOpening);
                    }
                    tokens.push(line);
                }
                LineType::PollOpen => {
                    present_jsx_component_types.insert(JSXComponentType::Poll);
                    if open_jsx_component_register.peek() != Some(&JSXComponentType::Poll) {
                        open_jsx_component_register.push(JSXComponentType::Poll);
                    }
                    tokens.push(line);
                }
                LineType::PollOpening => {
                    if open_jsx_component_register.peek() != Some(&JSXComponentType::PollOpening) {
                        open_jsx_component_register.push(JSXComponentType::PollOpening);
                    }
                    tokens.push(line);
                }
                LineType::VideoOpen => {
                    let current_open_jsx_component = open_jsx_component_register.peek();
                    if current_open_jsx_component == Some(&JSXComponentType::VideoOpening) {
                        open_jsx_component_register.pop();
                        open_jsx_component_register.push(JSXComponentType::Video);
                    } else if current_open_jsx_component != Some(&JSXComponentType::Video) {
                        open_jsx_component_register.push(JSXComponentType::Video);
                    }
                    tokens.push(line);
                }
                LineType::VideoOpening => {
                    if open_jsx_component_register.peek() != Some(&JSXComponentType::VideoOpening) {
                        open_jsx_component_register.push(JSXComponentType::VideoOpening);
                    }
                    tokens.push(line);
                }
                LineType::HTMLBlockLevelCommentOpen => {
                    if open_html_block_element_stack.peek() != Some(&HTMLBlockElementType::Comment)
                    {
                        open_html_block_element_stack.push(HTMLBlockElementType::Comment);
                    }
                    tokens.push(line);
                }
                LineType::HTMLDescriptionListOpen => {
                    if open_html_block_element_stack.peek()
                        != Some(&HTMLBlockElementType::DescriptionList)
                    {
                        open_html_block_element_stack.push(HTMLBlockElementType::DescriptionList);
                    }
                    tokens.push(line);
                }
                LineType::HTMLDivBlockOpen => {
                    if open_html_block_element_stack.peek() != Some(&HTMLBlockElementType::Div) {
                        open_html_block_element_stack.push(HTMLBlockElementType::Div);
                    }
                    tokens.push(line);
                }
                LineType::HTMLFigureBlockOpen => {
                    if open_html_block_element_stack.peek() != Some(&HTMLBlockElementType::Figure) {
                        open_html_block_element_stack.push(HTMLBlockElementType::Figure);
                    }
                    tokens.push(line);
                }
                LineType::HTMLTableHeadOpen => {
                    if open_html_block_element_stack.peek()
                        != Some(&HTMLBlockElementType::TableHead)
                    {
                        open_html_block_element_stack.push(HTMLBlockElementType::TableHead);
                    }
                    tokens.push(line);
                }
                LineType::HTMLTableBodyOpen => {
                    if open_html_block_element_stack.peek()
                        != Some(&HTMLBlockElementType::TableBody)
                    {
                        open_html_block_element_stack.pop();
                        open_html_block_element_stack.push(HTMLBlockElementType::TableBody);
                    }
                    tokens.push(line);
                }
                _ => tokens.push(line),
            },
            None => {
                while !open_lists.is_empty() {
                    match open_lists.pop() {
                        Some(ListType::Unordered) => tokens.push(String::from("</ul>")),
                        Some(ListType::Ordered) => {
                            tokens.push(String::from("</ol>"));
                            open_markdown_block_stack.pop();
                        }
                        None => {}
                    }
                }
            }
        };
    }
    let astro_frontmatter = form_astro_frontmatter(
        &present_jsx_component_types,
        &astro_frontmatter_markup,
        slug,
    );
    if verbose {
        for frontmatter_line in &astro_frontmatter {
            println!("{frontmatter_line}");
        }
        for token in &tokens {
            println!("{token}");
        }
        println! {"\n"};
    }

    let Ok(mut outfile) = File::create(output_path) else {
        panic!(
            "[ ERROR ] Was not able to create the output file: {:?}!",
            output_path.as_ref().display().to_string()
        )
    };

    // Experimental formatting currently disabled
    let format = false;

    if format {
        let mut cursor = Cursor::new(Vec::new());
        for line in &astro_frontmatter {
            cursor
                .write_all(line.as_bytes())
                .expect("[ ERROR ] Intermediate Astro buffer should have access to enough memory for markup.");
        }
        for line in &tokens {
            cursor.write_all(line.as_bytes()).expect(
                "[ ERROR ] Intermediate Astro buffer should have access to enough memory for markup."
                );
        }

        let mut buffer = Vec::new();
        cursor.rewind().unwrap();
        cursor.read_to_end(&mut buffer).unwrap();

        let options = FormatOptions::default();
        let formatted = format_text(
            std::str::from_utf8(&buffer)
                .expect("[ ERROR ] Astro markup should not contain UTF-8 characters."),
            Language::Astro,
            &options,
            |code, _| Ok::<_, std::convert::Infallible>(code.into()),
        )
        .unwrap_or_else(|_| {
            panic!(
            "[ ERROR ] Unformatted intermediate file `{}` should not contain syntactical errors.",
            output_path.as_ref().display())
        });
        let _ = outfile.write_all(formatted.as_bytes());
    } else {
        for line in &astro_frontmatter {
            outfile
                .write_all(line.as_bytes())
                .expect("[ ERROR ] Was not able to create the output file!");
            outfile
                .write_all(b"\n")
                .expect("[ ERROR ] Was not able to create the output file!");
        }
        for line in &tokens {
            outfile
                .write_all(line.as_bytes())
                .expect("[ ERROR ] Was not able to create the output file!");
            outfile
                .write_all(b"\n")
                .expect("[ ERROR ] Was not able to create the output file!");
        }
    }

    let duration = start.elapsed();
    let duration_milliseconds = duration.as_millis();
    let duration_microseconds = duration.as_micros() - (duration_milliseconds * 1000);
    let file_size = file.metadata().unwrap().len() / 1000;
    println!("[ INFO ] Parsing complete ({file_size} KB) in {duration_milliseconds}.{duration_microseconds:0>3} ms.");
}
