use nom::{
    bytes::complete::tag,
    sequence::{delimited, tuple},
    IResult,
};
use nom_locate::position;

use crate::{Context, Directive, Location, Span, Sundry};

use super::{delims0, delims1, parse_identifier, parse_inline_comment, parse_multiline_comments};

pub fn parse_go_directive(input: Span) -> IResult<Span, Context<Directive>> {
    let mut comments = vec![];
    let (input, multi_comments) = parse_multiline_comments(input)?;
    comments.extend(multi_comments.iter().filter_map(|i| match i {
        Sundry::Comment(c) => Some(c.fragment()),
        _ => None,
    }));
    let (input, start) = position(input)?;
    let (input, (_, ver, comment)) = tuple((
        delimited(delims0, tag("go"), delims1),
        parse_identifier,
        parse_inline_comment,
    ))(input)?;
    if let Sundry::Comment(c) = comment {
        comments.push(*c.fragment());
    }
    let (input, end) = position(input)?;
    Ok((
        input,
        Context {
            comments,
            range: (
                Location {
                    line: start.location_line(),
                    offset: start.location_offset(),
                },
                Location {
                    line: end.location_line(),
                    offset: end.location_offset(),
                },
            ),
            value: Directive::Go { version: ver },
        },
    ))
}

#[cfg(test)]
mod tests {
    use crate::{Context, Directive, Identifier, Location, Span};

    use super::parse_go_directive;

    #[test]
    fn test_go() {
        let s = r#"
// heheda
go "1.4.5\"rc1" // inline
"#;
        let (input, ret) = parse_go_directive(Span::new(s)).unwrap();
        assert_eq!("", *input.fragment());
        assert_eq!(
            ret,
            Context {
                range: (
                    Location {
                        line: 3,
                        offset: 11
                    },
                    Location {
                        line: 4,
                        offset: 37
                    }
                ),
                comments: vec![" heheda", " inline"],
                value: Directive::Go {
                    version: Identifier::Interpreted("1.4.5\"rc1".to_string())
                }
            }
        )
    }
}
