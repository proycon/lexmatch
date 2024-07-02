# Lexmatch

This is a simple lexicon matching tool that, given a lexicon of words or
phrases, identifies all matches in a given target text, returning their exact
positions. It can be used compute a frequency list for a lexicon, on a target
corpus.

The implementation uses suffix arrays or hash tables. The text must be
plain-text UTF-8. For the former implementation (default), it is limited to
2^32 bytes (about 4GB). For the latter implementation (`--tokens`/`--cjk`),
there is no such limit. The offsets outputted will be UTF-8 *byte* positions.

This tool only does exact (or case insensitive) matching, if you need fuzzy
matching against lexicons, check out [analiticcl](https://github.com/proycon/analiticcl)
instead.

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
$ lexmatch --lexicon lexicon.lst corpus.txt
```

The lexicon must be plain-text UTF-8 containing one entry per line, an entry
need not be a single word and is not constrained in length. If the lexicon
consists of Tab Separated Values (TSV), then only the first column is
considered, the rest is ignored.

Instead of a lexicon you can also provide the patterns to query on the command line using ``--query``.

By default, you will get a TSV file with a column for the text, the occurrence count, and
one with the begin position (UTF-8 byte position) for each match (dynamic columns):

```
$ lexmatch --query good --query bad /nettmp/republic.short.txt 
Reading text from /tmp/republic.short.txt...
Building suffix array (this may take a while)...
Searching...
good    4       193     3307    3480    278
bad     3       201     3315    3488
```

Matching is case sensitive by default, add `--no-case` for case insensitive
behaviour (all input and output will be lowercase, this may in rare cases cause
the UTF-8 offsets to no longer be valid on the original text).

For verbose output, add ``--verbose``. This produces cleaner TSV (tab seperated
values) output that you can easily import in for example the [STAM
tools](https://github.com/annotation/stam-tools):

```
$ lexmatch --verbose --query good --query bad /nettmp/republic.short.txt
Text    BeginUtf8Offset EndUtf8Offset
Reading text from /tmp/republic.short.txt...
Building suffix array (this may take a while)...
Searching...
good    193     197
good    3307    3311
good    3480    3484
good    278     282
bad     201     204
bad     3315    3318
bad     3488    3491
```

You may provide multiple lexicons as well as multiple test files, the output
will output the lexicon and/or test file in such cases. If multiple lexicons match, they are all returned (delimited by a semicolon). The order of the
results is arbitrary.

If you don't care for the exact positions but rather want to compute a
frequency list with the number of occurrences for each item in the lexicon or
passed through ``--query``, then pass ``--count-only``:

```
$ lexmatch --count-only --query good --query bad /tmp/republic.short.txt
Reading text from /tmp/republic.short.txt...
Building suffix array (this may take a while)...
Searching...
good    4
bad	3
```

You can configure a minimum frequency threshold using ``--freq``.

Rather than match all of the lexicon against the text, you can also iterate
over tokens in the text and check if they occur in the lexicon. This uses a
hash map instead of a suffix array and is typically faster. It is more limited,
however, and can not be used with frequency thresholds or counting. It will
always produce verbose output (similar to ``--verbose``):

```
$ lexmatch --tokens --query good --query bad /nettmp/republic.short.txt
Text    BeginUtf8Offset EndUtf8Offset
Reading text from /tmp/republic.short.txt...
good    193     197
bad     201     204
good    278     282
good    3307    3311
bad     3315    3318
good    3480    3484
bad     3488    3491
```

Unlike before, you will find the matches are now returned in reading order.

If you add `--coverage` then you will get an extra last line with some coverage
statistics. This is useful to see how much of the text is covered by your
lexicon.

```
#coverage (tokens) = 7/627 = 0.011164274322169059
```

Coverage can also be computed line-by-line and matching against multiple lexicons, we can also read directly from stdin rather than from file by passing `-` as filename:

```
$ echo "Is this good or bad?\nIt is quite good." | lexmatch --coverage-matrix --query good --query bad  -
Reading text from -...
Line            query
Is this good or bad?    0.4
It is quite good.       0.25
```

This can be used as a simple lexicon-based method for language detection:

```
$ echo "Do you know what language this is?\nUnd was ist das hier genau?\nÇa va assez bien je crois" | lexmatch -i  --coverage-matrix --lexicon ~X/en.lst --lexicon ~X/de.lst --lexicon ~X/fr.lst  -                     
Reading lexicon...
Reading lexicon...
Reading lexicon...
Reading text from -...
Line            /home/proycon/exp/en.lst        /home/proycon/exp/de.lst        /home/proycon/exp/fr.lst        Total
do you know what language this is?      1       0       0.14285714285714285     1.1428571428571428
und was ist das hier genau?     0.16666666666666666     0.8333333333333334      0.16666666666666666     1.1666666666666667
ça va assez bien je crois       0.2     0.2     0.6     1
```

When using ``--tokens`` (or `--coverage-matrix`) we rely on whitespace and punctuation to delimit
tokens. This does not work for languages such as Chinese, Japanese and Korean
that are not delimited in such a way. For such languages, similar linear search
behaviour can be attained by passing ``--cjk`` instead, with an integer value
representing the maximum character length to explore. A greedy search will then
be performed that favours longer patterns over shorter ones.

