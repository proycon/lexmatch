# Lexmatch

This is a simple lexicon matching tool that, given a lexicon of words or phrases, identifies all matches in a given target text.
It can be used compute a frequency list for a lexicon, on a target corpus.

The implementation uses suffix arrays. The text must be plain-text UTF-8 and is limited to 2^32 bytes (about 4GB).


## Installation

You can build and install the latest stable release using Rust's package manager:

```
cargo install lexmatch
```

or if you want the development version after cloning this repository:

```
cargo install --path .
```

No cargo/rust on your system yet? Do ``sudo apt install cargo`` on Debian/ubuntu based systems, ``brew install rust`` on mac, or use [rustup](https://rustup.rs/).

## Usage

See ``lexmatch --help``.

Simple example:

```
$ lexmatch --lexicon lexicon.lst --text corpus.txt
```

The lexicon must be plain-text UTF-8 containing one entry per line, an entry need not be a single word and is not constrained in length.
