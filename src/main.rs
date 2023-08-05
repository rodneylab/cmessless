mod parser;
mod utility;

use clap::Parser;
use is_terminal::IsTerminal;
use std::{
    fs,
    io::{self, BufRead},
    path::{Path, PathBuf},
};
use watchexec::{
    config::{Config, ConfigBuilder},
    error::Result,
    pathop::PathOp,
    run::{watch, ExecHandler, Handler},
};

use parser::{author_name_from_cargo_pkg_authors, parse_mdx_file};

#[derive(Parser)]
#[clap(author,version,about,long_about=None)]
struct Cli {
    path: Vec<PathBuf>,

    #[clap(short, long)]
    check: bool,

    #[clap(short, long)]
    modified: bool,

    #[clap(short = 'R', long)]
    relative: bool, // path should only contain UTF-8 characters

    #[clap(short, long)]
    verbose: bool,

    #[clap(short = 'V', long)]
    version: bool,

    #[clap(short, long)]
    watch: bool,

    #[clap(value_parser)]
    #[clap(short, long)]
    output: std::path::PathBuf,
}

fn get_title() -> String {
    let mut the_title = String::from(env!("CARGO_PKG_NAME"));
    the_title.push_str(" (v");
    the_title.push_str(env!("CARGO_PKG_VERSION"));
    the_title.push_str("), ");
    the_title.push_str(env!("CARGO_PKG_DESCRIPTION"));
    the_title
}

fn print_short_banner() {
    println!("{}", get_title());
}

fn print_long_banner() {
    print_short_banner();
    println!(
        "Written by: {}",
        author_name_from_cargo_pkg_authors().trim()
    );
    println!("Repo: {}", env!("CARGO_PKG_REPOSITORY"));
    println!("Usage: {} <somefile>.mdx", env!("CARGO_PKG_NAME"));
    println!("       {} --watch <somefile>.mdx", env!("CARGO_PKG_NAME"));
}

struct CmslessHandler(ExecHandler);

impl Handler for CmslessHandler {
    fn args(&self) -> Config {
        self.0.args()
    }

    fn on_manual(&self) -> Result<bool> {
        println!("[ INFO ] Running manually...");
        self.0.on_manual()
    }

    fn on_update(&self, ops: &[PathOp]) -> Result<bool> {
        println!("[ INFO ] Running manually {:?}...", ops);
        self.0.on_update(ops)
    }
}

fn parse_then_watch(mdx_paths: &[PathBuf], output_path: &Path, verbose: bool) -> Result<()> {
    let output_path_str = output_path.to_string_lossy();
    let mut command: Vec<String> = vec!["cmessless".into()];
    command.extend(mdx_paths.iter().map(|value| value.to_string_lossy().into()));
    command.push("--check --output".into());
    command.push(output_path.to_string_lossy().into());
    command.push("| cmessless --relative".into());
    if verbose {
        command.push("--verbose".into());
    }
    command.push(" --output".into());
    command.push(output_path_str.into());

    let config = ConfigBuilder::default()
        .clear_screen(true)
        .run_initially(true)
        .paths(mdx_paths)
        .cmd(command)
        .build()
        .expect("[ ERROR ] Issue while configuring watchexec");

    let handler = CmslessHandler(
        ExecHandler::new(config).expect("[ ERROR ] Issue while creating watchexec handler"),
    );
    watch(&handler)
}

//     let config = ConfigBuilder::default()
//         .clear_screen(true)
//         .run_initially(true)
//         .paths(mdx_paths)
//         .cmd(command)
//         .build()
//         .expect("[ ERROR ] Issue while configuring watchexec");

//     let handler = CmslessHandler(
//         ExecHandler::new(config).expect("[ ERROR ] Issue while creating watchexec handler"),
//     );
//     watch(&handler)
// }

fn relative_output_path_from_input(input_path: &Path, relative_output_path: &Path) -> PathBuf {
    match input_path.to_string_lossy().find("/./") {
        Some(_) => {}
        None => panic!(
            "[ ERROR ] Using relative mode: check input paths include a \"/./\" marker to separate \
        root and relative parts."
        ),
    }

    let mut components = input_path.components();
    loop {
        if components.as_path().to_string_lossy().find("/./").is_none() {
            break;
        }
        components.next();
    }

    let mut result = PathBuf::new();
    result.push(relative_output_path);
    let tail = components.as_path();
    let mut output_file_name = String::from(tail.file_stem().unwrap().to_string_lossy());
    output_file_name.push_str(".astro");
    relative_output_path
        .join(tail.parent().unwrap())
        .join(PathBuf::from(output_file_name))
}

fn check_modified_files(mdx_paths: &[PathBuf], relative_output_path: &Path) {
    let mut modified_files: Vec<String> = Vec::new();
    for input_path in mdx_paths {
        let output_path =
            relative_output_path_from_input(input_path.as_path(), relative_output_path);
        let input_modified = match fs::metadata(input_path).unwrap().modified() {
            Ok(value) => Some(value),
            Err(_) => None,
        };
        let output_modified = match fs::metadata(output_path) {
            Ok(metadata_value) => match metadata_value.modified() {
                Ok(value) => Some(value),
                Err(_) => None,
            },
            Err(_) => None,
        };
        if output_modified.is_none()
            || input_modified.is_none()
            || input_modified.unwrap() > output_modified.unwrap()
        {
            modified_files.push(String::from(input_path.to_string_lossy()));
        }
    }
    println!("{}", modified_files.join(" "));
}

fn parse_multiple_files(mdx_paths: &[PathBuf], relative_output_path: &Path, verbose: bool) {
    for input_path in mdx_paths {
        let output_path =
            relative_output_path_from_input(input_path.as_path(), relative_output_path);
        parse_mdx_file(input_path.as_path(), output_path.as_path(), verbose);
    }
    println!("\n[ INFO ] {} files parsed.", mdx_paths.len());
}

fn get_piped_input() -> Vec<PathBuf> {
    let mut buffer = String::new();
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    handle.read_line(&mut buffer).unwrap_or(0);
    let result = buffer[..buffer.len() - 1]
        .split(' ')
        .map(PathBuf::from)
        .collect();
    result
}

// #[tokio::main]
fn main() -> Result<()> {
    let cli = &Cli::parse();
    if cli.check {
        check_modified_files(&cli.path, &cli.output);
        return Ok(());
    }

    let inputs = if io::stdin().is_terminal() {
        cli.path.to_vec()
    } else {
        get_piped_input()
    };
    if inputs.is_empty() {
        return Ok(());
    }

    if cli.verbose {
        print_long_banner();
    } else {
        print_short_banner();
    }

    if cli.version {
        println!("{}", get_title());
        return Ok(());
    }

    if cli.watch {
        return parse_then_watch(&inputs, cli.output.as_path(), cli.verbose);
    } else if cli.path.len() > 1 {
        if !cli.relative {
            println!(
            "\n[ ERROR ] for multiple inputs, use the --relative flag to set a relative output path."
            );
        }
        parse_multiple_files(&inputs, &cli.output, cli.verbose);
    } else if cli.relative {
        let output_path = relative_output_path_from_input(inputs[0].as_path(), &cli.output);
        parse_mdx_file(inputs[0].as_path(), &output_path, cli.verbose);
    } else {
        parse_mdx_file(inputs[0].as_path(), cli.output.as_path(), cli.verbose);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::relative_output_path_from_input;
    use std::path::PathBuf;

    #[test]
    pub fn test_relative_output_path_from_input() {
        let input_path = PathBuf::from("local/files/input/./day-one/morning.txt");
        let relative_output_path = PathBuf::from("local/files/output");
        assert_eq!(
            relative_output_path_from_input(input_path.as_path(), relative_output_path.as_path()),
            PathBuf::from("local/files/output/day-one/morning.astro")
        );
    }

    #[test]
    #[should_panic(
        expected = "[ ERROR ] Using relative mode: check input paths include a \"/./\" \
        marker to separate root and relative parts."
    )]
    pub fn test_relative_output_path_from_input_panic() {
        let input_path = PathBuf::from("local/files/input/day-one/morning.mdx");
        let relative_output_path = PathBuf::from("local/files/output");
        assert_eq!(
            relative_output_path_from_input(input_path.as_path(), relative_output_path.as_path()),
            PathBuf::from("local/files/output/day-one/morning.astro")
        );
    }
}
