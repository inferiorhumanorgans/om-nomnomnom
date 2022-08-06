# om-nomnomnom

`om-nomnomnom` is a parser for the OpenMetrics (text) exposition format written using the `nom` parsing framework.  This crate is designed to ingest OpenMetrics data that an exporter might generate.  If you're looking to instrument your Rust application, this is the wrong crate.

```toml
[dependencies]
om-nomnomnom = "0.1"
```

*Compiler support: requires nightly*

## Usage

Usage is fairly simple:

```rust
    let om_data : &str = indoc! {"
        # TYPE a gauge
        # HELP a help
        a 1
        # EOF
    "};

    let families = om_nomnomnom::parse(om_data)?;

    for (name, family) in families.iter() {
        println!("{} ({:?})", name, family.metric_type);
    }
```

To iterate over the samples and pull out all the values:

```rust
    let family = families
        .values()
        .first()
        .expect("Empty exposition?");
    let values = family
        .samples
        .iter()
        .map(|sample| sample.number)
        .collect::<Vec<f64>>();
```

## Performance

`om-nomnomnom` focuses on correctness more than performance.  Even so its performance is on par with other Rust implementations and well ahead of the reference parser written in Python.

The speed / correctness tradeoff can be further configured with the following features:

* naive_wide_char_support
* no_interleave_metric
* enforce_timestamp_monotonic
* hash_fnv

## TODO

* Serialization
* Convenience structs for each family type
