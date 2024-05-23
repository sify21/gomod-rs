use nom::{
    bytes::complete::tag,
    character::complete::char,
    error::Error,
    multi::fold_many0,
    sequence::{pair, preceded, tuple},
    Err, IResult,
};
use nom_locate::position;

use crate::{
    parser::{parse_identifier, parse_module_path},
    Context, Directive, Identifier, Location, Span, Sundry,
};

use super::{delims0, delims1, parse_inline_comment, parse_multiline_comments, quoted};

fn parse_require_spec(input: Span) -> IResult<Span, Context<(&str, Identifier)>> {
    let (input, pos) = position(input)?;
    let start = Location {
        line: pos.location_line(),
        offset: pos.location_offset(),
    };
    let (input, (path, version, comment)) = tuple((
        quoted(parse_module_path),
        preceded(delims1, parse_identifier),
        parse_inline_comment,
    ))(input)?;
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
            value: (path.into_fragment(), version),
        },
    ))
}

pub fn parse_require_directive(input: Span) -> IResult<Span, Context<Directive>> {
    let mut comments = vec![];
    let (input, multi_comments) = parse_multiline_comments(input)?;
    comments.extend(multi_comments.into_iter().filter_map(|i| match i {
        Sundry::Comment(c) => Some(c.into_fragment()),
        _ => None,
    }));
    let (input, tmp) = preceded(delims0, tag("require"))(input)?;
    let start = Location {
        line: tmp.location_line(),
        offset: tmp.location_offset(),
    };
    let mut specs = vec![];
    let input = if let Ok((input, spec)) = preceded(delims1, parse_require_spec)(input) {
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
                preceded(delims0, parse_require_spec),
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
            value: Directive::Require { specs },
        },
    ))
}

#[cfg(test)]
mod tests {
    use crate::{Context, Directive, Identifier, Location, Span};

    use super::parse_require_directive;

    #[test]
    fn test_require() {
        let s = r#"
        // start require
        require ( // start specs
    golang.org/x/crypto v1.4.5 // indirect
    // mm
    golang.org/x/text v1.6.7
    // end specs
 ) // end require
"#;
        let (input, ret) = parse_require_directive(Span::new(s)).unwrap();
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
                        offset: 176
                    }
                ),
                comments: vec![
                    " start require",
                    " start specs",
                    " end specs",
                    " end require",
                ],
                value: Directive::Require {
                    specs: vec![
                        Context {
                            range: (
                                Location {
                                    line: 4,
                                    offset: 63
                                },
                                Location {
                                    line: 5,
                                    offset: 102
                                }
                            ),
                            comments: vec![" indirect"],
                            value: ("golang.org/x/crypto", Identifier::Raw("v1.4.5"))
                        },
                        Context {
                            range: (
                                Location {
                                    line: 6,
                                    offset: 116
                                },
                                Location {
                                    line: 7,
                                    offset: 141
                                }
                            ),
                            comments: vec![" mm"],
                            value: ("golang.org/x/text", Identifier::Raw("v1.6.7"))
                        },
                    ]
                }
            }
        );
    }
}
