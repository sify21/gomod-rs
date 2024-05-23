use crate::{Identifier, Span, Sundry};

use super::GoMod;
use nom::{
    branch::alt,
    bytes::complete::{escaped_transform, is_a, is_not, tag, take, take_while, take_while1},
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

// include trailing newline or eof
fn parse_inline_comment(input: Span) -> IResult<Span, Sundry> {
    alt((
        delimited(
            pair(delims0, tag("//")),
            take_while(|c| c != '\n'),
            char('\n'),
        )
        .map(|i| Sundry::Comment(i)),
        terminated(delims0, char('\n')).map(|i| Sundry::Empty(i)),
        delimited(pair(delims0, tag("//")), take_while(|c| c != '\n'), eof)
            .map(|i| Sundry::Comment(i)),
        terminated(delims1, eof).map(|i| Sundry::Empty(i)),
        eof.map(|_| Sundry::EOF),
    ))(input)
}
fn parse_multiline_comments(input: Span) -> IResult<Span, Vec<Sundry>> {
    fold_many0(
        verify(parse_inline_comment, |i| !matches!(i, &Sundry::EOF)),
        Vec::new,
        |mut acc, item| {
            acc.push(item);
            acc
        },
    )(input)
}

// https://go.dev/ref/mod#go-mod-file-lexical
//
// Identifiers are sequences of non-whitespace characters, such as module paths or semantic versions.
//
// Strings are quoted sequences of characters. There are two kinds of strings: interpreted strings
// beginning and ending with quotation marks (", U+0022) and raw strings beginning and ending with
// grave accents (`, U+0060). Interpreted strings may contain escape sequences consisting of a backslash
// (\, U+005C) followed by another character. An escaped quotation mark (\") does not terminate an
// interpreted string. The unquoted value of an interpreted string is the sequence of characters between
// quotation marks with each escape sequence replaced by the character following the backslash (for
// example, \" is replaced by ", \n is replaced by n). In contrast, the unquoted value of a raw string is
// simply the sequence of characters between grave accents; backslashes have no special meaning within raw strings.
//
// Identifiers and strings are interchangeable in the go.mod grammar.
fn parse_identifier(input: Span) -> IResult<Span, Identifier> {
    alt((
        parse_raw_string.map(|i| Identifier::Raw(i.into_fragment())),
        parse_interpreted_string.map(|i| Identifier::Interpreted(i)),
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
        )
        .map(|i: Span| Identifier::Raw(i.into_fragment())),
    ))(input)
}
fn parse_interpreted_string(input: Span) -> IResult<Span, String> {
    delimited(
        char('"'),
        escaped_transform(is_not("\n\r\t\u{08}\u{0c}\"\\"), '\\', take(1u8)),
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
    let (input, _) = parse_multiline_comments(input)?;
    Ok((input, ret))
}

#[cfg(test)]
mod tests {
    use crate::{
        Context, Directive, Identifier, Location, ReplaceSpec, Replacement, RetractSpec, Span,
        Sundry,
    };

    use super::{parse_gomod, parse_identifier, parse_inline_comment};

    #[test]
    fn test_inline_comment() {
        for s in ["// sdfsfs\n", "// sdfsfs", "  // sdfsfs\n", "  // sdfsfs"] {
            let (input, ret) = parse_inline_comment(Span::new(s)).unwrap();
            assert!(matches!(ret, Sundry::Comment(i) if i.into_fragment() == " sdfsfs"));
            assert_eq!(input.into_fragment(), "");
        }
        for s in ["//", "//\n", "  //", "  //\n"] {
            let (input, ret) = parse_inline_comment(Span::new(s)).unwrap();
            assert_eq!(input.into_fragment(), "");
            assert!(matches!(ret, Sundry::Comment(i) if i.fragment().is_empty()));
        }
        for s in ["  ", "\n", "  \n"] {
            let (input, ret) = parse_inline_comment(Span::new(s)).unwrap();
            assert_eq!(input.into_fragment(), "");
            assert!(matches!(ret, Sundry::Empty(_)));
        }
        let (input, ret) = parse_inline_comment(Span::new("")).unwrap();
        assert_eq!(input.into_fragment(), "");
        assert!(matches!(ret, Sundry::EOF));
    }

    #[test]
    fn test_identifier() {
        for s in [r#"`v1.0.0`"#, "v1.0.0", r#""v1.0.0""#] {
            let (input, ret) = parse_identifier(Span::new(s)).unwrap();
            assert_eq!(&ret as &str, "v1.0.0");
            assert_eq!(input.into_fragment(), "");
        }
        let (input, ret) = parse_identifier(Span::new(r#""abc\n\r\f\"dd""#)).unwrap();
        assert_eq!(&ret as &str, "abcnrf\"dd");
        assert_eq!(input.into_fragment(), "");
    }

    #[test]
    fn test_gomod() {
        let s = r#"
module example.com/my/thing

go 1.12

require (
    example.com/other/thing v1.0.2
    example.com/new/thing/v2 v2.3.4
)

exclude example.com/old/thing v1.2.3
replace example.com/bad/thing v1.4.5 => example.com/good/thing v1.4.5
retract [v1.9.0, v1.9.5]"#;
        let (input, ret) = parse_gomod(Span::new(s)).unwrap();
        assert_eq!(input.into_fragment(), "");
        assert_eq!(
            ret,
            vec![
                Context {
                    range: (
                        Location { line: 2, offset: 1 },
                        Location {
                            line: 3,
                            offset: 29
                        }
                    ),
                    comments: vec![],
                    value: Directive::Module {
                        module_path: "example.com/my/thing"
                    }
                },
                Context {
                    range: (
                        Location {
                            line: 4,
                            offset: 30
                        },
                        Location {
                            line: 5,
                            offset: 38
                        }
                    ),
                    comments: vec![],
                    value: Directive::Go {
                        version: Identifier::Raw("1.12")
                    }
                },
                Context {
                    range: (
                        Location {
                            line: 6,
                            offset: 39
                        },
                        Location {
                            line: 10,
                            offset: 122
                        }
                    ),
                    comments: vec![],
                    value: Directive::Require {
                        specs: vec![
                            Context {
                                range: (
                                    Location {
                                        line: 7,
                                        offset: 53
                                    },
                                    Location {
                                        line: 8,
                                        offset: 84
                                    }
                                ),
                                comments: vec![],
                                value: ("example.com/other/thing", Identifier::Raw("v1.0.2"))
                            },
                            Context {
                                range: (
                                    Location {
                                        line: 8,
                                        offset: 88
                                    },
                                    Location {
                                        line: 9,
                                        offset: 120
                                    }
                                ),
                                comments: vec![],
                                value: ("example.com/new/thing/v2", Identifier::Raw("v2.3.4"))
                            }
                        ]
                    }
                },
                Context {
                    range: (
                        Location {
                            line: 11,
                            offset: 123
                        },
                        Location {
                            line: 12,
                            offset: 160
                        }
                    ),
                    comments: vec![],
                    value: Directive::Exclude {
                        specs: vec![Context {
                            range: (
                                Location {
                                    line: 11,
                                    offset: 131
                                },
                                Location {
                                    line: 12,
                                    offset: 160
                                }
                            ),
                            comments: vec![],
                            value: ("example.com/old/thing", Identifier::Raw("v1.2.3"))
                        }]
                    }
                },
                Context {
                    range: (
                        Location {
                            line: 12,
                            offset: 160
                        },
                        Location {
                            line: 13,
                            offset: 230
                        }
                    ),
                    comments: vec![],
                    value: Directive::Replace {
                        specs: vec![Context {
                            range: (
                                Location {
                                    line: 12,
                                    offset: 168
                                },
                                Location {
                                    line: 13,
                                    offset: 230
                                }
                            ),
                            comments: vec![],
                            value: ReplaceSpec {
                                module_path: "example.com/bad/thing",
                                version: Some(Identifier::Raw("v1.4.5")),
                                replacement: Replacement::Module((
                                    "example.com/good/thing",
                                    Identifier::Raw("v1.4.5")
                                ))
                            }
                        }]
                    }
                },
                Context {
                    range: (
                        Location {
                            line: 13,
                            offset: 230
                        },
                        Location {
                            line: 13,
                            offset: 254
                        }
                    ),
                    comments: vec![],
                    value: Directive::Retract {
                        specs: vec![Context {
                            range: (
                                Location {
                                    line: 13,
                                    offset: 238
                                },
                                Location {
                                    line: 13,
                                    offset: 254
                                }
                            ),
                            comments: vec![],
                            value: RetractSpec::Range((
                                Identifier::Raw("v1.9.0"),
                                Identifier::Raw("v1.9.5")
                            ))
                        }]
                    }
                }
            ]
        );
    }
}
