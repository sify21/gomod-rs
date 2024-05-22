use crate::{Context, Directive, Location, Span, Sundry};

use nom::{
    bytes::complete::tag,
    character::complete::char,
    error::{Error, ErrorKind},
    sequence::{pair, preceded},
    Err, IResult,
};
use nom_locate::position;

use super::{
    delims0, delims1, parse_inline_comment, parse_module_path, parse_multiline_comments, quoted,
};

pub fn parse_module_directive(input: Span) -> IResult<Span, Context<Directive>> {
    let mut comments = vec![];
    let (input, multi_comments) = parse_multiline_comments(input)?;
    comments.extend(multi_comments.iter().filter_map(|i| match i {
        Sundry::Comment(c) => Some(c.fragment()),
        _ => None,
    }));
    let (input, tmp) = preceded(delims0, tag("module"))(input)?;
    let start = Location {
        line: tmp.location_line(),
        offset: tmp.location_offset(),
    };
    if let Ok((input, (path, comment))) = preceded(
        delims1,
        pair(quoted(parse_module_path), parse_inline_comment),
    )(input)
    {
        if let Sundry::Comment(c) = comment {
            comments.push(*c.fragment());
        }
        let (input, pos) = position(input)?;
        let end = Location {
            line: pos.location_line(),
            offset: pos.location_offset(),
        };
        return Ok((
            input,
            Context {
                range: (start, end),
                comments,
                value: Directive::Module {
                    module_path: path.fragment(),
                },
            },
        ));
    } else if let Ok((input, comment)) =
        preceded(pair(delims0, char('(')), parse_inline_comment)(input)
    {
        if let Sundry::Comment(c) = comment {
            comments.push(c.fragment());
        }
        let (input, multi_comments) = parse_multiline_comments(input)?;
        comments.extend(multi_comments.iter().filter_map(|i| match i {
            Sundry::Comment(c) => Some(c.fragment()),
            _ => None,
        }));
        let (input, (path, comment)) = preceded(
            delims0,
            pair(quoted(parse_module_path), parse_inline_comment),
        )(input)?;
        if let Sundry::Comment(c) = comment {
            comments.push(c.fragment());
        }
        let (input, multi_comments) = parse_multiline_comments(input)?;
        comments.extend(multi_comments.iter().filter_map(|i| match i {
            Sundry::Comment(c) => Some(c.fragment()),
            _ => None,
        }));
        let (input, comment) = preceded(pair(delims0, char(')')), parse_inline_comment)(input)?;
        if let Sundry::Comment(c) = comment {
            comments.push(c.fragment());
        }
        let (input, pos) = position(input)?;
        let end = Location {
            line: pos.location_line(),
            offset: pos.location_offset(),
        };
        return Ok((
            input,
            Context {
                range: (start, end),
                comments,
                value: Directive::Module {
                    module_path: path.fragment(),
                },
            },
        ));
    }
    Err(Err::Error(Error::new(input, ErrorKind::Alt)))
}

#[cfg(test)]
mod tests {
    use crate::{Context, Directive, Location, Span};

    use super::parse_module_directive;

    #[test]
    fn test_module() {
        let s = r#"
// heheda
// Deprecated: use *** instead.
module ( 
// abc
// def
    `rsdf/sf-f/s8._~` // inline
    // ghi
   ) // trailing
"#;
        let (input, ret) = parse_module_directive(Span::new(s)).unwrap();
        assert_eq!(*input.fragment(), "");
        assert_eq!(
            ret,
            Context {
                range: (
                    Location {
                        line: 4,
                        offset: 43,
                    },
                    Location {
                        line: 10,
                        offset: 127,
                    },
                ),
                comments: vec![
                    " heheda",
                    " Deprecated: use *** instead.",
                    " abc",
                    " def",
                    " inline",
                    " ghi",
                    " trailing"
                ],
                value: Directive::Module {
                    module_path: "rsdf/sf-f/s8._~"
                }
            }
        );
    }
}
