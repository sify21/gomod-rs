use nom::{
    bytes::complete::tag,
    sequence::{delimited, pair, preceded},
    IResult,
};
use nom_locate::position;

use crate::{Context, Directive, Location, Span, Sundry};

use super::{delims0, delims1, parse_identifier, parse_inline_comment, parse_multiline_comments};

pub fn parse_toolchain_directive(input: Span) -> IResult<Span, Context<Directive>> {
    let mut comments = vec![];
    let (input, multi_comments) = parse_multiline_comments(input)?;
    comments.extend(multi_comments.iter().filter_map(|i| match i {
        Sundry::Comment(c) => Some(c.fragment()),
        _ => None,
    }));
    let (input, start) = position(input)?;
    let (input, (name, comment)) = preceded(
        delimited(delims0, tag("toolchain"), delims1),
        pair(parse_identifier, parse_inline_comment),
    )(input)?;
    let (input, end) = position(input)?;
    if let Sundry::Comment(c) = comment {
        comments.push(*c.fragment());
    }
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
            value: Directive::Toolchain { name },
        },
    ))
}

#[cfg(test)]
mod tests {
    use crate::{Context, Directive, Identifier, Location, Span};

    use super::parse_toolchain_directive;

    #[test]
    fn test_toolchain() {
        let s = r#"
// heheda
toolchain go1.21.3+auto // inline
"#;
        let (input, ret) = parse_toolchain_directive(Span::new(s)).unwrap();
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
                        offset: 45
                    }
                ),
                comments: vec![" heheda", " inline"],
                value: Directive::Toolchain {
                    name: Identifier::Raw("go1.21.3+auto")
                }
            }
        )
    }
}
