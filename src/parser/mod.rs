#[cfg(test)]
mod tests;

use nom::{
    branch::alt,
    bytes::complete::{tag, tag_no_case, take_until},
    character::complete::{multispace0, multispace1},
    combinator::rest,
    multi::{many0, many1_count},
    sequence::{delimited, preceded, separated_pair, terminated, tuple},
    IResult,
};
use std::{
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader, Write},
    path::Path,
};

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

fn parse_opening_html_tag<'a>(line: &'a str, element_tag: &'a str) -> IResult<&'a str, &'a str> {
    let delimiter = &mut String::from("<");
    delimiter.push_str(element_tag);
    delimited(
        tag("<a"),
        delimited(multispace0, take_until(">"), multispace0),
        tag(">"),
    )(line)
}

// consumes delimiter
fn parse_inline_wrap_segment<'a>(
    line: &'a str,
    delimiter: &'a str,
) -> IResult<&'a str, (&'a str, &'a str)> {
    separated_pair(take_until(delimiter), tag(delimiter), rest)(line)
}

fn parse_heading_text(line: &str) -> IResult<&str, usize> {
    let (heading, level) = terminated(many1_count(tag("#")), multispace1)(line)?;
    Ok((heading, level))
}

fn form_heading_line(line: &str) -> IResult<&str, String> {
    let (value, level) = parse_heading_text(line)?;
    Ok(("", format!("<h{level}>{value}</h{level}>")))
}

fn form_inline_wrap_text(line: &str) -> IResult<&str, String> {
    let (_, parsed_line) = parse_inline_wrap_text(line)?;
    Ok(("", format!("<p>{parsed_line}</p>")))
}

fn parse_mdx_line(line: &str) -> Option<String> {
    match alt((form_heading_line, form_inline_wrap_text))(line) {
        Ok((_, value)) => Some(value),
        Err(_) => None,
    }
}

pub fn parse_mdx_file(_filename: &str) {
    println!("[ INFO ] Trying to parse {}...", _filename);

    let input_filename = Path::new(_filename);
    let file = File::open(&input_filename).expect("[ ERROR ] Couldn't open that file!");

    let mut tokens: Vec<String> = Vec::new();
    let reader = BufReader::new(file);

    for line in reader.lines() {
        let line_content = line.unwrap();
        if let Some(value) = parse_mdx_line(&line_content) {
            tokens.push(value)
        }
    }

    for token in &tokens {
        println!("{token}");
    }

    let mut output_filename = String::from(&_filename[.._filename.len() - 3]);
    output_filename.push_str("html");
    let mut outfile =
        File::create(output_filename).expect("[ ERROR ] Was not able to create the output file!");

    for line in &tokens {
        outfile
            .write_all(line.as_bytes())
            .expect("[ ERROR ] Was not able to create the output file!");
    }
    println!("[ INFO ] Parsing complete!")
}
