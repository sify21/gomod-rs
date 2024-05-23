# gomod-rs
A [go.mod](https://go.dev/ref/mod#go-mod-file) file parser with location information.

Implemented using [nom](https://github.com/rust-bakery/nom) and [nom\_locate](https://github.com/fflorent/nom_locate).

**No string copy/clone during parsing.**

(except for [interpreted strings](https://go.dev/ref/mod#go-mod-file-lexical), which yield `Identifier::Interpreted(String)` type)

## Example Usage
Here is an example printing all requirements defined in a go.mod file, along with their locations and related contents.
```rust
use gomod_rs::{parse_gomod, Context, Directive};

let contents = r#"
"#;
let gomod = parse_gomod(&contents)?;
gomod
    .iter()
    .filter_map(|i| match i {
        Context {
            value: Directive::Require { specs },
            ..
        } => Some(specs),
        _ => None,
    })
    .for_each(|require_specs| {
        require_specs.iter().for_each(|spec| {
            println!(
                "Requirement {{name: {}, version: {}}} at line {}, fragment: {}",
                spec.value.0,
                &spec.value.1 as &str,
                spec.range.0.line,
                &contents[spec.range.0.offset..spec.range.1.offset]
            );
        });
    });
```
