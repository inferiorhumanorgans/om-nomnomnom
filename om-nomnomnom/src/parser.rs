#[allow(unused)]
use tracing::{debug, error, info, span, trace, warn, Instrument, Level};

use std::{
    borrow::{Borrow, Cow},
    collections::{HashMap, HashSet},
    hash::{Hash, Hasher},
};

#[cfg(not(feature = "naive_label_hash"))]
use std::cmp::Ordering;

use itertools::{Itertools, Position};
use lazy_static::lazy_static;
use regex::{Captures, Regex};
use serde_derive::Serialize;

use crate::lexer;

// Suffixes for a MetricFamily that could conflict with a valid sample name
//
// From the spec:
//
// The name of a MetricFamily MUST NOT result in a potential clash for sample metric names as per
// the ABNF with another MetricFamily in the Text Format within a MetricSet. An example would be
// a gauge called "foo_created" as a counter called "foo" could create a "foo_created" in the
// text format.
//
// Exposers SHOULD avoid names that could be confused with the suffixes that text format sample
// metric names use.
//
// Suffixes for the respective types are:
// Counter: '_total', '_created'
// Summary: '_count', '_sum', '_created', '' (empty)
// Histogram: '_count', '_sum', '_bucket', '_created'
// GaugeHistogram: '_gcount', '_gsum', '_bucket'
// Info: '_info'
// Gauge: '' (empty)
// StateSet: '' (empty)
// Unknown: '' (empty)
const CONFLICT_SUFFIXES: &[&str] = &[
    "_bucket", "_count", "_created", "_gcount", "_gsum", "_info", "_sum", "_total",
];

lazy_static! {
    // Pattern used to check for escape characters
    static ref UNESCAPE_RE: Regex = Regex::new(r#"(\\[n"\\])"#).unwrap();
}

struct Builder<'a> {
    name: Option<&'a str>,
    help: Option<&'a str>,
    unit: Option<&'a str>,
    metric_type: Option<MetricType>,
    samples: Vec<Sample<'a>>,
    families: HashMap<&'a str, MetricFamily<'a>>,
    flags: BuilderFlags,
}

#[derive(Debug, Default)]
struct BuilderFlags {
    has_inf_bucket: bool,
    has_total_bucket: bool,
    has_neg_bucket: bool,
    has_bucket: bool,
    has_count: bool,
    has_gcount: bool,
    has_gsum: bool,
    has_sum: bool,
    has_eof: bool,
}

/// Exemplars are references to data outside of the MetricSet. A common use case are IDs of program traces.
///
/// Exemplars MUST consist of a LabelSet and a value, and MAY have a timestamp. They MAY each be different from the MetricPoints' LabelSet and timestamp.
#[derive(Clone, Debug, Serialize)]
pub struct Exemplar<'a> {
    labels: HashMap<&'a str, Cow<'a, str>>,
    number: f64,
    timestamp: Option<f64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Label<'a> {
    pub name: &'a str,
    pub value: Cow<'a, str>,
}

/// A MetricFamily is a collection of related (and similarly named) metrics
#[derive(Debug, Serialize)]
pub struct MetricFamily<'a> {
    pub metric_type: MetricType,
    pub help: Option<Cow<'a, str>>,
    pub unit: Option<&'a str>,
    pub samples: Vec<Sample<'a>>,
}

/// [`MetricFamily`] type.  The default is `Unknown`
#[derive(Debug, PartialEq, Serialize)]
pub enum MetricType {
    /// Counters measure discrete events.
    Counter,
    /// Gauges are current measurements, such as bytes of memory currently used or the number of items in a queue.
    Gauge,
    /// GaugeHistograms measure current distributions. Common examples are how long items have been waiting in a queue, or size of the requests in a queue.
    GaugeHistogram,
    /// Histograms measure distributions of discrete events.
    Histogram,
    /// Info metrics are used to expose textual information which SHOULD NOT change during process lifetime.
    Info,
    /// StateSets represent a series of related boolean values, also called a bitset.
    StateSet,
    /// Summaries also measure distributions of discrete events and MAY be used when Histograms are too expensive and/or an average event size is sufficient.
    Summary,
    /// Unknown SHOULD NOT be used. Unknown MAY be used when it is impossible to determine the types of individual metrics from 3rd party systems.
    Unknown,
}

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum ParseError {
    #[error("bad bucket")]
    BadBucket,

    #[error("Buckets MUST be sorted in number increasing order of «le»")]
    BadBucketOrder,

    #[error("malformed «Counter» MetricFamily")]
    BadCounter,

    #[error("malformed «Histogram» MetricFamily")]
    BadHistogram,

    #[error("malformed «Info» MetricFamily")]
    BadInfo,

    #[error("label too long")]
    BadLabelTooLong,

    #[error("malformed quantile sample")]
    BadQuantile,

    #[error("malformed «StateSet» MetricFamily")]
    BadStateSet,

    #[error("malformed «Summary» MetricFamily")]
    BadSummary,

    #[error("invalid sample suffix")]
    BadSuffix,

    #[cfg(feature = "enforce_timestamp_monotonic")]
    #[error("timestamps must increase monotonically")]
    BadTimestampOutOfOrder,

    #[error("duplicate metadata name/type/unit/help")]
    DuplicateMeta,

    #[error("empty label")]
    EmptyLabel,

    #[error("noise after eof")]
    Eof,

    #[error("interleaved data")]
    Interleave,

    #[error("missing samples")]
    MissingSample,

    #[error("MetricFamily name conflict")]
    NameConflict,

    #[cfg(feature = "generic_parse_error")]
    #[error("unknown error")]
    Unknown,
}

pub type Result<T> = std::result::Result<T, ParseError>;

#[derive(Debug, Serialize)]
pub struct Sample<'a> {
    pub name: &'a str,
    pub labels: HashMap<&'a str, Cow<'a, str>>,
    pub number: f64,
    pub timestamp: Option<f64>,
    pub exemplar: Option<Exemplar<'a>>,

    pub kind: SampleKind,
}

#[derive(Debug, PartialEq, Serialize)]
pub enum SampleKind {
    Other,
    Count,
    Total,
    Sum,
    GCount,
    GSum,
    HistogramBucket(f64),
    Quantile(f64),
}

impl<'a> Builder<'a> {
    fn new() -> Self {
        Self::default()
    }

    fn name(self, metric_name: &'a str) -> Result<Self> {
        if self.flags.has_eof {
            Err(ParseError::Eof)?
        }

        match self.name {
            None => Ok(Self {
                name: Some(metric_name),
                ..self
            }),
            Some(name) => {
                if name != metric_name {
                    self.finalize_family(Some(metric_name))
                } else {
                    Ok(self)
                }
            }
        }
    }

    fn is_meta_allowable(self) -> Result<Self> {
        if self.flags.has_eof {
            Err(ParseError::Eof)?
        }

        if !self.samples.is_empty() {
            Err(ParseError::DuplicateMeta)?
        }

        Ok(self)
    }

    fn metric_type(self, metric_type: MetricType) -> Result<Self> {
        if self.metric_type.is_some() {
            Err(ParseError::DuplicateMeta)?
        }

        Ok(Self {
            metric_type: Some(metric_type),
            ..self
        })
    }

    fn help_text(self, help_text: Option<&'a str>) -> Result<Self> {
        if self.help.is_some() {
            Err(ParseError::DuplicateMeta)?
        }

        Ok(Self {
            help: help_text.or_else(|| "".into()),
            ..self
        })
    }

    fn unit(self, unit: Option<&'a str>) -> Result<Self> {
        if self.unit.is_some() {
            Err(ParseError::DuplicateMeta)?
        }

        Ok(Self {
            unit: unit.or_else(|| "".into()),
            ..self
        })
    }

    fn meta(self, meta: lexer::MetricDescriptor<'a>) -> Result<Self> {
        if self.flags.has_eof {
            Err(ParseError::Eof)?
        }

        match meta {
            lexer::MetricDescriptor::Type {
                metric_name,
                metric_type,
            } => self
                .name(metric_name)?
                .is_meta_allowable()?
                .metric_type(metric_type.into()),
            lexer::MetricDescriptor::Help {
                metric_name,
                help_text,
            } => self
                .name(metric_name)?
                .is_meta_allowable()?
                .help_text(help_text),
            lexer::MetricDescriptor::Unit { metric_name, unit } => {
                self.name(metric_name)?.is_meta_allowable()?.unit(unit)
            }
        }
    }

    fn sample(self, sample: lexer::Sample<'a>) -> Result<Self> {
        if self.flags.has_eof {
            Err(ParseError::Eof)?
        }

        let mut builder = match self.name {
            None => self.name(sample.name)?,
            Some(_) => self,
        };

        let sample: Sample = sample.try_into()?;

        // [Counter] A MetricPoint in a Metric's Counter's Total MAY have an exemplar.
        // [Histogram] Bucket values MAY have exemplars.
        // [Histogram] Each bucket covers the values less and or equal to it, and the value of the exemplar MUST be within this range. Exemplars SHOULD be put into the bucket with the highest value. A bucket MUST NOT have more than one exemplar.
        // [GaugeHistogram] Bucket values can have exemplars.
        // [GaugeHistogram] Each bucket covers the values less and or equal to it, and the value of the exemplar MUST be within this range. Exemplars SHOULD be put into the bucket with the highest value. A bucket MUST NOT have more than one exemplar.
        if sample.exemplar.is_some() {
            // This is ugly
            if sample.name.ends_with("_bucket")
                && (builder.metric_type == Some(MetricType::Histogram)
                    || builder.metric_type == Some(MetricType::GaugeHistogram))
            {
            } else if sample.name.ends_with("_total")
                && builder.metric_type == Some(MetricType::Counter)
            {
            } else {
                Err(ParseError::BadSuffix)?
            }
        }

        if let Some(family_name) = builder.name.as_ref() {
            match builder.metric_type {
                Some(MetricType::Info) => {
                    if !(sample.name.starts_with(family_name) && sample.name.ends_with("_info")) {
                        Err(ParseError::BadInfo)?
                    } else if !sample.labels.contains_key(family_name) {
                        Err(ParseError::BadInfo)?
                    } else if sample.number != 1. {
                        // The Sample value MUST always be 1.
                        Err(ParseError::BadInfo)?
                    }
                }
                Some(MetricType::StateSet) => {
                    if &sample.name != family_name || !sample.labels.contains_key(family_name) {
                        Err(ParseError::BadStateSet)?
                    } else if sample.number != 1. && sample.number != 0. {
                        // The State sample's value MUST be 1 if the State is true and MUST be 0 if the State is false.
                        Err(ParseError::BadStateSet)?
                    }
                }
                Some(MetricType::Summary) => {
                    if !sample.name.ends_with("_count")
                        && !sample.name.ends_with("_sum")
                        && !sample.name.ends_with("_created")
                        && !(&sample.name == family_name && sample.labels.contains_key("quantile"))
                    {
                        Err(ParseError::BadSummary)?
                    }
                }
                _ => {}
            }

            match sample.kind {
                SampleKind::HistogramBucket(threshold) => {
                    builder.flags.has_bucket = true;

                    if threshold.is_infinite() {
                        trace!("is [+Inf]");
                        builder.flags.has_inf_bucket = true;
                    } else {
                        trace!("is {}", threshold);
                        if threshold < 0. {
                            builder.flags.has_neg_bucket = true;
                        }
                    }
                }
                SampleKind::Count => builder.flags.has_count = true,
                SampleKind::Total => builder.flags.has_total_bucket = true,
                SampleKind::GCount => builder.flags.has_gcount = true,
                SampleKind::Sum => {
                    if let Some(MetricType::Summary) = builder.metric_type && sample.number < 0. {
                        Err(ParseError::BadCounter)?
                    } else if builder.flags.has_neg_bucket {
                        Err(ParseError::BadHistogram)?
                    }
                    builder.flags.has_sum = true;
                }
                SampleKind::GSum => {
                    info!(flags=?builder.flags, number=sample.number);
                    if sample.number < 0. && !builder.flags.has_neg_bucket {
                        Err(ParseError::BadCounter)?
                    }

                    builder.flags.has_gsum = true;
                }
                _ => {
                    let is_quantile = sample.labels.contains_key("quantile");
                    if is_quantile && sample.number < 0. {
                        Err(ParseError::BadCounter)?
                    }
                }
            }
        }

        builder.samples.push(sample);

        Ok(builder)
    }

    fn eof(self) -> Result<Self> {
        Ok(Self {
            flags: BuilderFlags {
                has_eof: true,
                ..self.flags
            },
            ..self
        })
    }

    fn finalize_family(mut self, name: Option<&'a str>) -> Result<Self> {
        #[cfg(feature = "no_interleave_metric")]
        {
            self.samples.iter().try_fold(
                (None, HashSet::with_capacity(self.samples.len())),
                |(last_labelset, mut acc), sample| {
                    let key = sample.labelset();

                    if let Some(labelset) = last_labelset && labelset != key && acc.contains(&key) {
                        Err(ParseError::Interleave)
                    } else {
                        acc.insert(key.clone());
                        Ok((Some(key), acc))
                    }
                },
            )?;
        }

        #[cfg(feature = "enforce_timestamp_monotonic")]
        {
            self.samples.iter().try_fold(
                (None, None),
                |(cur_id, cur_timestamp): (Option<String>, Option<f64>), sample| {
                    let new_id = format!("{}:{}", sample.name, sample.labelset());
                    match (cur_id, new_id) {
                        (None, new_id) => Ok((Some(new_id), sample.timestamp)),
                        (Some(cur_id), new_id) if cur_id != new_id => {
                            Ok((Some(new_id), sample.timestamp))
                        }
                        (cur_id, _) => match (cur_timestamp, sample.timestamp) {
                            (None, _new_timestamp) => Err(ParseError::BadTimestampOutOfOrder),
                            (Some(_cur_timestamp), None) => Err(ParseError::BadTimestampOutOfOrder),
                            (Some(cur_timestamp), Some(new_timestamp)) => {
                                if new_timestamp < cur_timestamp {
                                    Err(ParseError::BadTimestampOutOfOrder)
                                } else {
                                    Ok((cur_id, Some(new_timestamp)))
                                }
                            }
                        },
                    }
                },
            )?;
        }

        // The name of a MetricFamily MUST NOT result in a potential clash for sample metric names
        // as per the ABNF with another MetricFamily in the Text Format within a MetricSet. An
        // example would be a gauge called "foo_created" as a counter called "foo" could create a
        // "foo_created" in the text format.
        if let Some(family_name) = self.name.as_ref() {
            for conflict in CONFLICT_SUFFIXES.iter() {
                let key = format!("{}{}", family_name, conflict);
                if self.families.contains_key(key.as_str()) {
                    Err(ParseError::NameConflict)?;
                }
            }
        }

        match self.metric_type {
            Some(MetricType::Histogram) => {
                if !self.flags.has_bucket {
                    Err(ParseError::BadHistogram)?
                } else if !self.flags.has_inf_bucket {
                    Err(ParseError::BadHistogram)?
                } else if self.flags.has_neg_bucket && self.flags.has_sum {
                    Err(ParseError::BadHistogram)?
                } else if (self.flags.has_sum && !self.flags.has_count)
                    || (!self.flags.has_sum && self.flags.has_count)
                {
                    Err(ParseError::BadHistogram)?
                } else if self.flags.has_count {
                    // If and only if a Sum Value is present in a MetricPoint, then the
                    // MetricPoint's +Inf Bucket value MUST also appear in a Sample with a
                    // MetricName with the suffix "_count".
                    #[cfg(feature = "validate_histogram_count")]
                    {
                        let counts = self
                            .samples
                            .iter()
                            .filter(|sample| 
                                sample.kind == SampleKind::HistogramBucket(f64::INFINITY) ||
                                sample.kind == SampleKind::Count
                            )
                            .map(|sample| sample.number)
                            .collect_vec();
                        if counts.len() != 2 || counts[0] != counts[1] {
                            Err(ParseError::BadHistogram)?
                        }
                    }
                }

                let bucket_it = self.samples.iter().filter_map(|sample| match sample.kind {
                    SampleKind::HistogramBucket(_) => Some(sample),
                    _ => None,
                });

                // Semantically, Sum, and buckets values are counters so MUST NOT be NaN or negative.
                bucket_it.clone().try_fold(0., |acc, sample| {
                    if sample.number < acc {
                        Err(ParseError::BadCounter)
                    } else {
                        Ok(sample.number)
                    }
                })?;

                // Buckets MUST be sorted in number increasing order of "le", and the value of the
                // "le" label MUST follow the rules for Canonical Numbers.
                // .with_position()
                bucket_it
                    .clone()
                    .filter_map(|sample| {
                        if let SampleKind::HistogramBucket(threshold) = sample.kind {
                            Some(threshold)
                        } else {
                            None
                        }
                    })
                    .with_position()
                    .try_fold(0., |acc, sample| match sample {
                        Position::First(threshold) => Ok(threshold),
                        Position::Only(threshold) | Position::Last(threshold) => {
                            match threshold.is_infinite() {
                                true => Ok(threshold),
                                false => Err(ParseError::BadBucketOrder),
                            }
                        }
                        Position::Middle(threshold) => {
                            if threshold <= acc {
                                Err(ParseError::BadBucketOrder)
                            } else {
                                Ok(threshold)
                            }
                        }
                    })?;
            }
            Some(MetricType::GaugeHistogram) => {
                if !self.flags.has_bucket {
                    Err(ParseError::BadHistogram)?
                } else if !self.flags.has_inf_bucket {
                    Err(ParseError::BadHistogram)?
                } else if self.flags.has_gcount != self.flags.has_gsum {
                    Err(ParseError::BadHistogram)?
                }
            }
            Some(MetricType::Counter) => {
                if !self.samples.is_empty() && !self.flags.has_total_bucket {
                    Err(ParseError::BadCounter)?
                }
            }
            _ => {}
        }

        let family = MetricFamily {
            metric_type: self.metric_type.unwrap_or(MetricType::Unknown),
            help: self.help.map(unescape_string),
            unit: self.unit,
            samples: self.samples,
        };

        self.families.insert(self.name.expect("our name"), family);

        Ok(Self {
            families: self.families,
            name,
            ..Self::default()
        })
    }

    fn finalize(self) -> Result<HashMap<&'a str, MetricFamily<'a>>> {
        if !self.flags.has_eof {
            Err(ParseError::Eof)?
        }

        Ok(self.finalize_family(None)?.families)
    }
}

impl<'a> Default for Builder<'a> {
    fn default() -> Self {
        Self {
            name: None,
            help: None,
            unit: None,
            metric_type: None,
            samples: vec![],
            families: HashMap::new(),
            flags: BuilderFlags::default(),
        }
    }
}

impl<'a> TryFrom<lexer::Exemplar<'a>> for Exemplar<'a> {
    type Error = ParseError;

    fn try_from(l: lexer::Exemplar<'a>) -> Result<Self> {
        Ok(Self {
            labels: Label::from_lexer_labels(l.labels)?,
            number: l.number,
            timestamp: l.timestamp,
        })
    }
}

impl<'a> Label<'a> {
    fn from_lexer_labels(l: Vec<lexer::Label<'a>>) -> Result<HashMap<&'a str, Cow<'a, str>>> {
        let l = l
            .into_iter()
            .map(|l| Label::try_from(l))
            .filter(|l| {
                if let Err(ParseError::EmptyLabel) = l {
                    false
                } else {
                    true
                }
            })
            .try_fold(HashMap::new(), |mut acc, label| {
                let label = label?;
                if acc.contains_key(label.name) {
                    Err(ParseError::DuplicateMeta)
                } else {
                    acc.insert(label.name, label.value);
                    Ok(acc)
                }
            })?;

        Ok(l)
    }
}

impl<'a> TryFrom<lexer::Label<'a>> for Label<'a> {
    type Error = ParseError;

    fn try_from(l: lexer::Label<'a>) -> Result<Self> {
        match l.value {
            Some(value) => {
                #[cfg(feature = "naive_wide_char_support")]
                {
                    let total_length = l.name.len() + value.len();
                    if l.name.len() + value.len() > 256 {
                        error!(name_len = l.name.len(), value_len = value.len());
                        Err(ParseError::BadLabelTooLong)?
                    }
                }

                #[cfg(not(feature = "naive_wide_char_support"))]
                {
                    let total_length = l.name.len() + value.chars().count();
                    if total_length > 128 {
                        error!(name_len = l.name.len(), value_len = value.len());
                        Err(ParseError::BadLabelTooLong)?
                    }
                }

                Ok(Self {
                    name: l.name,
                    value: unescape_string(value),
                })
            }
            None => Err(ParseError::EmptyLabel),
        }
    }
}

impl From<lexer::MetricType> for MetricType {
    fn from(l: lexer::MetricType) -> Self {
        match l {
            lexer::MetricType::Counter => Self::Counter,
            lexer::MetricType::Gauge => Self::Gauge,
            lexer::MetricType::Histogram => Self::Histogram,
            lexer::MetricType::GaugeHistogram => Self::GaugeHistogram,
            lexer::MetricType::StateSet => Self::StateSet,
            lexer::MetricType::Info => Self::Info,
            lexer::MetricType::Summary => Self::Summary,
            lexer::MetricType::Unknown => Self::Unknown,
        }
    }
}

impl<'a> Sample<'a> {
    fn labelset(&self) -> u64 {
        #[cfg(not(feature = "hash_fnv"))]
        let mut hasher = DefaultHasher::new();

        #[cfg(feature = "hash_fnv")]
        let mut hasher = fnv::FnvHasher::default();

        #[cfg(not(feature = "naive_label_hash"))]
        let it = {
            let mut key = self.labels.iter().collect::<Vec<_>>();
            key.sort_by(|a, b| match a.0.cmp(b.0) {
                Ordering::Equal => a.1.cmp(b.1),
                other => other,
            });
            key
        };

        #[cfg(feature = "naive_label_hash")]
        let it = self.labels.iter();

        for k in it {
            k.0.hash(&mut hasher);
            k.1.hash(&mut hasher);
        }

        hasher.finish()
    }
}

impl<'a> TryFrom<lexer::Sample<'a>> for Sample<'a> {
    type Error = ParseError;

    fn try_from(l: lexer::Sample<'a>) -> Result<Self> {
        let number = match l.number {
            lexer::MetricNumber::Float(f) => f,
            lexer::MetricNumber::Integer(i) => i as f64,
        };

        let labels = Label::from_lexer_labels(l.labels.unwrap_or_else(|| vec![]))?;

        let exemplar = match l.exemplar {
            Some(ex) => Some(Exemplar::try_from(ex)?),
            None => None,
        };

        let name = l.name;

        let kind = if name.ends_with("_bucket") {
            // Bucket thresholds MUST NOT equal NaN.
            let threshold_str = labels.get("le").ok_or(ParseError::BadBucket)?;
            let threshold = match threshold_str.borrow() {
                "+Inf" => f64::INFINITY,
                threshold_str => threshold_str
                    .parse::<f64>()
                    .map_err(|_| ParseError::BadBucket)
                    .and_then(|x| {
                        if x.is_infinite() {
                            Err(ParseError::BadBucket)
                        } else {
                            Ok(x)
                        }
                    })?,
            };

            if number < 0. {
                Err(ParseError::BadHistogram)?
            }

            // Technically value shouldn't be a float, but for now this should do
            if number.is_infinite() || number.is_nan() {
                Err(ParseError::BadHistogram)?
            }

            if threshold.is_nan() {
                Err(ParseError::BadBucket)?;
            }

            SampleKind::HistogramBucket(threshold)
        } else if name.ends_with("_count") {
            if number < 0. || number.is_nan() {
                Err(ParseError::BadHistogram)?
            }
            SampleKind::Count
        } else if name.ends_with("_total") {
            if number.is_nan() || number < 0. {
                Err(ParseError::BadCounter)?
            }
            SampleKind::Total
        } else if name.ends_with("_sum") {
            if number.is_nan() {
                Err(ParseError::BadCounter)?
            }

            SampleKind::Sum
        } else if name.ends_with("_gcount") {
            SampleKind::GCount
        } else if name.ends_with("_gsum") {
            if number.is_nan() {
                Err(ParseError::BadCounter)?
            }
            SampleKind::GSum
        } else if let Some(quantile) = labels.get("quantile") {
            // Summary quantiles must be float64, as they are estimates and thus fundamentally inaccurate.
            // Quantiles MUST be between 0 and 1 inclusive.
            // Quantile values MUST NOT be negative.

            let quantile = quantile
                .parse::<f64>()
                .map_err(|_| ParseError::BadQuantile)
                .and_then(|q| {
                    if q.is_nan() {
                        Err(ParseError::BadQuantile)
                    } else if q < 0. || q > 1. {
                        Err(ParseError::BadQuantile)
                    } else {
                        Ok(q)
                    }
                })?;

            SampleKind::Quantile(quantile)
        } else {
            SampleKind::Other
        };

        Ok(Self {
            name: l.name,
            labels,
            number,
            timestamp: l.timestamp,
            exemplar,
            kind,
        })
    }
}

fn unescape_string<'a>(input: &'a str) -> Cow<'a, str> {
    UNESCAPE_RE.replace_all(input, |caps: &Captures| {
        match caps.get(0).unwrap().as_str() {
            r"\n" => format!("\n"),
            r#"\""# => format!(r#"""#),
            r#"\\"# => format!(r#"\"#),
            c => c.to_string(),
        }
    })
}

#[tracing::instrument(skip_all)]
pub fn parse(tokens: Vec<crate::lexer::MetricToken>) -> Result<HashMap<&str, MetricFamily>> {
    Ok(tokens
        .into_iter()
        .try_fold(Builder::new(), |builder, token| match token {
            lexer::MetricToken::Descriptor(meta) => builder.meta(meta),
            lexer::MetricToken::Metric(sample) => builder.sample(sample),
            lexer::MetricToken::Eof => builder.eof(),
            lexer::MetricToken::Empty => Ok(builder),
        })?
        .finalize()?)
}
