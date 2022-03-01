mod parser;

use ::watchexec::{
    config::{Config, ConfigBuilder},
    error::Result,
    pathop::PathOp,
    run::{watch, ExecHandler, Handler},
};
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
    println!("       {} --watch <somefile>.mdx", env!("CARGO_PKG_NAME"));
}

fn usage() {
    print_long_banner();
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

fn parse_then_watch(mdx_path: &str) -> Result<()> {
    let config = ConfigBuilder::default()
        .clear_screen(true)
        .run_initially(true)
        .paths(vec![mdx_path.into()])
        .cmd(vec!["./target/release/cmessless".into(), mdx_path.into()])
        .build()
        .expect("[ ERROR ] Issue while configuring watchexec");

    let handler = CmslessHandler(
        ExecHandler::new(config).expect("[ ERROR ] Issue while creating watchexec handler"),
    );
    watch(&handler)
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    match args.len() {
        2 => {
            print_short_banner();
            parse_mdx_file(&args[1]);
            Ok(())
        }
        3 => {
            if args[1] == "--watch" {
                print_short_banner();
                return parse_then_watch(&args[2]);
            } else {
            }
            println!("[ ERROR ] Invalid invocation (not at all sure what you want)");
            usage();
            Ok(())
        }
        _ => {
            println!("[ ERROR ] Invalid invocation (not at all sure what you want)");
            usage();
            Ok(())
        }
    }
}
