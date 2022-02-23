mod parser;

use parser::{author_name_from_cargo_pkg_authors, parse_mdx_file};

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
}

fn usage() {
    print_long_banner();
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.len() {
        2 => {
            print_short_banner();
            parse_mdx_file(&args[1])
        }
        _ => {
            println!("[ ERROR ] Invalid invocation (not at all sure what you want)");
            usage()
        }
    }
}
