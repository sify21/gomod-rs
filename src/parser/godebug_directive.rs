use nom::{
    bytes::complete::{is_not, tag},
    character::complete::char,
    error::Error,
    multi::fold_many0,
    sequence::{delimited, pair, preceded},
    Err, IResult,
};
use nom_locate::position;

use crate::{parser::parse_multiline_comments, Context, Directive, Location, Span, Sundry};

use super::{delims0, delims1, parse_inline_comment, quoted};

fn parse_godebug_chars(input: Span) -> IResult<Span, Span> {
    is_not(" \t\r\n,\"'`=")(input)
}

fn parse_godebug_spec(input: Span) -> IResult<Span, Context<(&str, &str)>> {
    let (input, pos) = position(input)?;
    let start = Location {
        line: pos.location_line(),
        offset: pos.location_offset(),
    };
    let (input, key) = quoted(parse_godebug_chars)(input)?;
    let (input, (value, comment)) = preceded(
        delimited(delims0, char('='), delims0),
        pair(quoted(parse_godebug_chars), parse_inline_comment),
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
            value: (key.into_fragment(), value.into_fragment()),
        },
    ))
}

pub fn parse_godebug_directive(input: Span) -> IResult<Span, Context<Directive>> {
    let mut comments = vec![];
    let (input, multi_comments) = parse_multiline_comments(input)?;
    comments.extend(multi_comments.into_iter().filter_map(|i| match i {
        Sundry::Comment(c) => Some(c.into_fragment()),
        _ => None,
    }));
    let (input, tmp) = preceded(delims0, tag("godebug"))(input)?;
    let start = Location {
        line: tmp.location_line(),
        offset: tmp.location_offset(),
    };
    let mut specs = vec![];
    let input = if let Ok((input, spec)) = preceded(delims1, parse_godebug_spec)(input) {
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
                preceded(delims0, parse_godebug_spec),
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
        let (input, comment) = preceded(preceded(delims0, char(')')), parse_inline_comment)(input)?;
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
            value: Directive::Godebug { specs },
        },
    ))
}

#[cfg(test)]
mod tests {
    use crate::{Context, Directive, Location, Span};

    use super::parse_godebug_directive;

    #[test]
    fn test_godebug() {
        let s = r#"
        // hehe
        // start godebug
        godebug ( // start specs
        // abc
    //
    "panicnil"=1 // spec1
    // ghi

    asynctimerchan=`0` // spec2
    // jkl
 ) // end godebug
"#;
        let (input, ret) = parse_godebug_directive(Span::new(s)).unwrap();
        assert_eq!("", input.into_fragment());
        assert_eq!(
            ret,
            Context {
                range: (
                    Location {
                        line: 4,
                        offset: 50
                    },
                    Location {
                        line: 13,
                        offset: 196
                    }
                ),
                comments: vec![
                    " hehe",
                    " start godebug",
                    " start specs",
                    " jkl",
                    " end godebug"
                ],
                value: Directive::Godebug {
                    specs: vec![
                        Context {
                            range: (
                                Location {
                                    line: 7,
                                    offset: 101
                                },
                                Location {
                                    line: 8,
                                    offset: 123
                                }
                            ),
                            comments: vec![" abc", "", " spec1"],
                            value: ("panicnil", "1")
                        },
                        Context {
                            range: (
                                Location {
                                    line: 10,
                                    offset: 139
                                },
                                Location {
                                    line: 11,
                                    offset: 167
                                }
                            ),
                            comments: vec![" ghi", " spec2"],
                            value: ("asynctimerchan", "0")
                        },
                    ]
                }
            }
        );
    }
}
