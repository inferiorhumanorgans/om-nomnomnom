#[allow(unused)]
use tracing::{debug, error, info, span, trace, warn, Instrument, Level};

use std::str::FromStr;

use itertools::{Itertools, Position};
use nom::{
    branch::alt,
    bytes::complete::{tag, take_while},
    character::complete::{alphanumeric1, satisfy},
    combinator::{eof, map, map_res, opt, peek, value},
    multi::separated_list0,
    sequence::{delimited, preceded, terminated, tuple},
    IResult,
};
use serde::Serializer;
use serde_derive::Serialize;

mod types;
use types::*;

#[derive(Clone, Debug, Serialize)]
pub struct Exemplar<'a> {
    pub labels: Vec<Label<'a>>,
    pub number: f64,
    pub timestamp: Option<f64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Label<'a> {
    pub name: &'a str,
    pub value: Option<&'a str>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Sample<'a> {
    pub name: &'a str,
    pub labels: Option<Vec<Label<'a>>>,
    pub number: MetricNumber,
    pub timestamp: Option<f64>,
    pub exemplar: Option<Exemplar<'a>>,
}

#[derive(Clone, Debug, Serialize)]
pub enum MetricDescriptor<'a> {
    Type {
        metric_name: &'a str,
        metric_type: MetricType,
    },
    Help {
        metric_name: &'a str,
        help_text: Option<&'a str>,
    },
    Unit {
        metric_name: &'a str,
        unit: Option<&'a str>,
    },
}

#[derive(Debug, Serialize)]
struct MetricFamily<'a> {
    tokens: Vec<MetricToken<'a>>,
}

#[derive(Clone, Debug)]
pub enum MetricNumber {
    Float(f64),
    Integer(i64),
}

#[derive(Clone, Debug, Serialize)]
pub enum MetricToken<'a> {
    Descriptor(MetricDescriptor<'a>),
    Metric(Sample<'a>),
    Eof,
    Empty,
}

#[derive(Clone, Debug, Serialize)]
pub enum MetricType {
    Counter,
    Gauge,
    Histogram,
    GaugeHistogram,
    StateSet,
    Info,
    Summary,
    Unknown,
}

impl<'a> Exemplar<'a> {
    /// ```abnf
    /// exemplar = SP HASH SP labels SP number [SP timestamp]
    /// ```
    fn nom(input: &'a str) -> IResult<&str, Self> {
        map(
            tuple((
                tag(" # "),
                delimited(tag("{"), separated_list0(tag(","), Label::nom), tag("}")),
                tag(" "),
                floatlike,
                opt(preceded(single_space, realnumber)),
            )),
            |(_, labels, _, number, timestamp)| Self {
                labels,
                number,
                timestamp,
            },
        )(input)
    }
}

impl<'a> Label<'a> {
    #[tracing::instrument]
    fn nom(input: &'a str) -> IResult<&str, Self> {
        map(
            tuple((
                Self::label_name1,
                tag("="),
                delimited(tag(r#"""#), opt(escaped_string1), tag(r#"""#)),
            )),
            |(name, _, value)| {
                debug!(name, value);
                Self { name, value }
            },
        )(input)
    }

    /// ```abnf
    /// label-name = label-name-initial-char *label-name-char
    ///
    /// label-name-char = label-name-initial-char / DIGIT
    /// label-name-initial-char = ALPHA / "_"
    /// ```
    #[tracing::instrument]
    fn label_name1(input: &str) -> IResult<&str, &str> {
        peek(satisfy(|c| c.is_alphabetic() || c == '_'))(input)?;

        take_while(|item: char| item.is_alphanumeric() || item == '_')(input)
    }
}

impl<'a> Sample<'a> {
    fn nom(input: &'a str) -> IResult<&str, Self> {
        let (input, name) = metric_name1(input)?;
        let (input, labels) = map(
            opt(delimited(
                tag("{"),
                separated_list0(tag(","), Label::nom),
                tag("}"),
            )),
            |labels| match labels {
                Some(labels) if labels.len() > 0 => Some(labels),
                Some(_) => None,
                None => None,
            },
        )(input)?;
        let (input, number) = preceded(single_space, MetricNumber::nom)(input)?;

        let (input, timestamp) = opt(preceded(single_space, realnumber))(input)?;
        let (input, exemplar) = opt(Exemplar::nom)(input)?;
        let (input, _) = eof(input)?;

        Ok((
            input,
            Self {
                name,
                number,
                labels,
                timestamp,
                exemplar,
            },
        ))
    }
}

impl<'a> MetricDescriptor<'a> {
    /// ```abnf
    /// type = %d84.89.80.69
    /// metric-descriptor = HASH SP type SP metricname SP metric-type LF
    /// ```
    fn nom_type_descriptor(input: &'a str) -> IResult<&str, Self> {
        map(
            terminated(
                tuple((
                    tag("TYPE"),
                    single_space,
                    metric_name1,
                    preceded(single_space, MetricType::nom),
                )),
                eof,
            ),
            |(_, _, metric_name, metric_type)| MetricDescriptor::Type {
                metric_name,
                metric_type,
            },
        )(input)
    }

    /// ```abnf
    /// help = %d72.69.76.80
    /// metric-descriptor =/ HASH SP help SP metricname SP escaped-string LF
    /// ```
    fn nom_help_descriptor(input: &'a str) -> IResult<&str, Self> {
        map(
            tuple((
                tag("HELP"),
                single_space,
                metric_name1,
                single_space,
                map(
                    opt(escaped_string1),
                    // Yiiiiikes
                    |s| match s {
                        Some(s) if s.is_empty() => None,
                        Some(s) => Some(s),
                        None => None,
                    },
                ),
            )),
            |(_, _, metric_name, _, help_text)| MetricDescriptor::Help {
                metric_name,
                help_text: help_text,
            },
        )(input)
    }

    /// ```abnf
    /// unit = %d85.78.73.84
    /// metric-descriptor =/ HASH SP unit SP metricname SP *metricname-char LF
    /// ```
    fn nom_unit_descriptor(input: &'a str) -> IResult<&str, Self> {
        map(
            tuple((
                tag("UNIT"),
                single_space,
                metric_name1,
                opt(tuple((single_space, alphanumeric1))),
            )),
            |(_, _, metric_name, unit)| MetricDescriptor::Unit {
                metric_name,
                unit: unit.map(|t| t.1),
            },
        )(input)
    }

    /// ```abnf
    /// metric-descriptor = HASH SP type SP metricname SP metric-type LF
    /// metric-descriptor =/ HASH SP help SP metricname SP escaped-string LF
    /// metric-descriptor =/ HASH SP unit SP metricname SP *metricname-char LF
    /// ```
    fn nom(input: &'a str) -> IResult<&str, Self> {
        let (input, _) = tuple((tag("#"), single_space))(input)?;
        alt((
            Self::nom_type_descriptor,
            Self::nom_help_descriptor,
            Self::nom_unit_descriptor,
        ))(input)
    }
}

impl<'a> MetricFamily<'a> {
    fn nom_last_line(input: &'a str) -> IResult<&str, MetricToken<'a>> {
        alt((
            value(MetricToken::Eof, terminated(tag("# EOF"), eof)),
            value(MetricToken::Empty, eof),
        ))(input)
    }
    fn nom(input: &'a str) -> IResult<&str, MetricToken<'a>> {
        alt((
            map(MetricDescriptor::nom, MetricToken::Descriptor),
            map(Sample::nom, MetricToken::Metric),
            value(MetricToken::Eof, terminated(tag("# EOF"), eof)),
        ))(input)
    }
}

impl<'a> MetricNumber {
    fn nom(input: &'a str) -> IResult<&str, Self> {
        alt((
            map(floatlike, Self::Float),
            map(nom::character::complete::i64, Self::Integer),
        ))(input)
    }
}

impl serde::Serialize for MetricNumber {
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Float(n) => {
                if n.is_nan() {
                    s.serialize_str("NaN")
                } else if n.is_infinite() && n.is_sign_positive() {
                    s.serialize_str("+Inf")
                } else if n.is_infinite() && n.is_sign_negative() {
                    s.serialize_str("-Inf")
                } else {
                    s.serialize_f64(*n)
                }
            }
            Self::Integer(n) => s.serialize_i64(*n),
        }
    }
}

impl MetricType {
    fn nom(input: &str) -> IResult<&str, Self> {
        map_res(
            alt((
                tag("counter"),
                tag("gaugehistogram"),
                tag("gauge"),
                tag("histogram"),
                tag("stateset"),
                tag("info"),
                tag("summary"),
                tag("summary"),
                tag("unknown"),
            )),
            MetricType::from_str,
        )(input)
    }
}

impl FromStr for MetricType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "counter" => Ok(Self::Counter),
            "gauge" => Ok(Self::Gauge),
            "gaugehistogram" => Ok(Self::GaugeHistogram),
            "histogram" => Ok(Self::Histogram),
            "stateset" => Ok(Self::StateSet),
            "info" => Ok(Self::Info),
            "summary" => Ok(Self::Summary),
            "unknown" => Ok(Self::Unknown),
            _ => Err(()),
        }
    }
}

#[tracing::instrument(skip(input))]
pub(super) fn exposition<'a>(input: &'a str) -> IResult<&str, Vec<MetricToken<'a>>> {
    debug!(input);
    let data: Result<Vec<_>, _> = input
        .split("\n")
        .with_position()
        .map(|line| {
            debug!(?line);
            match line {
                Position::First(line) | Position::Middle(line) => MetricFamily::nom(line),
                Position::Only(line) | Position::Last(line) => MetricFamily::nom_last_line(line),
            }
        })
        .collect();
    let data = data?.into_iter().map(|x| x.1).collect();
    Ok(("", data))
}
