use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::char,
    error::Error,
    multi::fold_many0,
    sequence::{delimited, pair, preceded, separated_pair, tuple},
    Err, IResult, Parser,
};
use nom_locate::position;

use crate::{parser::parse_identifier, Context, Directive, Location, RetractSpec, Span, Sundry};

use super::{delims0, delims1, parse_inline_comment, parse_multiline_comments};

fn parse_retract_spec(input: Span) -> IResult<Span, Context<RetractSpec>> {
    let (input, pos) = position(input)?;
    let start = Location {
        line: pos.location_line(),
        offset: pos.location_offset(),
    };
    let (input, (version, comment)) = pair(
        alt((
            delimited(
                pair(char('['), delims0),
                separated_pair(
                    parse_identifier,
                    tuple((delims0, char(','), delims0)),
                    parse_identifier,
                ),
                pair(delims0, char(']')),
            )
            .map(|(v1, v2)| RetractSpec::Range((v1, v2))),
            parse_identifier.map(|i| RetractSpec::Version(i)),
        )),
        parse_inline_comment,
    )(input)?;
    let mut comments = vec![];
    if let Sundry::Comment(c) = comment {
        comments.push(c.into_fragment());
    }
    let (input, pos) = position(input)?;
    let end = Location {
        line: pos.location_line(),
        offset: pos.location_offset(),
    };
    Ok((
        input,
        Context {
            range: (start, end),
            comments,
            value: version,
        },
    ))
}

pub fn parse_retract_directive(input: Span) -> IResult<Span, Context<Directive>> {
    let mut comments = vec![];
    let (input, multi_comments) = parse_multiline_comments(input)?;
    comments.extend(multi_comments.into_iter().filter_map(|i| match i {
        Sundry::Comment(c) => Some(c.into_fragment()),
        _ => None,
    }));
    let (input, tmp) = preceded(delims0, tag("retract"))(input)?;
    let start = Location {
        line: tmp.location_line(),
        offset: tmp.location_offset(),
    };
    let mut specs = vec![];
    let input = if let Ok((input, spec)) = preceded(delims1, parse_retract_spec)(input) {
        specs.push(spec);
        input
    } else if let Ok((input, comment)) =
        preceded(pair(delims0, char('(')), parse_inline_comment)(input)
    {
        if let Sundry::Comment(c) = comment {
            comments.push(c.into_fragment());
        }
        let (input, ret) = fold_many0(
            pair(
                parse_multiline_comments,
                preceded(delims0, parse_retract_spec),
            ),
            Vec::new,
            |mut acc, (multi_comments, mut spec)| {
                let mut multi_comments = multi_comments
                    .into_iter()
                    .filter_map(|i| match i {
                        Sundry::Comment(c) => Some(c.into_fragment()),
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                if !multi_comments.is_empty() {
                    multi_comments.extend_from_slice(&spec.comments[..]);
                    spec.comments = multi_comments;
                }
                acc.push(spec);
                acc
            },
        )(input)?;
        specs.extend(ret.into_iter());
        let (input, multi_comments) = parse_multiline_comments(input)?;
        comments.extend(multi_comments.into_iter().filter_map(|i| match i {
            Sundry::Comment(c) => Some(c.into_fragment()),
            _ => None,
        }));
        let (input, comment) = preceded(pair(delims0, char(')')), parse_inline_comment)(input)?;
        if let Sundry::Comment(c) = comment {
            comments.push(c.into_fragment());
        }
        input
    } else {
        return Err(Err::Error(Error::new(input, nom::error::ErrorKind::Alt)));
    };
    let (input, pos) = position(input)?;
    let end = Location {
        line: pos.location_line(),
        offset: pos.location_offset(),
    };
    Ok((
        input,
        Context {
            range: (start, end),
            comments,
            value: Directive::Retract { specs },
        },
    ))
}

#[cfg(test)]
mod tests {
    use crate::{Context, Directive, Identifier, Location, RetractSpec, Span};

    use super::parse_retract_directive;

    #[test]
    fn test_retract() {
        let s = r#"
        // start retract
        retract ( // start specs
    v1.0.0 // aaa
    // bbb
    [v1.0.0, v1.9.9] // ccc
    // end specs
) // end retract
"#;
        let (input, ret) = parse_retract_directive(Span::new(s)).unwrap();
        assert_eq!("", input.into_fragment());
        assert_eq!(
            ret,
            Context {
                range: (
                    Location {
                        line: 3,
                        offset: 34
                    },
                    Location {
                        line: 9,
                        offset: 150
                    }
                ),
                comments: vec![
                    " start retract",
                    " start specs",
                    " end specs",
                    " end retract",
                ],
                value: Directive::Retract {
                    specs: vec![
                        Context {
                            range: (
                                Location {
                                    line: 4,
                                    offset: 63
                                },
                                Location {
                                    line: 5,
                                    offset: 77
                                }
                            ),
                            comments: vec![" aaa"],
                            value: RetractSpec::Version(Identifier::Raw("v1.0.0"))
                        },
                        Context {
                            range: (
                                Location {
                                    line: 6,
                                    offset: 92
                                },
                                Location {
                                    line: 7,
                                    offset: 116
                                }
                            ),
                            comments: vec![" bbb", " ccc"],
                            value: RetractSpec::Range((
                                Identifier::Raw("v1.0.0"),
                                Identifier::Raw("v1.9.9")
                            ))
                        },
                    ]
                }
            }
        );
    }
}
