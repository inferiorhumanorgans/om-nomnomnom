use std::fs::{self, DirEntry};

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use pyo3::prelude::*;
use pyo3::types::IntoPyDict;

// const FILTER :&[&'static str] = &[];
const FILTER: &[&'static str] = &["roundtrip", "escaping"];
// const FILTER :&[&'static str] = &["bad_value_9"];
// const FILTER: &[&'static str] = &["empty_metadata"];

fn enumerate_test_cases() -> Vec<DirEntry> {
    let test_dir: &'static str = concat!(env!("CARGO_MANIFEST_DIR"), "/../parse-tests");
    let test_dir = std::fs::canonicalize(test_dir).unwrap();

    let mut dirs: Vec<DirEntry> = fs::read_dir(test_dir)
        .unwrap()
        .map(|x| x.unwrap())
        .collect();
    dirs.sort_by(|b, a| a.path().cmp(&b.path()));
    dirs.into_iter()
        .filter(|x| FILTER.is_empty() || FILTER.contains(&x.file_name().to_str().unwrap()))
        .collect()
}

const PYTHON_CODE: &'static str = include_str!("../scripts/py-reference-parser.py");

#[inline]
fn do_test<A>(group: &mut criterion::BenchmarkGroup<A>, path: DirEntry)
where
    A: criterion::measurement::Measurement,
{
    let test_name: String = path
        .file_name()
        .into_string()
        .expect("filename is not utf-8");
    let test_data =
        std::fs::read_to_string(path.path().join("metrics")).expect("couldn't read input");

    let _: PyResult<_> = Python::with_gil(|py| {
        let sys = py.import("sys")?;
        let prom = py.import("prometheus_client.openmetrics.parser")?;
        let locals = [("prometheus_client", prom), ("sys", sys)].into_py_dict(py);
        locals.set_item("data", &test_data)?;

        group.bench_function(BenchmarkId::new(&test_name, "python"), |b| {
            b.iter(|| {
                py.run(PYTHON_CODE, None, Some(&locals))
                    .expect("python error")
            })
        });

        Ok(())
    });

    group.bench_function(BenchmarkId::new(&test_name, "om-nomnomnom"), |b| {
        b.iter(|| match om_nomnomnom::parse(&test_data) {
            Ok(data) => {
                data.values().count();
            }
            Err(_) => {}
        })
    });

    group.bench_function(BenchmarkId::new(&test_name, "openmetrics-parser"), |b| {
        b.iter(
            || match openmetrics_parser::openmetrics::parse_openmetrics(&test_data) {
                Ok(data) => {
                    data.families.values().count();
                }
                Err(_) => {}
            },
        )
    });
}

fn openmetrics(cr: &mut Criterion) {
    let paths = enumerate_test_cases();
    let (bad, good): (_, Vec<DirEntry>) = paths.into_iter().partition(|x| {
        x.file_name()
            .into_string()
            .expect("non-utf8 filename")
            .starts_with("bad_")
    });

    {
        let mut group = cr.benchmark_group("should_pass");
        for path in good {
            if !path
                .file_type()
                .expect("couldn't determine file type")
                .is_dir()
            {
                continue;
            }

            do_test(&mut group, path);
        }
    }

    {
        let mut group = cr.benchmark_group("should_fail");
        for path in bad {
            if !path
                .file_type()
                .expect("couldn't determine file type")
                .is_dir()
            {
                continue;
            }

            do_test(&mut group, path);
        }
    }
}

criterion_group!(openmetrics_benches, openmetrics);
criterion_main!(openmetrics_benches);
