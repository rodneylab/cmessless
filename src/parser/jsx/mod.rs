#[cfg(test)]
mod tests;

use crate::{
    parser::{
        escape_code, form_fenced_code_block_first_line, form_fenced_code_block_last_line,
        parse_closing_html_tag, parse_html_tag_attributes, parse_opening_html_tag,
        parse_opening_html_tag_end, parse_opening_html_tag_start, parse_self_closing_html_tag,
        parse_self_closing_html_tag_end, HTMLTagType, LineType,
    },
    utility::stack::Stack,
};

use nom::{
    branch::alt,
    bytes::complete::{tag, take_until},
    combinator::{all_consuming, map, rest, value},
    error::{Error, ErrorKind},
    sequence::{delimited, preceded, terminated},
    Err, IResult,
};
use std::collections::HashMap;

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum JSXComponentType {
    CodeFragment,
    CodeFragmentOpening,
    FencedCodeBlock,
    GatsbyNotMaintained,
    HowTo,
    HowToOpening,
    HowToSection,
    HowToSectionOpening,
    HowToStep,
    HowToStepOpening,
    HowToDirection,
    HowToDirectionOpening,
    Image,
    Poll,
    PollOpening,
    Questions,
    Tweet,
    Video,
    VideoOpening,
}

struct HowToDirectionComponent {
    text: String,
}

impl HowToDirectionComponent {
    pub fn new(text: &str) -> HowToDirectionComponent {
        HowToDirectionComponent {
            text: text.to_string(),
        }
    }
}

struct HowToStepComponent {
    name: String,
    image: Option<String>,
    video: Option<String>,
    start: Option<u64>,
    end: Option<u64>,
    directions: Vec<HowToDirectionComponent>,
}

impl HowToStepComponent {
    pub fn new() -> HowToStepComponent {
        HowToStepComponent {
            name: "".to_string(),
            image: None,
            video: None,
            start: None,
            end: None,
            directions: Vec::new(),
        }
    }

    pub fn set_name(&mut self, name: &str) {
        self.name = name.to_string();
    }

    pub fn set_image(&mut self, image: &str) {
        self.image = Some(image.to_string());
    }

    pub fn set_video(&mut self, video: &str) {
        self.video = Some(video.to_string());
    }

    pub fn set_start(&mut self, start: u64) {
        self.start = Some(start);
    }

    pub fn set_end(&mut self, end: u64) {
        self.end = Some(end);
    }

    pub fn add_direction(&mut self, text: &str) -> usize {
        self.directions.push(HowToDirectionComponent::new(text));
        self.directions.len()
    }
}

struct HowToSectionComponent {
    name: String,
    steps: Vec<HowToStepComponent>,
}

impl HowToSectionComponent {
    pub fn new(name: &str) -> HowToSectionComponent {
        HowToSectionComponent {
            name: name.to_string(),
            steps: Vec::new(),
        }
    }

    pub fn add_step(&mut self) -> usize {
        self.steps.push(HowToStepComponent::new());
        self.steps.len()
    }
}

pub struct HowToComponent {
    props: HashMap<String, String>,
    sections: Vec<HowToSectionComponent>,
}

impl HowToComponent {
    pub fn new() -> HowToComponent {
        HowToComponent {
            props: HashMap::new(),
            sections: Vec::new(),
        }
    }

    pub fn add_section(&mut self, name: &str) -> usize {
        self.sections.push(HowToSectionComponent::new(name));
        self.sections.len()
    }

    // Returns (section_position, position)
    pub fn add_step(&mut self) -> (usize, usize) {
        let section_position = self.sections.len();
        let mut_sections: &mut Vec<HowToSectionComponent> = self.sections.as_mut();
        let last_section: &mut HowToSectionComponent = mut_sections
            .last_mut()
            .expect("Error adding How to Section");
        (section_position, last_section.add_step())
    }

    fn get_last_step(&mut self) -> &mut HowToStepComponent {
        let mut_sections: &mut Vec<HowToSectionComponent> = self.sections.as_mut();
        let last_section: &mut HowToSectionComponent = mut_sections
            .last_mut()
            .expect("Error getting last section while adding sep name");
        let steps: &mut Vec<HowToStepComponent> = last_section.steps.as_mut();
        steps
            .last_mut()
            .expect("Error getting last step while adding name")
    }

    pub fn add_step_name(&mut self, name: &str) {
        self.get_last_step().set_name(name);
    }

    pub fn add_step_image(&mut self, image: &str) {
        self.get_last_step().set_image(image);
    }

    pub fn add_step_video(&mut self, video: &str) {
        self.get_last_step().set_video(video);
    }

    pub fn add_step_start(&mut self, start: u64) {
        self.get_last_step().set_start(start);
    }

    pub fn add_step_end(&mut self, end: u64) {
        self.get_last_step().set_end(end);
    }

    pub fn add_direction(&mut self, text: &str) -> usize {
        self.get_last_step().add_direction(text)
    }
    pub fn insert_prop(&mut self, key: &str, value: &str) {
        self.props.insert(key.to_string(), value.to_string());
    }

    pub fn astro_frontmatter_markup(&self) -> Vec<String> {
        let mut result: Vec<String> = vec!["const howTo = {".to_string()];

        if self.props.contains_key("name") {
            result.push(format!("  name: \"{}\",", self.props.get("name").unwrap()));
        }
        if self.props.contains_key("description") {
            result.push(format!(
                "  description: \"{}\",",
                self.props.get("description").unwrap()
            ));
        }
        result.push("  sections: [".to_string());
        for (position, section) in self.sections.iter().enumerate() {
            result.push("    {".to_string());
            result.push(format!("      name: \"{}\",", section.name));
            result.push(format!("      position: {},", position + 1));
            result.push("      steps: [".to_string());
            for (step_position, step) in section.steps.iter().enumerate() {
                result.push("        {".to_string());
                result.push(format!("          name: \"{}\",", step.name));
                result.push(format!("          position: {},", step_position + 1));
                match &step.image {
                    Some(value) => result.push(format!("          image: \"{value}\",")),
                    None => {}
                }
                match &step.video {
                    Some(value) => result.push(format!("          video: \"{value}\",")),
                    None => {}
                }
                match &step.start {
                    Some(value) => result.push(format!("          start: {value},")),
                    None => {}
                }
                match &step.end {
                    Some(value) => result.push(format!("          end: {value},")),
                    None => {}
                }
                result.push("          directions: [".to_string());
                for (direction_position, direction) in step.directions.iter().enumerate() {
                    result.push("            {".to_string());
                    result.push(format!("              text: \"{}\",", direction.text));
                    result.push(format!(
                        "              position: {},",
                        direction_position + 1
                    ));
                    result.push("            }".to_string());
                }
                result.push("          ],".to_string());
                result.push("        },".to_string());
            }
            result.push("      ],".to_string());
            result.push("    },".to_string());
        }
        result.push("  ],".to_string());
        result.push("};".to_string());
        result
    }
}

pub struct JSXComponentRegister {
    components: Stack<JSXComponentType>,
    how_to: Option<HowToComponent>,
}

impl JSXComponentRegister {
    pub fn new() -> JSXComponentRegister {
        JSXComponentRegister {
            components: Stack::new(),
            how_to: None,
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

    pub fn add_how_to_section(&mut self, name: &str) -> usize {
        self.how_to
            .as_mut()
            .expect("Error adding How to Section")
            .add_section(name)
    }

    // Returns (section_position, position)
    pub fn add_how_to_step(&mut self) -> (usize, usize) {
        self.how_to
            .as_mut()
            .expect("Error adding How to Step")
            .add_step()
    }

    pub fn add_how_to_step_name(&mut self, name: &str) {
        let _ = &self
            .how_to
            .as_mut()
            .expect("Error adding How to Step Name")
            .add_step_name(name);
    }

    pub fn add_how_to_step_image(&mut self, image: &str) {
        let _ = &self
            .how_to
            .as_mut()
            .expect("Error adding How to Step Name")
            .add_step_image(image);
    }

    pub fn add_how_to_step_video(&mut self, video: &str) {
        let _ = &self
            .how_to
            .as_mut()
            .expect("Error adding How to Step Name")
            .add_step_video(video);
    }

    pub fn add_how_to_step_start(&mut self, start: &str) {
        let start_int: u64 = start
            .parse()
            .expect("Error parsing HowTo step video start time");
        let _ = &self
            .how_to
            .as_mut()
            .expect("Error adding HowTo step video start time")
            .add_step_start(start_int);
    }

    pub fn add_how_to_step_end(&mut self, end: &str) {
        let end_int: u64 = end
            .parse()
            .expect("Error parsing HowTo step video end time");
        let _ = &self
            .how_to
            .as_mut()
            .expect("Error adding HowTo step video end time")
            .add_step_end(end_int);
    }

    pub fn add_how_to_direction(&mut self, text: &str) -> usize {
        self.how_to
            .as_mut()
            .expect("Error adding How to Step Name")
            .add_direction(text)
    }

    pub fn insert_prop(&mut self, key: &str, value: &str) {
        match &self.how_to {
            Some(_) => {
                let _ = &self
                    .how_to
                    .as_mut()
                    .expect("Error inserting How to Prop")
                    .insert_prop(key, value);
            }
            None => {
                self.how_to = Some(HowToComponent::new());
                self.insert_prop(key, value);
            }
        };
    }

    pub fn how_to(&self) -> Option<&HowToComponent> {
        self.how_to.as_ref()
    }

    pub fn how_to_mut(&mut self) -> &mut Option<HowToComponent> {
        &mut self.how_to
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

fn parse_jsx_component_last_line<'a>(
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
) -> IResult<&'a str, (String, &'a str, HTMLTagType, usize)> {
    let (remaining_line, (component_name, attributes, tag_type)) = alt((
        parse_self_closing_html_tag,
        parse_opening_html_tag,
        parse_opening_html_tag_start,
    ))(line)?;
    all_consuming(tag(component_identifier))(component_name)?; // check names match
    match tag_type {
        HTMLTagType::Opening | HTMLTagType::OpeningStart | HTMLTagType::SelfClosing => {
            Ok((remaining_line, (line.to_string(), attributes, tag_type, 0)))
        }
        HTMLTagType::Closing => Err(Err::Error(Error::new(line, ErrorKind::Tag))),
    }
}

// assumed tag is opened in earlier line and this has been recognised
fn form_jsx_component_opening_line(
    line: &str,
) -> IResult<&str, (String, &str, HTMLTagType, usize)> {
    let (remaining_line, (attributes, tag_type)) =
        alt((parse_self_closing_html_tag_end, parse_opening_html_tag_end))(line)?;
    match tag_type {
        HTMLTagType::Opening | HTMLTagType::OpeningStart | HTMLTagType::SelfClosing => {
            Ok((remaining_line, (line.to_string(), attributes, tag_type, 0)))
        }
        HTMLTagType::Closing => Err(Err::Error(Error::new(line, ErrorKind::Tag))),
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

fn form_code_fragment_component_last_line(line: &str) -> IResult<&str, (String, LineType, usize)> {
    let component_identifier = "CodeFragment";
    let (final_segment, initial_segment) =
        parse_jsx_component_last_line(line, component_identifier)?;
    Ok((
        "",
        (
            format!("{initial_segment}{final_segment}"),
            LineType::CodeFragment,
            0,
        ),
    ))
}

fn form_how_to_component_first_line(line: &str) -> IResult<&str, (String, &str, LineType, usize)> {
    let (remaining_line, (markup, attributes, tag_type, indentation)) =
        form_jsx_component_first_line(line, "HowTo")?;
    match tag_type {
        HTMLTagType::Opening => Ok((
            remaining_line,
            (markup, attributes, LineType::HowToOpen, indentation),
        )),
        HTMLTagType::OpeningStart => Ok((
            "",
            (markup, attributes, LineType::HowToOpening, indentation),
        )),
        HTMLTagType::SelfClosing => Ok((
            remaining_line,
            (markup, attributes, LineType::HowTo, indentation),
        )),
        HTMLTagType::Closing => Err(Err::Error(Error::new(line, ErrorKind::Tag))),
    }
}

fn form_how_to_section_component_first_line(
    line: &str,
) -> IResult<&str, (String, &str, LineType, usize)> {
    let (remaining_line, (markup, attributes, tag_type, indentation)) =
        form_jsx_component_first_line(line, "HowToSection")?;
    match tag_type {
        HTMLTagType::Opening => Ok((
            remaining_line,
            (markup, attributes, LineType::HowToSectionOpen, indentation),
        )),
        HTMLTagType::OpeningStart => Ok((
            "",
            (
                markup,
                attributes,
                LineType::HowToSectionOpening,
                indentation,
            ),
        )),
        HTMLTagType::SelfClosing => Ok((
            remaining_line,
            (markup, attributes, LineType::HowToSection, indentation),
        )),
        HTMLTagType::Closing => Err(Err::Error(Error::new(line, ErrorKind::Tag))),
    }
}

fn form_how_to_step_component_first_line(
    line: &str,
) -> IResult<&str, (String, &str, LineType, usize)> {
    let (remaining_line, (markup, attributes, tag_type, indentation)) =
        form_jsx_component_first_line(line, "HowToStep")?;
    match tag_type {
        HTMLTagType::Opening => Ok((
            remaining_line,
            (markup, attributes, LineType::HowToStepOpen, indentation),
        )),
        HTMLTagType::OpeningStart => Ok((
            "",
            (markup, attributes, LineType::HowToStepOpening, indentation),
        )),
        HTMLTagType::SelfClosing => Ok((
            remaining_line,
            (markup, attributes, LineType::HowToStep, indentation),
        )),
        HTMLTagType::Closing => Err(Err::Error(Error::new(line, ErrorKind::Tag))),
    }
}

fn form_how_to_direction_component_first_line(
    line: &str,
) -> IResult<&str, (String, &str, LineType, usize)> {
    let (remaining_line, (markup, attributes, tag_type, indentation)) =
        form_jsx_component_first_line(line, "HowToDirection")?;
    match tag_type {
        HTMLTagType::Opening => Ok((
            remaining_line,
            (
                markup,
                attributes,
                LineType::HowToDirectionOpen,
                indentation,
            ),
        )),
        HTMLTagType::OpeningStart => Ok((
            "",
            (
                markup,
                attributes,
                LineType::HowToDirectionOpening,
                indentation,
            ),
        )),
        HTMLTagType::SelfClosing => Ok((
            remaining_line,
            (markup, attributes, LineType::HowToDirection, indentation),
        )),
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
pub fn form_how_to_component_opening_line(
    line: &str,
) -> IResult<&str, (String, &str, LineType, usize)> {
    let (remaining_line, (markup, atrributes, tag_type, indentation)) =
        form_jsx_component_opening_line(line)?;
    match tag_type {
        HTMLTagType::Opening | HTMLTagType::SelfClosing => Ok((
            remaining_line,
            (markup, atrributes, LineType::HowToOpen, indentation),
        )),
        _ => Ok((
            "",
            (
                String::from(line),
                atrributes,
                LineType::HowToOpening,
                indentation,
            ),
        )),
    }
}

pub fn form_how_to_section_component_opening_line(
    line: &str,
) -> IResult<&str, (String, &str, LineType, usize)> {
    let (remaining_line, (markup, atrributes, tag_type, indentation)) =
        form_jsx_component_opening_line(line)?;
    match tag_type {
        HTMLTagType::Opening | HTMLTagType::SelfClosing => Ok((
            remaining_line,
            (markup, atrributes, LineType::HowToSectionOpen, indentation),
        )),
        _ => Ok((
            "",
            (
                String::from(line),
                atrributes,
                LineType::HowToSectionOpening,
                indentation,
            ),
        )),
    }
}

pub fn form_how_to_step_component_opening_line(
    line: &str,
) -> IResult<&str, (String, &str, LineType, usize)> {
    let (remaining_line, (markup, atrributes, tag_type, indentation)) =
        form_jsx_component_opening_line(line)?;
    match tag_type {
        HTMLTagType::Opening | HTMLTagType::SelfClosing => Ok((
            remaining_line,
            (markup, atrributes, LineType::HowToStepOpen, indentation),
        )),
        _ => Ok((
            "",
            (
                String::from(line),
                atrributes,
                LineType::HowToStepOpening,
                indentation,
            ),
        )),
    }
}

pub fn form_how_to_direction_component_opening_line(
    line: &str,
) -> IResult<&str, (String, &str, LineType, usize)> {
    let (remaining_line, (markup, atrributes, tag_type, indentation)) =
        form_jsx_component_opening_line(line)?;
    match tag_type {
        HTMLTagType::Opening | HTMLTagType::SelfClosing => Ok((
            remaining_line,
            (
                markup,
                atrributes,
                LineType::HowToDirectionOpen,
                indentation,
            ),
        )),
        _ => Ok((
            "",
            (
                String::from(line),
                atrributes,
                LineType::HowToDirectionOpening,
                indentation,
            ),
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
    let (remaining_line, (markup, _attributes, tag_type, indentation)) =
        form_jsx_component_opening_line(line)?;
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

pub fn form_how_to_section_component_last_line(
    line: &str,
) -> IResult<&str, (String, LineType, usize)> {
    let (remaining_line, (markup, tag_type, indentation)) =
        form_jsx_component_last_line(line, "HowToSection")?;
    match tag_type {
        HTMLTagType::Closing => Ok((
            remaining_line,
            (markup, LineType::HowToSection, indentation),
        )),
        HTMLTagType::Opening | HTMLTagType::OpeningStart | HTMLTagType::SelfClosing => Ok((
            remaining_line,
            (markup, LineType::HowToSectionOpen, indentation),
        )),
    }
}

pub fn form_how_to_step_component_last_line(
    line: &str,
) -> IResult<&str, (String, LineType, usize)> {
    let (remaining_line, (markup, tag_type, indentation)) =
        form_jsx_component_last_line(line, "HowToStep")?;
    match tag_type {
        HTMLTagType::Closing => Ok((remaining_line, (markup, LineType::HowToStep, indentation))),
        HTMLTagType::Opening | HTMLTagType::OpeningStart | HTMLTagType::SelfClosing => Ok((
            remaining_line,
            (markup, LineType::HowToStepOpen, indentation),
        )),
    }
}

pub fn form_how_to_direction_component_last_line(
    line: &str,
) -> IResult<&str, (String, LineType, usize)> {
    let (remaining_line, (markup, tag_type, indentation)) =
        form_jsx_component_last_line(line, "HowToDirection")?;
    match tag_type {
        HTMLTagType::Closing => Ok((
            remaining_line,
            (markup, LineType::HowToDirection, indentation),
        )),
        HTMLTagType::Opening | HTMLTagType::OpeningStart | HTMLTagType::SelfClosing => Ok((
            remaining_line,
            (markup, LineType::HowToDirectionOpen, indentation),
        )),
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
    open_jsx_component_register: &mut JSXComponentRegister,
) -> Option<(String, LineType, usize)> {
    let open_jsx_component_type = open_jsx_component_register.peek();
    match open_jsx_component_type {
        Some(JSXComponentType::HowToOpening) => match form_how_to_component_opening_line(line) {
            Ok((_, (line, attributes, line_type, level))) => {
                if !line.is_empty() {
                    let (_, attributes_vector) = parse_html_tag_attributes(attributes)
                        .unwrap_or_else(|_| {
                            panic!("[ ERROR ] Unable to parse HowTo component props: {line}")
                        });
                    for (key, value) in attributes_vector {
                        open_jsx_component_register.insert_prop(key, value);
                    }
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
        Some(JSXComponentType::HowTo) => match form_how_to_section_component_first_line(line) {
            Ok((_, (line, attributes, line_type, level))) => {
                let (_, attributes_vector) =
                    parse_html_tag_attributes(attributes).unwrap_or_else(|_| {
                        panic!("[ ERROR ] Unable to parse HowToSection component props: {line}")
                    });
                for (key, value) in &attributes_vector {
                    open_jsx_component_register.insert_prop(key, value);
                }
                match attributes_vector
                    .iter()
                    .find(|&&(key, _value)| key == "name")
                {
                    Some((_, value)) => {
                        let position = open_jsx_component_register.add_how_to_section(value);
                        match line_type {
                            LineType::HowToSectionOpen => Some((
                                format!(
                                    "  <HowToSection name=\"{value}\" position={{{position}}}>"
                                ),
                                line_type,
                                level,
                            )),
                            LineType::HowToSectionOpening => Some((
                                format!("  <HowToSection name=\"{value}\" position={{{position}}}"),
                                line_type,
                                level,
                            )),
                            _ => Some((line, line_type, level)),
                        }
                    }
                    _ => Some((line, line_type, level)),
                }
            }
            Err(_) => match alt((
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
        },
        Some(JSXComponentType::HowToSectionOpening) => {
            match form_how_to_section_component_opening_line(line) {
                Ok((_, (line, attributes, line_type, level))) => {
                    let (_, attributes_vector) = parse_html_tag_attributes(attributes)
                        .unwrap_or_else(|_| {
                            panic!("[ ERROR ] Unable to parse HowToStep component props: {line}")
                        });
                    match attributes_vector
                        .iter()
                        .find(|&&(key, _value)| key == "name")
                    {
                        Some((_, value)) => {
                            let position = open_jsx_component_register.add_how_to_section(value);
                            match line_type {
                                LineType::HowToSectionOpen => Some((
                                    format!("    name=\"{value}\" position={{{position}}}>"),
                                    line_type,
                                    level,
                                )),
                                LineType::HowToSectionOpening => Some((
                                    format!("    name=\"{value}\" position={{{position}}}"),
                                    line_type,
                                    level,
                                )),
                                _ => Some((line, line_type, level)),
                            }
                        }
                        _ => Some((line, line_type, level)),
                    }
                }
                Err(_) => Some((line.to_string(), LineType::HowToSectionOpening, 0)),
            }
        }
        Some(JSXComponentType::HowToSection) => match form_how_to_step_component_first_line(line) {
            Ok((_, (line, attributes, line_type, level))) => {
                let (_, attributes_vector) =
                    parse_html_tag_attributes(attributes).unwrap_or_else(|_| {
                        panic!("[ ERROR ] Unable to parse HowToStep component props: {line}")
                    });
                let (section_position, position) = open_jsx_component_register.add_how_to_step();
                let mut attributes_markup_vector: Vec<String> = Vec::new();
                attributes_markup_vector.push(format!(
                    "{{slug}} position={{{position}}} section={{{section_position}}}"
                ));
                for (key, value) in attributes_vector {
                    match key {
                        "name" => {
                            open_jsx_component_register.add_how_to_step_name(value);
                            attributes_markup_vector.push(format!("name=\"{value}\""));
                        }
                        "image" => {
                            open_jsx_component_register.add_how_to_step_image(value);
                            attributes_markup_vector.push(format!("image=\"{value}\""));
                        }
                        "video" => {
                            open_jsx_component_register.add_how_to_step_video(value);
                            attributes_markup_vector.push(format!("video=\"{value}\""));
                        }
                        "start" => {
                            open_jsx_component_register.add_how_to_step_start(value);
                            attributes_markup_vector.push(format!("start={{{value}}}"));
                        }
                        "end" => {
                            open_jsx_component_register.add_how_to_step_end(value);
                            attributes_markup_vector.push(format!("end={{{value}}}"));
                        }
                        &_ => {}
                    }
                }
                let attributes_markup = attributes_markup_vector.join(" ");
                match line_type {
                    LineType::HowToStepOpen => Some((
                        format!("    <HowToStep {attributes_markup}>"),
                        line_type,
                        level,
                    )),
                    LineType::HowToStepOpening => Some((
                        format!("    <HowToStep {attributes_markup}"),
                        line_type,
                        level,
                    )),
                    _ => Some((line, line_type, level)),
                }
            }
            Err(_) => match alt((
                form_fenced_code_block_first_line,
                form_video_component_first_line,
                form_how_to_section_component_last_line,
            ))(line)
            {
                Ok((_, (line, line_type, level))) => {
                    if !line.is_empty() {
                        Some((line, line_type, level))
                    } else {
                        None
                    }
                }
                Err(_) => Some((line.to_string(), LineType::HowToSectionOpen, 0)),
            },
        },
        Some(JSXComponentType::HowToStepOpening) => {
            match form_how_to_step_component_opening_line(line) {
                Ok((_, (line, attributes, line_type, level))) => {
                    let (_, attributes_vector) = parse_html_tag_attributes(attributes)
                        .unwrap_or_else(|_| {
                            panic!(
                                "[ ERROR ] Unable to parse HowToDirection component props: {line}"
                            )
                        });
                    let mut attributes_markup_vector: Vec<String> = Vec::new();
                    for (key, value) in attributes_vector {
                        match key {
                            "name" => {
                                open_jsx_component_register.add_how_to_step_name(value);
                                attributes_markup_vector.push(format!("name=\"{value}\""));
                            }
                            "image" => {
                                open_jsx_component_register.add_how_to_step_image(value);
                                attributes_markup_vector.push(format!("image=\"{value}\""));
                            }
                            "video" => {
                                open_jsx_component_register.add_how_to_step_video(value);
                                attributes_markup_vector.push(format!("video=\"{value}\""));
                            }
                            "start" => {
                                open_jsx_component_register.add_how_to_step_start(value);
                                attributes_markup_vector.push(format!("start={{{value}}}"));
                            }
                            "end" => {
                                open_jsx_component_register.add_how_to_step_end(value);
                                attributes_markup_vector.push(format!("end={{{value}}}"));
                            }
                            &_ => {}
                        }
                    }
                    let attributes_markup = attributes_markup_vector.join(" ");
                    match line_type {
                        LineType::HowToStepOpen => {
                            Some((format!("    {attributes_markup}>"), line_type, level))
                        }
                        LineType::HowToStepOpening => {
                            Some((format!("    {attributes_markup}"), line_type, level))
                        }
                        _ => Some((line, line_type, level)),
                    }
                }
                Err(_) => Some((line.to_string(), LineType::HowToStepOpening, 0)),
            }
        }
        Some(JSXComponentType::HowToStep) => match form_how_to_direction_component_first_line(line)
        {
            Ok((_, (line, attributes, line_type, level))) => {
                let (_, attributes_vector) =
                    parse_html_tag_attributes(attributes).unwrap_or_else(|_| {
                        panic!("[ ERROR ] Unable to parse HowToDirection component props: {line}")
                    });
                match attributes_vector
                    .iter()
                    .find(|&&(key, _value)| key == "text")
                {
                    Some((_, value)) => {
                        let position = open_jsx_component_register.add_how_to_direction(value);
                        match line_type {
                            LineType::HowToDirectionOpen => Some((
                                format!(
                                    "      <HowToDirection text=\"{value}\" position={{{position}}}>"
                                ),
                                line_type,
                                level,
                            )),
                            LineType::HowToDirectionOpening => Some((
                                format!("      <HowToDirection text=\"{value}\" position={{{position}}}"),
                                line_type,
                                level,
                            )),
                            _ => Some((line, line_type, level)),
                        }
                    }
                    _ => Some((line, line_type, level)),
                }
            }
            Err(_) => match alt((
                form_fenced_code_block_first_line,
                form_video_component_first_line,
                form_how_to_step_component_last_line,
            ))(line)
            {
                Ok((_, (line, line_type, level))) => {
                    if !line.is_empty() {
                        Some((line, line_type, level))
                    } else {
                        None
                    }
                }
                Err(_) => Some((line.to_string(), LineType::HowToStepOpen, 0)),
            },
        },
        Some(JSXComponentType::HowToDirectionOpening) => {
            match form_how_to_direction_component_opening_line(line) {
                Ok((_, (line, attributes, line_type, level))) => {
                    let (_, attributes_vector) = parse_html_tag_attributes(attributes)
                        .unwrap_or_else(|_| {
                            panic!(
                                "[ ERROR ] Unable to parse HowToDirection component props: {line}"
                            )
                        });
                    match attributes_vector
                        .iter()
                        .find(|&&(key, _value)| key == "text")
                    {
                        Some((_, value)) => {
                            let position = open_jsx_component_register.add_how_to_direction(value);
                            match line_type {
                                LineType::HowToDirectionOpen => Some((
                                    format!("text=\"{value}\" position=\"{position}\">"),
                                    line_type,
                                    level,
                                )),
                                LineType::HowToDirectionOpening => Some((
                                    format!("text=\"{value}\" position=\"{position}\""),
                                    line_type,
                                    level,
                                )),
                                _ => Some((line, line_type, level)),
                            }
                        }
                        _ => Some((line, line_type, level)),
                    }
                }
                Err(_) => Some((line.to_string(), LineType::HowToDirectionOpening, 0)),
            }
        }
        Some(JSXComponentType::HowToDirection) => {
            match form_how_to_direction_component_last_line(line) {
                Ok((_, (line, line_type, level))) => Some((line, line_type, level)),
                Err(_) => match alt((
                    form_fenced_code_block_first_line,
                    form_video_component_first_line,
                ))(line)
                {
                    Ok((_, (line, line_type, level))) => {
                        if !line.is_empty() {
                            Some((line, line_type, level))
                        } else {
                            None
                        }
                    }
                    Err(_) => Some((line.to_string(), LineType::HowToDirectionOpen, 0)),
                },
            }
        }
        Some(_) => {
            match alt((
                form_code_fragment_component_last_line,
                form_poll_component_last_line,
                form_video_component_last_line,
                form_how_to_step_component_last_line,
                form_how_to_section_component_last_line,
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
                Err(_) => Some((line.to_string(), LineType::JSXComponent, 0)),
            }
        }
        None => match form_how_to_component_first_line(line) {
            Ok((_, (line, attributes, line_type, level))) => {
                if !line.is_empty() {
                    let (_, attributes_vector) = parse_html_tag_attributes(attributes)
                        .unwrap_or_else(|_| {
                            panic!("[ ERROR ] Unable to parse HowTo component props: {line}")
                        });
                    let how_to = open_jsx_component_register.how_to_mut();
                    match how_to {
                        Some(how_to_value) => {
                            for (key, value) in attributes_vector {
                                how_to_value.insert_prop(key, value);
                            }
                        }
                        None => {}
                    };

                    Some((line, line_type, level))
                } else {
                    None
                }
            }
            Err(_) => None,
        },
    }
}
