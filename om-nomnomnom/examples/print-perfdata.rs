use anyhow::{anyhow, Result};
use indoc::indoc;
use om_nomnomnom::parser::{MetricFamily, MetricType};

trait ToPerfdata {
    fn to_perfdata(&self) -> Result<String>;
}

impl<'a> ToPerfdata for MetricFamily<'a> {
    fn to_perfdata(&self) -> Result<String> {
        if self.metric_type != MetricType::Gauge {
            Err(anyhow!("ToPerdata only supports Gauge types"))?
        }

        let unit = self.unit.ok_or(anyhow!("no unit"))?;
        let measurement = self.samples.first().ok_or(anyhow!("no samples?"))?;

        Ok(format!(
            "{} UNKNOWN: {} | '{}'={}{}",
            measurement.name,
            self.help.as_ref().unwrap(),
            measurement.name,
            measurement.number,
            unit
        ))
    }
}

fn main() -> Result<()> {
    let om_data = indoc! {r#"
        # TYPE a_seconds gauge
        # UNIT a_seconds seconds
        # HELP a_seconds the number of seconds in a
        a_seconds 1
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

    let (_family_name, gauge) = families.iter().next().ok_or(anyhow!("empty exposition?"))?;

    println!("{}", gauge.to_perfdata()?);

    Ok(())
}
