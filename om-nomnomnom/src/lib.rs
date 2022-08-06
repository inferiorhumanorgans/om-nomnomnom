//! `om-nomnomnom` is an OpenMetrics exposition parser
//!
//! Yes.

#[allow(unused)]
use tracing::{debug, error, info, span, trace, warn, Instrument, Level};

use std::collections::HashMap;

/// Tokenizes an exposition document
pub mod lexer;

/// Parses the tokens into a more user friendly format and performs additional validation.
pub mod parser;

#[cfg(test)]
mod test;

/// Indicates that an error occurred while processing an exposition document
#[derive(thiserror::Error, Debug)]
pub enum OmError {
    #[error(transparent)]
    Parse(#[from] parser::ParseError),

    #[error("lexer failed {0}")]
    LexError(String),

    #[error("unknown error")]
    Unknown,
}

/// Parses an exposition document into a [`HashMap`] containing an entry per [`MetricFamily`](crate::parser::MetricFamily).
pub fn parse<'a>(data: &'a str) -> Result<HashMap<&'a str, parser::MetricFamily<'a>>, OmError> {
    let (_, tokens) = lexer::exposition(data).map_err(|e| OmError::LexError(e.to_string()))?;
    let metric_families = parser::parse(tokens)?;
    Ok(metric_families)
}
