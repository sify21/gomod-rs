use nom_locate::LocatedSpan;

mod parser;

pub use parser::parse_gomod;

pub type Span<'a> = LocatedSpan<&'a str>;

#[derive(Debug)]
pub enum Sundry<'a> {
    Comment(Span<'a>),
    Empty(Span<'a>),
}

#[derive(Debug, PartialEq, Eq)]
pub enum RetractSpec<'a> {
    Version(&'a str),
    Range((&'a str, &'a str)),
}

#[derive(Debug, PartialEq, Eq)]
pub struct ReplaceSpec<'a> {
    pub module_path: &'a str,
    pub version: Option<&'a str>,
    pub replacement: Replacement<'a>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Replacement<'a> {
    FilePath(&'a str),
    Module((&'a str, &'a str)),
}

// comments on directive includes preceding-line comments and same-line comment
#[derive(Debug, PartialEq, Eq)]
pub enum Directive<'a> {
    Module {
        module_path: &'a str,
    },
    Go {
        version: &'a str,
    },
    Require {
        specs: Vec<Context<'a, (&'a str, &'a str)>>,
    },
    Toolchain {
        name: &'a str,
    },
    Godebug {
        specs: Vec<Context<'a, (&'a str, &'a str)>>,
    },
    Replace {
        specs: Vec<Context<'a, ReplaceSpec<'a>>>,
    },
    Exclude {
        specs: Vec<Context<'a, (&'a str, &'a str)>>,
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
