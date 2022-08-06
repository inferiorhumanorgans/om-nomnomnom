# openmetrics-bench

Parsing speed of the OpenMetrics [reference library](https://github.com/prometheus/client_python/), [`openmetrics-parser`](https://crates.io/crates/openmetrics-parser/), and `om-nomnomnom`.


### Usage

To launch criterion:

```sh
$ cargo bench -p om-bench
```

Note: Python 3 is required.

### Performance with an i5-3570

```
╔═══════════════════════════════════════════════════╗
║ roundtrip/python                       1422 / sec ║
║ roundtrip/openmetrics-parser           4419 / sec ║
║ roundtrip/om-nomnomnom                 9200 / sec ║
╠───────────────────────────────────────────────────╢
║ escaping/python                        8874 / sec ║
║ escaping/openmetrics-parser           43617 / sec ║
║ escaping/om-nomnomnom                 70192 / sec ║
╚═══════════════════════════════════════════════════╝
```
