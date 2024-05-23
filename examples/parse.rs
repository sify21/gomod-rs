use gomod_rs::{parse_gomod, Context, Directive};

fn main() {
    let mod_file = std::env::args().nth(1).expect("specify a go.mod filepath");
    let contents = std::fs::read_to_string(mod_file).unwrap();
    let gomod = parse_gomod(&contents).unwrap();
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
                    "Defined a dependency {{name: {}, version: {}}} at line {}, fragment: {}",
                    spec.value.0,
                    &spec.value.1 as &str,
                    spec.range.0.line,
                    &contents[spec.range.0.offset..spec.range.1.offset]
                );
            });
        });
}
