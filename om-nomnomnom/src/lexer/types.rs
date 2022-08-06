#[allow(unused)]
use tracing::{debug, error, info, span, trace, warn, Instrument, Level};

use nom::{
    branch::alt,
    bytes::complete::{escaped, tag, tag_no_case, take_while},
    character::complete::{one_of, satisfy},
    combinator::{not, opt, peek, value},
    sequence::preceded,
    IResult,
};

/// ```abnf
/// ; Any unicode character, except newline, double quote, and backslash
/// normal-char = %x00-09 / %x0B-21 / %x23-5B / %x5D-D7FF / %xE000-10FFFF
/// ```
fn is_normal_char(c: char) -> bool {
    let codepoint = c as u32;
    !(codepoint == 0x0A
        || codepoint == 0x22
        || codepoint == 0x5C
        || (codepoint > 0xD7FF && codepoint < 0xE000))
}

/// Recognize single "normal_char"
pub(super) fn normal_char(input: &str) -> IResult<&str, char> {
    satisfy(is_normal_char)(input)
}

/// Recognize a sequence of one or more "normal char"
pub(super) fn normal_char1(input: &str) -> IResult<&str, &str> {
    nom::InputTakeAtPosition::split_at_position1_complete(
        &input,
        |item| !is_normal_char(item),
        nom::error::ErrorKind::AlphaNumeric,
    )
}

/// abnf's SP token
pub(super) fn single_space(input: &str) -> IResult<&str, &str> {
    tag(" ")(input)
}

/// ```abnf
/// escaped-string = *escaped-char
///
/// escaped-char = normal-char
/// escaped-char =/ BS ("n" / DQUOTE / BS)
/// escaped-char =/ BS normal-char
/// ```
pub(super) fn escaped_string1(input: &str) -> IResult<&str, &str> {
    escaped(normal_char1, '\\', alt((one_of(r#""\"#), normal_char)))(input)
}

/// ```abnf
/// metricname = metricname-initial-char 0*metricname-char
///
/// metricname-char = metricname-initial-char / DIGIT
/// metricname-initial-char = ALPHA / "_" / ":"
/// ```
#[tracing::instrument]
pub(super) fn metric_name1(input: &str) -> IResult<&str, &str> {
    peek(satisfy(|c| c.is_alphabetic() || c == '_' || c == ':'))(input)?;

    take_while(|item: char| item.is_alphanumeric() || item == '_' || item == ':')(input)
}

pub(super) fn realnumber<'a>(input: &'a str) -> IResult<&str, f64> {
    not(alt((
        tag_no_case("NaN"),
        preceded(
            opt(one_of("+-")),
            alt((tag_no_case("Inf"), tag_no_case("Infinity"))),
        ),
    )))(input)?;

    nom::number::complete::double(input)
}

pub(super) fn floatlike<'a>(input: &'a str) -> IResult<&str, f64> {
    alt((
        value(f64::NAN, tag_no_case("NaN")),
        value(
            f64::NEG_INFINITY,
            preceded(tag("-"), alt((tag("Infinity"), tag("Inf")))),
        ),
        value(
            f64::INFINITY,
            preceded(opt(tag("+")), alt((tag("Infinity"), tag("Inf")))),
        ),
        nom::number::complete::double,
    ))(input)
}
