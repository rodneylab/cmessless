mod parser;

use clap::Parser;
use std::path::Path;
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
    path: std::path::PathBuf,

    #[clap(parse(from_os_str))]
    #[clap(short = 'o', long = "output")]
    output: std::path::PathBuf,

    #[clap(short, long)]
    verbose: bool,

    #[clap(short, long)]
    watch: bool,
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

fn parse_then_watch(mdx_path: &Path, output_path: &Path, verbose: bool) -> Result<()> {
    let command = if verbose {
        format!(
            "cmessless {} --verbose --output {}",
            mdx_path.to_string_lossy(),
            output_path.to_string_lossy()
        )
    } else {
        format!(
            "cmessless {} --output {}",
            mdx_path.to_string_lossy(),
            output_path.to_string_lossy()
        )
    };
    let config = ConfigBuilder::default()
        .clear_screen(true)
        .run_initially(true)
        .paths(vec![mdx_path.into()])
        .cmd(vec![command])
        .build()
        .expect("[ ERROR ] Issue while configuring watchexec");

    let handler = CmslessHandler(
        ExecHandler::new(config).expect("[ ERROR ] Issue while creating watchexec handler"),
    );
    watch(&handler)
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.verbose {
        print_long_banner();
    } else {
        print_short_banner();
    }

    if cli.watch {
        return parse_then_watch(cli.path.as_path(), cli.output.as_path(), cli.verbose);
    } else {
        parse_mdx_file(cli.path.as_path(), cli.output.as_path(), cli.verbose);
    }
    Ok(())
}
