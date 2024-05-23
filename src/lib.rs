use std::ops::Deref;

use nom::{error::Error, Err};
use nom_locate::LocatedSpan;

mod parser;

type Span<'a> = LocatedSpan<&'a str>;

#[derive(Debug)]
pub enum Sundry<'a> {
    Comment(Span<'a>),
    Empty(Span<'a>),
    EOF,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Identifier<'a> {
    Raw(&'a str),
    Interpreted(String),
}

impl Deref for Identifier<'_> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Raw(s) => s,
            Self::Interpreted(s) => s.as_str(),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum RetractSpec<'a> {
    Version(Identifier<'a>),
    Range((Identifier<'a>, Identifier<'a>)),
}

#[derive(Debug, PartialEq, Eq)]
pub struct ReplaceSpec<'a> {
    pub module_path: &'a str,
    pub version: Option<Identifier<'a>>,
    pub replacement: Replacement<'a>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Replacement<'a> {
    FilePath(Identifier<'a>),
    Module((&'a str, Identifier<'a>)),
}

// comments on directive includes preceding-line comments and same-line comment
#[derive(Debug, PartialEq, Eq)]
pub enum Directive<'a> {
    Module {
        module_path: &'a str,
    },
    Go {
        version: Identifier<'a>,
    },
    Require {
        specs: Vec<Context<'a, (&'a str, Identifier<'a>)>>,
    },
    Toolchain {
        name: Identifier<'a>,
    },
    Godebug {
        specs: Vec<Context<'a, (&'a str, &'a str)>>,
    },
    Replace {
        specs: Vec<Context<'a, ReplaceSpec<'a>>>,
    },
    Exclude {
        specs: Vec<Context<'a, (&'a str, Identifier<'a>)>>,
    },
    Retract {
        specs: Vec<Context<'a, RetractSpec<'a>>>,
    },
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Location {
    pub line: u32,
    pub offset: usize,
}

pub type Range = (Location, Location);

#[derive(Debug, PartialEq, Eq)]
pub struct Context<'a, T: 'a> {
    pub range: Range,
    pub comments: Vec<&'a str>,
    pub value: T,
}

pub type GoMod<'a> = Vec<Context<'a, Directive<'a>>>;

/// Return an error indicating (line, offset)
pub fn parse_gomod(text: &str) -> Result<GoMod, Err<Error<(u32, usize)>>> {
    let (_, ret) = parser::parse_gomod(Span::new(text))
        .map_err(|e| e.map_input(|i| (i.location_line(), i.location_offset())))?;
    Ok(ret)
}
