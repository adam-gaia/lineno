use anyhow::Result;
use clap::Parser;
use lineno::{filter, Filters};
use log::debug;
use std::fs::File;
use std::io::stdin;
use std::io::BufReader;
use std::path::PathBuf;

#[derive(Debug, Parser)]
struct Cli {
    /// File to filter
    #[clap(short, long)]
    file: Option<PathBuf>,

    /// TODO
    lines: Vec<Filters>,
}

fn main() -> Result<()> {
    env_logger::init();
    let args = Cli::parse();
    let filters = args.lines;
    debug!("Filters: {:?}", filters);

    let lines = match args.file {
        Some(path) => {
            let f = File::open(path)?;
            let reader = BufReader::new(f);
            filter(filters, reader)?
        }
        None => {
            let stdin = stdin().lock();
            filter(filters, stdin)?
        }
    };

    for line in lines {
        println!("{}", line);
    }

    Ok(())
}
