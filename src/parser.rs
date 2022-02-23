use nom::bytes::complete::take_until;
use nom::{
    bytes::complete::tag, character::complete::multispace1, multi::many1_count,
    sequence::terminated, IResult,
};
use std::{
    fs::File,
    io::{BufRead, BufReader, Write},
    path::Path,
};

fn parse_author_name_from_cargo_pkg_authors(cargo_pkg_authors: &str) -> IResult<&str, &str> {
    take_until(" <")(cargo_pkg_authors)
}

pub fn author_name_from_cargo_pkg_authors() -> &'static str {
    match parse_author_name_from_cargo_pkg_authors(env!("CARGO_PKG_AUTHORS")) {
        Ok((_, result)) => result,
        Err(_) => panic!("[ ERROR ] Authors does not seem to be defined!"),
    }
}

fn parse_heading_text(line: &str) -> IResult<&str, usize> {
    let (heading, level) = terminated(many1_count(tag("#")), multispace1)(line)?;
    Ok((heading, level))
}

fn parse_mdx_line(line: &str) -> Option<String> {
    match parse_heading_text(line) {
        Ok((value, level)) => Some(format!("<h{level}>{value}</h{level}>")),
        Err(_) => {
            if !line.is_empty() {
                Some(format!("<p>{line}</p>"))
            } else {
                None
            }
        }
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

#[cfg(test)]
mod tests {
    use crate::parser::{parse_heading_text, parse_mdx_line};

    #[test]
    pub fn test_parse_mdx_line() {
        let mdx_line = "# Getting Started with NewTech  ";
        assert_eq!(
            parse_mdx_line(mdx_line),
            Some(String::from("<h1>Getting Started with NewTech  </h1>"))
        );

        let mdx_line = "### What Does All This Mean?";
        assert_eq!(
            parse_mdx_line(mdx_line),
            Some(String::from("<h3>What Does All This Mean?</h3>"))
        );

        let mdx_line = "NewTech was first set up to solve the common problem coming up for identifiers in computer science.";
        assert_eq!(
            parse_mdx_line(mdx_line),
            Some(String::from("<p>NewTech was first set up to solve the common problem coming up for identifiers in computer science.</p>"))
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
}
