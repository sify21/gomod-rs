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

use crate::{
    parser::{parse_identifier, parse_module_path},
    Context, Directive, Location, ReplaceSpec, Replacement, Span, Sundry,
};

use super::{delims0, delims1, parse_inline_comment, parse_multiline_comments, quoted};

fn parse_replace_spec(input: Span) -> IResult<Span, Context<ReplaceSpec>> {
    let (input, pos) = position(input)?;
    let start = Location {
        line: pos.location_line(),
        offset: pos.location_offset(),
    };
    let (input, path) = quoted(parse_module_path)(input)?;
    let (input, version) = alt((
        delimited(delims0, tag("=>"), delims0).map(|_| None),
        delimited(
            delims1,
            parse_identifier,
            tuple((delims0, tag("=>"), delims0)),
        )
        .map(|i| Some(i)),
    ))(input)?;
    let (input, (replacement, comment)) = pair(
        alt((
            separated_pair(quoted(parse_module_path), delims1, parse_identifier)
                .map(|(p, v)| Replacement::Module((p.into_fragment(), v.into_fragment()))),
            parse_identifier.map(|i| Replacement::FilePath(i.into_fragment())),
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
            value: ReplaceSpec {
                module_path: path.into_fragment(),
                version: version.map(|i| i.into_fragment()),
                replacement,
            },
        },
    ))
}

pub fn parse_replace_directive(input: Span) -> IResult<Span, Context<Directive>> {
    let mut comments = vec![];
    let (input, multi_comments) = parse_multiline_comments(input)?;
    comments.extend(multi_comments.into_iter().filter_map(|i| match i {
        Sundry::Comment(c) => Some(c.into_fragment()),
        _ => None,
    }));
    let (input, tmp) = preceded(delims0, tag("replace"))(input)?;
    let start = Location {
        line: tmp.location_line(),
        offset: tmp.location_offset(),
    };
    let mut specs = vec![];
    let input = if let Ok((input, spec)) = preceded(delims1, parse_replace_spec)(input) {
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
                preceded(delims0, parse_replace_spec),
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
            value: Directive::Replace { specs },
        },
    ))
}

#[cfg(test)]
mod tests {
    use crate::{Context, Directive, Location, ReplaceSpec, Replacement, Span};

    use super::{parse_replace_directive, parse_replace_spec};

    #[test]
    fn test_replace_spec() {
        let s = "golang.org/x/net v1.2.3 => example.com/fork/net v1.4.5 // sfsdff";
        let (input, ret) = parse_replace_spec(Span::new(s)).unwrap();
        assert_eq!("", input.into_fragment());
        assert_eq!(
            ret,
            Context {
                range: (
                    Location { line: 1, offset: 0 },
                    Location {
                        line: 1,
                        offset: 64
                    }
                ),
                comments: vec![" sfsdff"],
                value: ReplaceSpec {
                    module_path: "golang.org/x/net",
                    version: Some("v1.2.3"),
                    replacement: Replacement::Module(("example.com/fork/net", "v1.4.5"))
                }
            }
        );
    }

    #[test]
    fn test_replace() {
        let s = r#"
        // start replace
        replace ( // start specs
    golang.org/x/net v1.2.3 => example.com/fork/net v1.4.5 //aa
    // bb
    golang.org/x/net => example.com/fork/net v1.4.5 // bbb
    
    golang.org/x/net v1.2.3 => ./fork/net //cc
    golang.org/x/net => ./fork/net //dd
    // trailing comments
) // end specs"#;
        let (input, ret) = parse_replace_directive(Span::new(s)).unwrap();
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
                        line: 11,
                        offset: 323
                    }
                ),
                comments: vec![
                    " start replace",
                    " start specs",
                    " trailing comments",
                    " end specs",
                ],
                value: Directive::Replace {
                    specs: vec![
                        Context {
                            range: (
                                Location {
                                    line: 4,
                                    offset: 63
                                },
                                Location {
                                    line: 5,
                                    offset: 123
                                }
                            ),
                            comments: vec!["aa"],
                            value: ReplaceSpec {
                                module_path: "golang.org/x/net",
                                version: Some("v1.2.3"),
                                replacement: Replacement::Module((
                                    "example.com/fork/net",
                                    "v1.4.5"
                                ))
                            }
                        },
                        Context {
                            range: (
                                Location {
                                    line: 6,
                                    offset: 137
                                },
                                Location {
                                    line: 7,
                                    offset: 192
                                }
                            ),
                            comments: vec![" bb", " bbb"],
                            value: ReplaceSpec {
                                module_path: "golang.org/x/net",
                                version: None,
                                replacement: Replacement::Module((
                                    "example.com/fork/net",
                                    "v1.4.5"
                                ))
                            }
                        },
                        Context {
                            range: (
                                Location {
                                    line: 8,
                                    offset: 201
                                },
                                Location {
                                    line: 9,
                                    offset: 244
                                }
                            ),
                            comments: vec!["cc"],
                            value: ReplaceSpec {
                                module_path: "golang.org/x/net",
                                version: Some("v1.2.3"),
                                replacement: Replacement::FilePath("./fork/net")
                            }
                        },
                        Context {
                            range: (
                                Location {
                                    line: 9,
                                    offset: 248
                                },
                                Location {
                                    line: 10,
                                    offset: 284
                                }
                            ),
                            comments: vec!["dd"],
                            value: ReplaceSpec {
                                module_path: "golang.org/x/net",
                                version: None,
                                replacement: Replacement::FilePath("./fork/net")
                            }
                        },
                    ]
                }
            }
        );
    }
}
