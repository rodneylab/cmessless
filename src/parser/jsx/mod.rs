#[cfg(test)]
mod tests;

use crate::parser::{
    escape_code, form_code_fragment_component_last_line, form_fenced_code_block_first_line,
    form_fenced_code_block_last_line, parse_closing_html_tag, parse_opening_html_tag,
    parse_opening_html_tag_end, parse_opening_html_tag_start, parse_self_closing_html_tag,
    parse_self_closing_html_tag_end, HTMLTagType, LineType,
};
use crate::utility::stack::Stack;

use nom::{
    branch::alt,
    bytes::complete::{tag, take_until},
    combinator::{all_consuming, map, rest, value},
    error::{Error, ErrorKind},
    sequence::{delimited, preceded, terminated},
    Err, IResult,
};

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum JSXComponentType {
    CodeFragment,
    CodeFragmentOpening,
    FencedCodeBlock,
    GatsbyNotMaintained,
    HowTo,
    HowToOpening,
    Image,
    Poll,
    PollOpening,
    Questions,
    Tweet,
    Video,
    VideoOpening,
}

pub struct JSXComponentRegister {
    components: Stack<JSXComponentType>,
}

impl JSXComponentRegister {
    pub fn new() -> JSXComponentRegister {
        JSXComponentRegister {
            components: Stack::new(),
        }
    }

    pub fn peek(&self) -> Option<&JSXComponentType> {
        self.components.peek()
    }
    pub fn pop(&mut self) -> Option<JSXComponentType> {
        self.components.pop()
    }
    pub fn push(&mut self, component: JSXComponentType) {
        self.components.push(component)
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum JSXTagType {
    SelfClosed,
    Opened,
    Closed,
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
) -> IResult<&'a str, (&'a str, &'a JSXTagType)> {
    let left_delimiter = &mut String::from("<");
    left_delimiter.push_str(component_identifier);
    let result = alt((
        value(
            (line, &JSXTagType::SelfClosed),
            delimited(tag(left_delimiter.as_str()), take_until("/>"), tag("/>")),
        ),
        value(
            (line, &JSXTagType::Closed),
            delimited(tag(left_delimiter.as_str()), take_until(">"), tag(">")),
        ),
        value(
            (line, &JSXTagType::Opened),
            preceded(tag(left_delimiter.as_str()), rest),
        ),
    ))(line)?;
    Ok(result)
}

pub fn parse_jsx_component_last_line<'a>(
    line: &'a str,
    component_identifier: &'a str,
) -> IResult<&'a str, &'a str> {
    let delimiter = &mut String::from("</");
    delimiter.push_str(component_identifier);
    let result = tag(delimiter.as_str())(line);
    result
}

fn form_jsx_component_first_line<'a>(
    line: &'a str,
    component_identifier: &'a str,
) -> IResult<&'a str, (String, HTMLTagType, usize)> {
    let (remaining_line, (component_name, _attributes, tag_type)) = alt((
        parse_self_closing_html_tag,
        parse_opening_html_tag,
        parse_opening_html_tag_start,
    ))(line)?;
    all_consuming(tag(component_identifier))(component_name)?; // check names match
    match tag_type {
        HTMLTagType::Opening | HTMLTagType::OpeningStart | HTMLTagType::SelfClosing => {
            Ok((remaining_line, (line.to_string(), tag_type, 0)))
        }
        HTMLTagType::Closing => Err(Err::Error(Error::new(line, ErrorKind::Tag))),
    }
}

// assumed tag is opened in earlier line and this has been recognised
fn form_jsx_component_opening_line(line: &str) -> IResult<&str, (String, HTMLTagType, usize)> {
    let (remaining_line, (_attributes, tag_type)) =
        alt((parse_self_closing_html_tag_end, parse_opening_html_tag_end))(line)?;
    match tag_type {
        HTMLTagType::Opening | HTMLTagType::SelfClosing => {
            Ok((remaining_line, (line.to_string(), tag_type, 0)))
        }
        HTMLTagType::OpeningStart | HTMLTagType::Closing => {
            Err(Err::Error(Error::new(line, ErrorKind::Tag)))
        }
    }
}

fn form_jsx_component_last_line<'a>(
    line: &'a str,
    component_identifier: &'a str,
) -> IResult<&'a str, (String, HTMLTagType, usize)> {
    let (_remaining_line, (component_name, _attributes, tag_type)) = parse_closing_html_tag(line)?;
    all_consuming(tag(component_identifier))(component_name)?; // check names match
    match tag_type {
        HTMLTagType::Closing => Ok((_remaining_line, (line.to_string(), tag_type, 0))),
        HTMLTagType::Opening | HTMLTagType::OpeningStart | HTMLTagType::SelfClosing => {
            Err(Err::Error(Error::new(line, ErrorKind::Tag)))
        }
    }
}

pub fn form_code_fragment_component_first_line(
    line: &str,
) -> IResult<&str, (String, LineType, usize)> {
    let component_identifier = "CodeFragment";
    let (_, (_parsed_value, jsx_tag_type)) =
        parse_jsx_component_first_line(line, component_identifier)?;
    match jsx_tag_type {
        JSXTagType::Closed => Ok(("", (line.to_string(), LineType::CodeFragmentOpen, 0))),
        JSXTagType::Opened => Ok(("", (line.to_string(), LineType::CodeFragmentOpening, 0))),
        JSXTagType::SelfClosed => Ok(("", (line.to_string(), LineType::CodeFragment, 0))),
    }
}

pub fn form_how_to_component_first_line(line: &str) -> IResult<&str, (String, LineType, usize)> {
    let (remaining_line, (markup, tag_type, indentation)) =
        form_jsx_component_first_line(line, "HowTo")?;
    match tag_type {
        HTMLTagType::Opening => Ok((remaining_line, (markup, LineType::HowToOpen, indentation))),
        HTMLTagType::OpeningStart => Ok(("", (markup, LineType::HowToOpening, indentation))),
        HTMLTagType::SelfClosing => Ok((remaining_line, (markup, LineType::HowTo, indentation))),
        HTMLTagType::Closing => Err(Err::Error(Error::new(line, ErrorKind::Tag))),
    }
}

pub fn form_image_component(line: &str) -> IResult<&str, (String, LineType, usize)> {
    let component_identifier = "Image";
    let (_, attributes) = parse_jsx_component(line, component_identifier)?;
    Ok(("", (format!("<Image{attributes}/>"), LineType::Image, 0)))
}

pub fn form_gatsby_not_maintained_component(
    line: &str,
) -> IResult<&str, (String, LineType, usize)> {
    let component_identifier = "GatsbyNotMaintained";
    let (_, attributes) = parse_jsx_component(line, component_identifier)?;
    Ok((
        "",
        (
            format!("<GatsbyNotMaintained{attributes}/>"),
            LineType::GatsbyNotMaintained,
            0,
        ),
    ))
}

pub fn form_tweet_component(line: &str) -> IResult<&str, (String, LineType, usize)> {
    let component_identifier = "Tweet";
    let (_, attributes) = parse_jsx_component(line, component_identifier)?;
    Ok(("", (format!("<Tweet{attributes}/>"), LineType::Tweet, 0)))
}

pub fn form_poll_component_first_line(line: &str) -> IResult<&str, (String, LineType, usize)> {
    let component_identifier = "Poll";
    let (_, (_parsed_value, jsx_tag_type)) =
        parse_jsx_component_first_line(line, component_identifier)?;
    match jsx_tag_type {
        JSXTagType::Closed => Ok(("", (line.to_string(), LineType::PollOpen, 0))),
        JSXTagType::Opened => Ok(("", (line.to_string(), LineType::PollOpening, 0))),
        JSXTagType::SelfClosed => Ok(("", (line.to_string(), LineType::Poll, 0))),
    }
}

pub fn form_questions_component(line: &str) -> IResult<&str, (String, LineType, usize)> {
    let component_identifier = "Questions";
    let (_, attributes) = parse_jsx_component(line, component_identifier)?;
    Ok((
        "",
        (format!("<Questions{attributes}/>"), LineType::Questions, 0),
    ))
}

pub fn form_video_component_first_line(line: &str) -> IResult<&str, (String, LineType, usize)> {
    let component_identifier = "Video";
    let (_, (__parsed_value_, jsx_tag_type)) =
        parse_jsx_component_first_line(line, component_identifier)?;
    match jsx_tag_type {
        JSXTagType::Closed => Ok(("", (line.to_string(), LineType::VideoOpen, 0))),
        JSXTagType::Opened => Ok(("", (line.to_string(), LineType::VideoOpening, 0))),
        JSXTagType::SelfClosed => Ok(("", (line.to_string(), LineType::Video, 0))),
    }
}

// handles the continuation of an opening tag
pub fn form_how_to_component_opening_line(line: &str) -> IResult<&str, (String, LineType, usize)> {
    let (remaining_line, (markup, tag_type, indentation)) = form_jsx_component_opening_line(line)?;
    match tag_type {
        HTMLTagType::Opening | HTMLTagType::SelfClosing => {
            Ok((remaining_line, (markup, LineType::HowToOpen, indentation)))
        }
        _ => Ok((
            "",
            (String::from(line), LineType::HowToOpening, indentation),
        )),
    }
}

pub fn form_poll_component_opening_line(line: &str) -> IResult<&str, (String, LineType, usize)> {
    let (_, line_type) = alt((
        map(terminated(take_until("/>"), tag("/>")), |_| LineType::Poll),
        map(terminated(take_until(">"), tag(">")), |_| {
            LineType::PollOpen
        }),
        map(rest, |_| LineType::PollOpening),
    ))(line)?;
    Ok(("", (line.to_string(), line_type, 0)))
}

pub fn form_video_component_opening_line(line: &str) -> IResult<&str, (String, LineType, usize)> {
    let (remaining_line, (markup, tag_type, indentation)) = form_jsx_component_opening_line(line)?;
    match tag_type {
        HTMLTagType::Opening => Ok((remaining_line, (markup, LineType::VideoOpen, indentation))),
        HTMLTagType::SelfClosing => Ok((remaining_line, (markup, LineType::Video, indentation))),
        _ => Ok((
            "",
            (String::from(line), LineType::VideoOpening, indentation),
        )),
    }
}

// assumed tag is already open
pub fn form_how_to_component_last_line(line: &str) -> IResult<&str, (String, LineType, usize)> {
    let (remaining_line, (markup, tag_type, indentation)) =
        form_jsx_component_last_line(line, "HowTo")?;
    match tag_type {
        HTMLTagType::Closing => Ok((remaining_line, (markup, LineType::HowTo, indentation))),
        HTMLTagType::Opening | HTMLTagType::OpeningStart | HTMLTagType::SelfClosing => {
            Ok((remaining_line, (markup, LineType::HowToOpen, indentation)))
        }
    }
}

fn form_poll_component_last_line(line: &str) -> IResult<&str, (String, LineType, usize)> {
    let component_identifier = "Poll";
    let (final_segment, initial_segment) =
        parse_jsx_component_last_line(line, component_identifier)?;
    Ok((
        "",
        (
            format!("{initial_segment}{final_segment}"),
            LineType::Poll,
            0,
        ),
    ))
}

pub fn form_video_component_last_line(line: &str) -> IResult<&str, (String, LineType, usize)> {
    let component_identifier = "Video";
    let (final_segment, initial_segment) =
        parse_jsx_component_last_line(line, component_identifier)?;
    Ok((
        "",
        (
            format!("{initial_segment}{final_segment}"),
            LineType::Video,
            0,
        ),
    ))
}

pub fn parse_open_jsx_block(
    line: &str,
    open_jsx_component_type: Option<&JSXComponentType>,
) -> Option<(String, LineType, usize)> {
    match open_jsx_component_type {
        Some(JSXComponentType::HowToOpening) => match form_how_to_component_opening_line(line) {
            Ok((_, (line, line_type, level))) => {
                if !line.is_empty() {
                    Some((line, line_type, level))
                } else {
                    None
                }
            }
            Err(_) => Some((line.to_string(), LineType::HowToOpening, 0)),
        },
        Some(JSXComponentType::PollOpening) => match form_poll_component_opening_line(line) {
            Ok((_, (line, line_type, level))) => {
                if !line.is_empty() {
                    Some((line, line_type, level))
                } else {
                    None
                }
            }
            Err(_) => Some((line.to_string(), LineType::JSXComponent, 0)),
        },
        Some(JSXComponentType::VideoOpening) => match form_video_component_opening_line(line) {
            Ok((_, (line, line_type, level))) => {
                if !line.is_empty() {
                    Some((line, line_type, level))
                } else {
                    None
                }
            }
            Err(_) => Some((line.to_string(), LineType::JSXComponent, 0)),
        },
        Some(JSXComponentType::FencedCodeBlock) => {
            match alt((form_fenced_code_block_last_line,))(line) {
                Ok((_, (line, line_type, level))) => {
                    if !line.is_empty() {
                        Some((line, line_type, level))
                    } else {
                        None
                    }
                }
                Err(_) => Some((escape_code(line), LineType::FencedCodeBlockOpen, 0)),
            }
        }
        Some(JSXComponentType::HowTo) => match alt((
            form_fenced_code_block_first_line,
            form_video_component_first_line,
            form_how_to_component_last_line,
        ))(line)
        {
            Ok((_, (line, line_type, level))) => {
                if !line.is_empty() {
                    Some((line, line_type, level))
                } else {
                    None
                }
            }
            Err(_) => Some((line.to_string(), LineType::HowToOpen, 0)),
        },
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
        None => None,
    }
}
