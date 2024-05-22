use crate::{Span, Sundry};

use super::GoMod;
use nom::{
    branch::alt,
    bytes::complete::{escaped, is_a, is_not, tag, take, take_till, take_while, take_while1},
    character::{
        complete::{char, one_of},
        is_alphanumeric,
    },
    combinator::{eof, peek, recognize, verify},
    error::ParseError,
    multi::{fold_many0, fold_many1, many0, many_till},
    sequence::{delimited, pair, preceded, terminated},
    IResult, Parser,
};

mod exclude_directive;
mod go_directive;
mod godebug_directive;
mod module_directive;
mod replace_directive;
mod require_directive;
mod retract_directive;
mod toolchain_directive;

fn delims0(input: Span) -> IResult<Span, Span> {
    take_while(|c| c == ' ' || c == '\t' || c == '\r')(input)
}
fn delims1(input: Span) -> IResult<Span, Span> {
    is_a(" \t\r")(input)
}
fn quoted<'a, E: ParseError<Span<'a>>, F>(
    f: F,
) -> impl FnMut(Span<'a>) -> IResult<Span<'a>, Span<'a>, E>
where
    F: Parser<Span<'a>, Span<'a>, E> + Copy,
{
    alt((
        f,
        delimited(char('"'), f, char('"')),
        delimited(char('`'), f, char('`')),
    ))
}

fn parse_empty_line(input: Span) -> IResult<Span, Sundry> {
    terminated(delims0, char('\n'))
        .map(|i| Sundry::Empty(i))
        .parse(input)
}
// include trailing newline or eof
fn parse_inline_comment(input: Span) -> IResult<Span, Sundry> {
    terminated(
        alt((
            preceded(pair(delims0, tag("//")), take_while(|c| c != '\n'))
                .map(|i| Sundry::Comment(i)),
            delims0.map(|i| Sundry::Empty(i)),
        )),
        alt((tag("\n"), eof)),
    )
    .parse(input)
}
// with leading spaces and trailing newline
fn parse_oneline_comment(input: Span) -> IResult<Span, Sundry> {
    delimited(
        preceded(delims0, tag("//")),
        take_till(|c| c == '\n'),
        char('\n'),
    )
    .map(|comment| Sundry::Comment(comment))
    .parse(input)
}
fn parse_multiline_comments(input: Span) -> IResult<Span, Vec<Sundry>> {
    fold_many0(
        alt((parse_empty_line, parse_oneline_comment)),
        Vec::new,
        |mut acc: Vec<_>, item| {
            acc.push(item);
            acc
        },
    )(input)
}

fn parse_identifier(input: Span) -> IResult<Span, Span> {
    alt((
        parse_raw_string,
        parse_interpreted_string,
        verify(
            recognize(many_till(
                take(1usize),
                peek(alt((
                    tag("//"),
                    tag("=>"),
                    recognize(one_of(" \t\n\r(),[]")),
                    eof,
                ))),
            )),
            |i: &Span| !i.is_empty(),
        ),
    ))(input)
}
fn parse_interpreted_string(input: Span) -> IResult<Span, Span> {
    delimited(
        char('"'),
        escaped(is_not("\n\r\t\u{08}\u{0c}\"\\"), '\\', one_of(r#"nrtbf"\"#)),
        char('"'),
    )(input)
}
fn parse_raw_string(input: Span) -> IResult<Span, Span> {
    delimited(char('`'), is_not("`\n"), char('`'))(input)
}

fn parse_module_path_fragment(input: Span) -> IResult<Span, Span> {
    take_while1(|c| is_alphanumeric(c as u8) || c == '-' || c == '_' || c == '.' || c == '~')(input)
}
fn parse_module_path(input: Span) -> IResult<Span, Span> {
    recognize(pair(
        parse_module_path_fragment,
        many0(preceded(char('/'), parse_module_path_fragment)),
    ))(input)
}

pub fn parse_gomod(input: Span) -> IResult<Span, GoMod> {
    let (input, ret) = fold_many1(
        alt((
            go_directive::parse_go_directive,
            module_directive::parse_module_directive,
            exclude_directive::parse_exclude_directive,
            godebug_directive::parse_godebug_directive,
            replace_directive::parse_replace_directive,
            require_directive::parse_require_directive,
            retract_directive::parse_retract_directive,
            toolchain_directive::parse_toolchain_directive,
        )),
        Vec::new,
        |mut acc, directive| {
            acc.push(directive);
            acc
        },
    )(input)?;
    let (input, _) = pair(parse_multiline_comments, eof)(input)?;
    Ok((input, ret))
}

#[cfg(test)]
mod tests {
    use crate::{Span, Sundry};

    use super::{parse_identifier, parse_inline_comment};

    #[test]
    fn test_inline_comment() {
        for s in ["// sdfsfs\n", "// sdfsfs"] {
            let (input, ret) = parse_inline_comment(Span::new(s)).unwrap();
            assert!(matches!(ret, Sundry::Comment(i) if i.into_fragment() == " sdfsfs"));
            assert_eq!(input.into_fragment(), "");
        }
    }

    #[test]
    fn test_identifier() {
        for s in [r#"`v1.0.0`"#, "v1.0.0", r#""v1.0.0""#] {
            let (input, ret) = parse_identifier(Span::new(s)).unwrap();
            assert_eq!(ret.into_fragment(), "v1.0.0");
            assert_eq!(input.into_fragment(), "");
        }
    }
}
