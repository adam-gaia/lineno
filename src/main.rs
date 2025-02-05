use anyhow::Result;
use clap::Parser;
use lineno::Filters;
use log::debug;
use std::fs::File;
use std::io::stdin;
use std::io::BufReader;
use std::path::PathBuf;

#[derive(Debug, Parser)]
struct Cli {
    /// TODO
    lines: Filters,

    /// File to filter
    file: Option<PathBuf>,
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
            filters.filter(reader)?
        }
        None => {
            let stdin = stdin().lock();
            filters.filter(stdin)?
        }
    };

    for line in lines {
        println!("{}", line);
    }

    Ok(())
}
