# netcon-rs

A collections of tools and helper functions developed for and by NetCon
Unternehmensberatung GmbH.

## Usage

To use this library, just add the path to this repository to the dependencies
section of your `Cargo.toml`.

### Features

The library is structured in several features, that allow to keep the used
dependencies as low as possible. The following features are available:

-   `threadpool`: A struct to limit the number parallel running threads in a
    multithreaded program.

## Documentation

To build the documentation, run `cargo doc --all-features`. This builds the
documentation including all optional features.

## Tests

Since all features are turned off by default, running `cargo test` will do
nothing actually useful. To run all tests, the `--all-features` flag must be
added. Otherwise specific features can be selected by adding the with the
`--features <feature>,...` option.
