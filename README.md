# gomod-rs
A [go.mod](https://go.dev/ref/mod#go-mod-file) file parser with location information.

Implemented using [nom](https://github.com/rust-bakery/nom) and [nom\_locate](https://github.com/fflorent/nom_locate).

**No string copy/clone during parsing.** (except for [interpreted strings](https://go.dev/ref/mod#go-mod-file-lexical))

## Example Usage
```rust
```
