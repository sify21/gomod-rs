# go.mod Parser
A [go.mod](https://go.dev/ref/mod#go-mod-file) file parser with location information.

Implemented using [nom](https://github.com/rust-bakery/nom) and [nom\_locate](https://github.com/fflorent/nom_locate).

A `go.mod` file must be read into a string beforehand. 

No string copy/clone during parsing.
