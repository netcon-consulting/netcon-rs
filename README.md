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
