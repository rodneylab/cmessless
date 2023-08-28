mod parser;
mod utility;

use clap::Parser;
use notify::RecursiveMode;
use notify_debouncer_mini::{new_debouncer, DebouncedEvent};
use std::{
    collections::HashMap,
    fs,
    io::{self, BufRead, IsTerminal, Write},
    path::{Path, PathBuf},
    time::Duration,
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

/***
 * watch a single file for changes and parse to output_path, when changes occur
 */
async fn debounce_watch<P1: AsRef<Path>, P2: AsRef<Path>>(
    mdx_path: &P1,
    output_path: &P2,
    verbose: bool,
) {
    let (tx, rx) = std::sync::mpsc::channel();

    let mut debouncer = new_debouncer(Duration::from_millis(250), tx).unwrap();

    debouncer
        .watcher()
        .watch(mdx_path.as_ref(), RecursiveMode::NonRecursive)
        .unwrap();

    for events in rx {
        match events {
            // could add a check to make sure the paths match
            Ok(_) => {
                parse_mdx_file(&mdx_path, output_path, verbose);
            }
            Err(e) => eprintln!("Something went wrong: {:?}", e),
        }
    }
}

/***
 * deduce the directory to watch from an input file path which contains a '/./' pattern
 */
fn watch_directory_from_relative_input_path<P: AsRef<Path>>(input_path: &P) -> PathBuf {
    match input_path.as_ref().to_str() {
        Some(value) => match value.split_once("/./") {
            Some((path_root_value, _)) => PathBuf::from(path_root_value),
            None => panic!("Expected relative path with a '/./' pattern"),
        },
        None => panic!(
            "Only valid UTF-8 paths are supported, for now.  Got path {}",
            input_path.as_ref().to_string_lossy()
        ),
    }
}

/***
 * Given a relative input path and the output root directory, return the full absolute output path
 */
fn output_path_from_relative_input<P1: AsRef<Path>, P2: AsRef<Path>>(
    output_root_directory: &P1,
    relative_input_path: &P2,
) -> PathBuf {
    let input_path_tail = match relative_input_path.as_ref().to_str() {
        Some(value) => match value.rsplit_once("/./") {
            Some((_, result_value)) => PathBuf::from(result_value),
            None => panic!(
                "[ ERROR ] Using relative mode: check input paths include a \"/./\" marker to separate root and relative parts."
            ),
        },
        None => panic!(
                "[ ERROR ] Using relative mode: check input paths include a \"/./\" marker to separate root and relative parts."
        ),
    };
    match input_path_tail.file_stem() {
        Some(value) => match value.to_str() {
            Some(stem_value) => match input_path_tail.parent() {
                Some(parent_value) => output_root_directory
                    .as_ref()
                    .join(parent_value)
                    .join(format!("{stem_value}.astro")),
                None => output_root_directory
                    .as_ref()
                    .join(format!("{stem_value}.astro")),
            },
            None => panic!(
                "Expected input filename composed of valid UTF-8 characters, got: {}",
                value.to_string_lossy()
            ),
        },
        None => panic!(
            "Expected input path to have an extension, but got: {}",
            relative_input_path.as_ref().display()
        ),
    }
}

/**
 * watch multiple input paths for changes, input paths need to contain a '/./'
 * pattern to mark the relative part of the path.  To get the output path, we place the relative
 * part (after the '/./') on the end of the output_path_root, passed in. Output will have a .astro
 * extension.  Multiple paths may be passed in, but perhaps only one or two input files eveer get
 * updated, so to save working out the outpaths for unused paths, a hash map caches the values.
 */
async fn debounce_watch_multiple<P1: AsRef<Path>, P2: AsRef<Path>>(
    mdx_paths: &[P1],
    output_path_root: &P2,
    verbose: bool,
) {
    let (tx, rx) = std::sync::mpsc::channel();

    let mut debouncer = new_debouncer(Duration::from_millis(250), tx).unwrap();

    let watch_directory = watch_directory_from_relative_input_path(&mdx_paths[0]);
    debouncer
        .watcher()
        .watch(watch_directory.as_ref(), RecursiveMode::Recursive)
        .unwrap();

    let canonicalized_paths: &Vec<PathBuf> = &mdx_paths
        .iter()
        .map(|val| val.as_ref().canonicalize().unwrap())
        .collect::<Vec<PathBuf>>();

    // hash map to save determining the output path for any input more than once
    let mut output_paths_map: HashMap<String, PathBuf> = HashMap::new();

    for events in rx {
        match events {
            Ok(event) => {
                for individual_event in event.iter() {
                    let DebouncedEvent { path, .. } = individual_event;

                    if let Some((index, _)) = canonicalized_paths
                        .iter()
                        .enumerate()
                        .find(|(_, val)| val == &path)
                    {
                        let path_as_string = path.to_str().unwrap();
                        match output_paths_map.get(path_as_string) {
                            Some(value) => parse_mdx_file(path, &value, verbose),
                            None => {
                                let output_path_result = output_path_from_relative_input(
                                    &output_path_root,
                                    &mdx_paths[index],
                                );
                                parse_mdx_file(path, &output_path_result, verbose);
                                output_paths_map
                                    .insert((&path_as_string).to_string(), output_path_result);
                            }
                        };
                    };
                }
            }
            Err(e) => eprintln!("Something went wrong: {:?}", e),
        }
    }
}

/** Check if an input file has been modified since its output was created. Return true when not able
 * to detemine modified time of either file
 */
fn check_file_modified<P1: AsRef<Path>, P2: AsRef<Path>>(
    input_path: &P1,
    output_path: &P2,
) -> bool {
    let input_modified = match fs::metadata(input_path) {
        Ok(value) => match value.modified() {
            Ok(modified_value) => modified_value,
            Err(_) => return true,
        },
        Err(_) => return true,
    };
    let output_modified = match fs::metadata(output_path) {
        Ok(value) => match value.modified() {
            Ok(modified_value) => modified_value,
            Err(_) => return true,
        },
        Err(_) => return true,
    };
    input_modified > output_modified
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = &Cli::parse();

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

    if cli.path.len() > 1 && !cli.relative {
        println!(
            "\n[ ERROR ] for multiple inputs, use the --relative flag to set a relative output path."
            );
        return Ok(());
    }

    if cli.check {
        if cli.path.len() == 1 && !cli.relative {
            if check_file_modified(&inputs[0], &&cli.output) {
                println!("{}", inputs[0].display());
            }
        } else {
            let stdout = io::stdout();
            let mut stdout_handle = io::BufWriter::new(stdout);
            inputs.iter().for_each(|val| {
                let absolute_output_path = output_path_from_relative_input(&cli.output, val);
                if check_file_modified(val, &absolute_output_path) {
                    writeln!(stdout_handle, "{}", val.display())
                        .expect("Unable to write to stdout");
                }
            });
            stdout_handle.flush().expect("Unable to write to stdout");
        }
        return Ok(());
    }

    if cli.watch {
        if cli.path.len() == 1 && !cli.relative {
            debounce_watch(&inputs[0], &cli.output, cli.verbose).await;
        } else {
            debounce_watch_multiple(&inputs, &cli.output, cli.verbose).await;
        }
        return Ok(());
    }

    if cli.relative {
        inputs.iter().for_each(|val| {
            let absolute_output_path = output_path_from_relative_input(&cli.output, val);
            parse_mdx_file(val, &absolute_output_path, cli.verbose);
        })
    } else {
        parse_mdx_file(&inputs[0], &cli.output, cli.verbose);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::output_path_from_relative_input;
    use std::path::PathBuf;

    #[test]
    pub fn test_output_path_from_relative_input() {
        let input_path = PathBuf::from("local/files/input/./day-one/morning.txt");
        let relative_output_path = PathBuf::from("local/files/output");
        assert_eq!(
            output_path_from_relative_input(&relative_output_path, &input_path),
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
            output_path_from_relative_input(&relative_output_path, &input_path),
            PathBuf::from("local/files/output/day-one/morning.astro")
        );
    }
}
