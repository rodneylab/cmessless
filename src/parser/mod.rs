#[cfg(test)]
mod tests;

use nom::{
    branch::alt,
    bytes::complete::{tag, tag_no_case, take_until},
    character::complete::{digit1, multispace0, multispace1},
    combinator::rest,
    multi::{many0, many0_count, many1_count},
    sequence::{delimited, preceded, separated_pair, terminated, tuple},
    IResult,
};
use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::{BufRead, BufReader, Write},
    path::Path,
};

#[derive(Debug, PartialEq, Eq, Hash)]
enum JSXComponentType {
    CodeFragment,
    Image,
    Poll,
    Questions,
    Tweet,
    Video,
}

#[derive(Debug, PartialEq)]
enum LineType {
    CodeFragmentOpen,
    CodeFragmentClose,
    JSXComponent,
    Heading,
    Image,
    OrderedListItem,
    Paragraph,
    PollOpen,
    PollClose,
    Questions,
    Tweet,
    UnorderedListItem,
    VideoOpen,
    VideoClose,
}

#[derive(Debug, PartialEq)]
enum ListType {
    Ordered,
    Unordered,
}

struct ListStack {
    structure: Vec<ListType>,
}

impl ListStack {
    fn new() -> Self {
        ListStack {
            structure: Vec::new(),
        }
    }

    fn pop(&mut self) -> Option<ListType> {
        self.structure.pop()
    }

    fn push(&mut self, element: ListType) {
        self.structure.push(element)
    }

    fn len(&self) -> usize {
        self.structure.len()
    }

    fn is_empty(&self) -> bool {
        self.structure.is_empty()
    }
}

#[allow(dead_code)]
fn discard_leading_whitespace(line: &str) -> IResult<&str, &str> {
    preceded(multispace0, rest)(line)
}

fn parse_author_name_from_cargo_pkg_authors(cargo_pkg_authors: &str) -> IResult<&str, &str> {
    take_until(" <")(cargo_pkg_authors)
}

pub fn author_name_from_cargo_pkg_authors() -> &'static str {
    match parse_author_name_from_cargo_pkg_authors(env!("CARGO_PKG_AUTHORS")) {
        Ok((_, result)) => result,
        Err(_) => panic!("[ ERROR ] Authors does not seem to be defined!"),
    }
}

// consumes delimiter
fn parse_up_to_inline_wrap_segment<'a>(
    line: &'a str,
    delimiter: &'a str,
) -> IResult<&'a str, (&'a str, &'a str)> {
    separated_pair(take_until(delimiter), tag(delimiter), rest)(line)
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

fn segment_anchor_element_line(line: &str) -> IResult<&str, (&str, &str, &str)> {
    let delimiter = "a";
    let (remainder, initial_segment) = parse_up_to_opening_html_tag(line, delimiter)?;
    let (final_segment, anchor_attributes_segment) = parse_opening_html_tag(remainder, delimiter)?;
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
    tuple((
        preceded(multispace0, take_until("=")),
        delimited(tag("=\""), take_until("\""), tag("\"")),
    ))(line)
}

fn parse_html_tag_attributes(attributes: &str) -> IResult<&str, Vec<(&str, &str)>> {
    many0(parse_html_tag_attribute)(attributes)
}

fn parse_href_scheme(href: &str) -> IResult<&str, &str> {
    alt((tag_no_case("HTTP://"), tag_no_case("HTTPS://")))(href)
}

fn form_html_anchor_element_line(line: &str) -> IResult<&str, String> {
    let (_, (initial_segment, anchor_attributes_segment, final_segment)) =
        segment_anchor_element_line(line)?;
    let (_, attributes_vector) = parse_html_tag_attributes(anchor_attributes_segment)?;

    let attributes_hash_map: HashMap<&str, &str> = attributes_vector.into_iter().collect();
    let href = attributes_hash_map["href"];
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

    Ok((
        final_segment,
        format!("{initial_segment}<a {anchor_attributes_segment}{additional_attributes}>"),
    ))
}

fn form_code_span_line(line: &str) -> IResult<&str, String> {
    let (_, (initial_segment, bold_segment, final_segment)) = segment_code_span_line(line)?;
    Ok((
        final_segment,
        format!("{initial_segment}<code>{bold_segment}</code>"),
    ))
}

fn parse_jsx_component<'a>(
    line: &'a str,
    component_identifier: &'a str,
) -> IResult<&'a str, &'a str> {
    let delimiter = &mut String::from("<");
    delimiter.push_str(component_identifier);
    let result = delimited(tag(delimiter.as_str()), take_until("/>"), tag("/>"))(line);
    result
}

fn parse_jsx_component_first_line<'a>(
    line: &'a str,
    component_identifier: &'a str,
) -> IResult<&'a str, &'a str> {
    let delimiter = &mut String::from("<");
    delimiter.push_str(component_identifier);
    let result = tag(delimiter.as_str())(line);
    result
}

fn parse_jsx_component_last_line<'a>(
    line: &'a str,
    component_identifier: &'a str,
) -> IResult<&'a str, &'a str> {
    let delimiter = &mut String::from("</");
    delimiter.push_str(component_identifier);
    let result = tag(delimiter.as_str())(line);
    result
}

fn form_code_fragment_component_first_line(line: &str) -> IResult<&str, (String, LineType, usize)> {
    let component_identifier = "CodeFragment";
    let (final_segment, initial_segment) =
        parse_jsx_component_first_line(line, component_identifier)?;
    Ok((
        "",
        (
            format!("{initial_segment}{final_segment}"),
            LineType::CodeFragmentOpen,
            0,
        ),
    ))
}

fn form_image_component(line: &str) -> IResult<&str, (String, LineType, usize)> {
    let component_identifier = "Image";
    let (_, attributes) = parse_jsx_component(line, component_identifier)?;
    Ok(("", (format!("<Image{attributes}/>"), LineType::Image, 0)))
}

fn form_tweet_component(line: &str) -> IResult<&str, (String, LineType, usize)> {
    let component_identifier = "Tweet";
    let (_, attributes) = parse_jsx_component(line, component_identifier)?;
    Ok(("", (format!("<Tweet{attributes}/>"), LineType::Tweet, 0)))
}

fn form_poll_component_first_line(line: &str) -> IResult<&str, (String, LineType, usize)> {
    let component_identifier = "Poll";
    let (final_segment, initial_segment) =
        parse_jsx_component_first_line(line, component_identifier)?;
    Ok((
        "",
        (
            format!("{initial_segment}{final_segment}"),
            LineType::PollOpen,
            0,
        ),
    ))
}

fn form_questions_component(line: &str) -> IResult<&str, (String, LineType, usize)> {
    let component_identifier = "Questions";
    let (_, attributes) = parse_jsx_component(line, component_identifier)?;
    Ok((
        "",
        (format!("<Questions{attributes}/>"), LineType::Questions, 0),
    ))
}

fn form_video_component_first_line(line: &str) -> IResult<&str, (String, LineType, usize)> {
    let component_identifier = "Video";
    let (final_segment, initial_segment) =
        parse_jsx_component_first_line(line, component_identifier)?;
    Ok((
        "",
        (
            format!("{initial_segment}{final_segment}"),
            LineType::VideoOpen,
            0,
        ),
    ))
}

fn form_code_fragment_component_last_line(line: &str) -> IResult<&str, (String, LineType, usize)> {
    let component_identifier = "CodeFragment";
    let (final_segment, initial_segment) =
        parse_jsx_component_last_line(line, component_identifier)?;
    Ok((
        "",
        (
            format!("{initial_segment}{final_segment}"),
            LineType::CodeFragmentClose,
            0,
        ),
    ))
}

fn form_poll_component_last_line(line: &str) -> IResult<&str, (String, LineType, usize)> {
    let component_identifier = "Poll";
    let (final_segment, initial_segment) =
        parse_jsx_component_last_line(line, component_identifier)?;
    Ok((
        "",
        (
            format!("{initial_segment}{final_segment}"),
            LineType::PollClose,
            0,
        ),
    ))
}

fn form_video_component_last_line(line: &str) -> IResult<&str, (String, LineType, usize)> {
    let component_identifier = "Video";
    let (final_segment, initial_segment) =
        parse_jsx_component_last_line(line, component_identifier)?;
    Ok((
        "",
        (
            format!("{initial_segment}{final_segment}"),
            LineType::VideoClose,
            0,
        ),
    ))
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
    Ok((
        final_segment,
        format!("{initial_segment}<strong>{bold_segment}</strong>"),
    ))
}

fn parse_inline_wrap_text(line: &str) -> IResult<&str, String> {
    let (initial_segment, final_segment): (String, &str) = match alt((
        form_strong_emphasis_line,
        form_emphasis_line,
        form_code_span_line,
        form_html_anchor_element_line,
    ))(line)
    {
        Ok((value_1, value_2)) => (value_2, value_1),
        Err(_) => return Ok(("", line.to_string())),
    };

    let (_, final_final_segment) = parse_inline_wrap_text(final_segment)?;
    Ok(("", format!("{initial_segment}{final_final_segment}")))
}

fn parse_opening_html_tag<'a>(line: &'a str, element_tag: &'a str) -> IResult<&'a str, &'a str> {
    let delimiter = &mut String::from("<");
    delimiter.push_str(element_tag);
    delimited(
        tag("<a"),
        delimited(multispace0, take_until(">"), multispace0),
        tag(">"),
    )(line)
}

fn parse_heading_text(line: &str) -> IResult<&str, usize> {
    let (heading, level) = terminated(many1_count(tag("#")), multispace1)(line)?;
    Ok((heading, level))
}

// consumes delimiter
fn parse_inline_wrap_segment<'a>(
    line: &'a str,
    delimiter: &'a str,
) -> IResult<&'a str, (&'a str, &'a str)> {
    separated_pair(take_until(delimiter), tag(delimiter), rest)(line)
}

fn parse_ordered_list_text(line: &str) -> IResult<&str, usize> {
    let (heading, indentation) =
        terminated(many0_count(tag(" ")), preceded(digit1, tag(". ")))(line)?;
    Ok((heading, indentation))
}

fn parse_unordered_list_text(line: &str) -> IResult<&str, usize> {
    let (heading, indentation) = terminated(many0_count(tag(" ")), tag("- "))(line)?;
    Ok((heading, indentation))
}

fn form_heading_line(line: &str) -> IResult<&str, (String, LineType, usize)> {
    let (value, level) = parse_heading_text(line)?;
    Ok((
        "",
        (
            format!("<h{level}>{value}</h{level}>"),
            LineType::Heading,
            level,
        ),
    ))
}

fn form_ordered_list_line(line: &str) -> IResult<&str, (String, LineType, usize)> {
    // let (list_text, indentation) = parse_ordered_list_text(line)?;
    let (list_text, indentation) = parse_ordered_list_text(line)?;
    let (_, parsed_list_text) = parse_inline_wrap_text(list_text)?;
    Ok((
        "",
        (
            format!("<li>{parsed_list_text}</li>"),
            LineType::OrderedListItem,
            indentation,
        ),
    ))
}

fn form_unordered_list_line(line: &str) -> IResult<&str, (String, LineType, usize)> {
    let (list_text, indentation) = parse_unordered_list_text(line)?;
    Ok((
        "",
        (
            format!("<li>{list_text}</li>"),
            LineType::UnorderedListItem,
            indentation,
        ),
    ))
}

fn form_inline_wrap_text(line: &str) -> IResult<&str, (String, LineType, usize)> {
    let (_, parsed_line) = parse_inline_wrap_text(line)?;
    if !parsed_line.is_empty() {
        Ok((
            "",
            (format!("<p>{parsed_line}</p>"), LineType::Paragraph, 0),
        ))
    } else {
        Ok(("", (String::new(), LineType::Paragraph, 0)))
    }
}

fn form_astro_frontmatter(components: &HashSet<JSXComponentType>, slug: &str) -> Vec<String> {
    let mut result: Vec<String> = Vec::new();
    let mut define_slug = false;
    let mut image_data_imports: Vec<String> = Vec::new();

    result.push(String::from("---"));
    if components.contains(&JSXComponentType::CodeFragment) {
        result.push(String::from(
            "import CodeFragment from '$components/CodeFragment.tsx';",
        ));
    }
    if components.contains(&JSXComponentType::Image) {
        image_data_imports.push(String::from("imageProps"));
        result.push(String::from(
            "import Image from '$components/BlogPost/Image.svelte';",
        ));
        result.push(format!("import imageData from '$generated/blog/{slug}';"));
    }
    if components.contains(&JSXComponentType::Poll) {
        result.push(String::from("import Poll from '$components/Poll.svelte';"));
    }
    if components.contains(&JSXComponentType::Questions) {
        result.push(String::from(
            "import Poll from '$components/Questions.svelte'",
        ));
    }
    if components.contains(&JSXComponentType::Tweet) {
        result.push(String::from(
            "import Tweet from '$components/Tweet.svelte';",
        ));
    }
    if components.contains(&JSXComponentType::Video) {
        define_slug = true;
        image_data_imports.push(String::from("poster"));
        result.push(String::from(
            "import Video from '$components/Video.svelte';",
        ));
    }

    if !image_data_imports.is_empty() {
        let mut line = format!("\nconst {{ {}", image_data_imports[0]);
        for import in &image_data_imports[1..] {
            line.push_str(", ");
            line.push_str(import.as_str());
        }
        line.push_str(" } = imageData");
        result.push(line);
    }
    if define_slug {
        result.push(format!("\nconst slug = '{slug}';"));
    }
    result.push(String::from("---\n"));
    result
}

fn parse_mdx_line(
    line: &str,
    open_jsx_component_type: &Option<JSXComponentType>,
) -> Option<(String, LineType, usize)> {
    match open_jsx_component_type {
        Some(_) => {
            match alt((
                form_code_fragment_component_last_line,
                form_poll_component_last_line,
                form_video_component_last_line,
            ))(line)
            {
                Ok((_, (line, line_type, level))) => {
                    if !line.is_empty() {
                        Some((line, line_type, level))
                    } else {
                        None
                    }
                }
                Err(_) => Some((line.to_string(), LineType::JSXComponent, 0)),
            }
        }
        None => {
            match alt((
                form_code_fragment_component_first_line,
                form_image_component,
                form_poll_component_first_line,
                form_questions_component,
                form_tweet_component,
                form_video_component_first_line,
                form_heading_line,
                form_ordered_list_line,
                form_unordered_list_line,
                form_inline_wrap_text,
            ))(line)
            {
                Ok((_, (line, line_type, level))) => {
                    if !line.is_empty() {
                        Some((line, line_type, level))
                    } else {
                        None
                    }
                }
                Err(_) => None,
            }
        }
    }
}

pub fn parse_mdx_file(_filename: &str) {
    println!("[ INFO ] Trying to parse {}...", _filename);

    let input_filename = Path::new(_filename);
    let file = File::open(&input_filename).expect("[ ERROR ] Couldn't open that file!");
    let slug = match input_filename
        .file_stem()
        .expect("[ ERROR ] Couldn't open that file!")
        .to_str()
    {
        Some(value) => match value {
            "index" => &input_filename
                .parent()
                .expect("[ ERROR ] Couldn't open that file!")
                .to_str()
                .expect("[ ERROR ] Couldn't open that file!")[1..],
            other => other,
        },
        None => panic!("[ ERROR ] Couldn't open that file!"),
    };

    let mut tokens: Vec<String> = Vec::new();
    let reader = BufReader::new(file);

    let mut current_indentation: usize = 0;
    let mut open_lists = ListStack::new();
    let mut open_jsx_component_type: Option<JSXComponentType> = None;
    let mut present_jsx_component_types: HashSet<JSXComponentType> = HashSet::new();

    for line in reader.lines() {
        let line_content = line.unwrap();
        match parse_mdx_line(&line_content, &open_jsx_component_type) {
            Some((line, line_type, indentation)) => match line_type {
                LineType::OrderedListItem => {
                    if open_lists.is_empty() {
                        open_lists.push(ListType::Ordered);
                        tokens.push(format!("<ol>\n  {line}"));
                    } else if indentation > current_indentation {
                        open_lists.push(ListType::Ordered);
                        let list_item_indentation = " ".repeat(2 * open_lists.len());
                        tokens.push(format!("<ol>\n{list_item_indentation}{line}"));
                    } else if indentation == current_indentation {
                        let list_item_indentation = " ".repeat(2 * open_lists.len());
                        tokens.push(format!("{list_item_indentation}{line}"));
                    } else {
                        while open_lists.pop() != Some(ListType::Ordered) {
                            tokens.push(String::from("</ul>"));
                        }
                        tokens.push(String::from("</ol>"));
                    }
                    current_indentation = indentation
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
                        }
                        tokens.push(String::from("</ul>"));
                    }
                    current_indentation = indentation
                }
                LineType::CodeFragmentClose | LineType::PollClose | LineType::VideoClose => {
                    open_jsx_component_type = None;
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
                LineType::Tweet => {
                    present_jsx_component_types.insert(JSXComponentType::Tweet);
                    tokens.push(line);
                }
                LineType::CodeFragmentOpen => {
                    present_jsx_component_types.insert(JSXComponentType::CodeFragment);
                    open_jsx_component_type = Some(JSXComponentType::CodeFragment);
                    tokens.push(line);
                }
                LineType::PollOpen => {
                    present_jsx_component_types.insert(JSXComponentType::Poll);
                    open_jsx_component_type = Some(JSXComponentType::Poll);
                    tokens.push(line);
                }
                LineType::VideoOpen => {
                    present_jsx_component_types.insert(JSXComponentType::Video);
                    open_jsx_component_type = Some(JSXComponentType::Video);
                    tokens.push(line);
                }
                _ => tokens.push(line),
            },
            None => {
                while !open_lists.is_empty() {
                    match open_lists.pop() {
                        Some(ListType::Unordered) => tokens.push(String::from("</ul>")),
                        Some(ListType::Ordered) => tokens.push(String::from("</ol>")),
                        None => {}
                    }
                }
            }
        };
    }
    let astro_frontmatter = form_astro_frontmatter(&present_jsx_component_types, slug);
    for frontmatter_line in &astro_frontmatter {
        println!("{frontmatter_line}");
    }
    for token in &tokens {
        println!("{token}");
    }

    let mut output_filename = String::from(&_filename[.._filename.len() - 3]);
    output_filename.push_str("astro");
    let mut outfile =
        File::create(output_filename).expect("[ ERROR ] Was not able to create the output file!");

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
    println!("[ INFO ] Parsing complete!")
}
