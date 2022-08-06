use std::borrow::Borrow;

use anyhow::Result;
use clap::Parser;

#[derive(Debug, Parser)]
struct Args {
    #[clap(short, long, required = true)]
    input: String,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let om_data = std::fs::read_to_string(&args.input)?;

    let families = om_nomnomnom::parse(&om_data)?;

    for (name, family) in families.iter() {
        println!("{} ({:?})", name, family.metric_type);
        if let Some(help) = family.help.borrow() {
            println!("{}", help);
        }

        for sample in family.samples.iter() {
            println!("  {:?}", sample);
        }
        println!("");
    }

    Ok(())
}
