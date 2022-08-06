use anyhow::{anyhow, Result};
use indoc::indoc;
use itertools::Itertools;
use om_nomnomnom::parser::{MetricType, SampleKind};

const COLORS: &[&'static str] = &[
    "\u{2591}", "\u{2592}", "\u{2593}",
    "\u{25A3}", "\u{25A9}", "\u{25A4}"
];

fn main() -> Result<()> {
    let om_data = indoc! {r#"
        # TYPE a histogram
        # HELP a help
        a_bucket{le="0.5"} 5
        a_bucket{le="1.0"} 7
        a_bucket{le="+Inf"} 15
        a_count 15
        a_sum 2
        # EOF
    "#};

    let mut args = std::env::args();

    let progname = args.next().ok_or(anyhow!("ARGV[0] was not set??"))?;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" => {
                println!("Usage: {} [--print-exposition]", progname);
                return Ok(());
            }
            "--print-exposition" => {
                println!("Exposition:\n\n{}", om_data);
            }
            arg => return Err(anyhow!("Unknown argument: {}", arg)),
        }
    }

    let families = om_nomnomnom::parse(&om_data)?;

    let (family_name, histogram) = families.iter().next().ok_or(anyhow!("empty exposition?"))?;

    assert_eq!(MetricType::Histogram, histogram.metric_type);

    let sample_count = histogram
        .samples
        .iter()
        .filter(|sample| sample.name.ends_with("_count"))
        .next()
        .ok_or(anyhow!("no _count?"))?
        .number
        .round() as usize;

    let buckets = histogram
        .samples
        .iter()
        .filter(|sample| matches! {sample.kind, SampleKind::HistogramBucket(_)})
        .collect_vec();

    let factor = match sample_count {
        sample_count if sample_count < 25 => 2,
        _ => 1,
    };

    print!("Distribution of «{}»: ", family_name);
    buckets.iter().zip(COLORS).fold(0, |acc, (bucket, color)| {
        let cur_length = bucket.number.round() as usize;
        print!("{}", color.repeat((cur_length - acc) * factor));
        cur_length
    });
    print!("\t");

    println!(
        "[ {}]",
        buckets
            .iter()
            .zip(COLORS.iter().cycle())
            .map(|(bucket, color)| format!("{} ≤ {} ", color, bucket.labels["le"]))
            .join(" ")
    );

    Ok(())
}
