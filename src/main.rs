extern crate clap;
extern crate suffix;

use clap::{App, Arg};
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::process::exit;
use suffix::SuffixTable;

///Read a lexicon, one entry per line
fn read_lexicon(filename: &str) -> Result<Vec<String>, std::io::Error> {
    let mut lexicon: Vec<String> = Vec::new();
    let f = File::open(filename)?;
    let f_buffer = BufReader::new(f);
    for line in f_buffer.lines() {
        if let Ok(entry) = line {
            if !entry.is_empty() {
                lexicon.push(entry);
            }
        }
    }
    Ok(lexicon)
}

fn read_text(filename: &str) -> Result<String, std::io::Error> {
    let mut f = File::open(filename)?;
    let mut text: String = String::new();
    f.read_to_string(&mut text)?;
    Ok(text)
}

fn build_suffixarray(text: &str) -> SuffixTable {
    SuffixTable::new(text)
}

fn main() {
    let args = App::new("Lexmatch")
                    .version("0.3")
                    .author("Maarten van Gompel (proycon) <proycon@anaproy.nl>")
                    .about("Simple lexicon matcher powered by suffix arrays. Matches lookups from a lexicon to a text and returns, for each, the number of hits and the hits themselves (byte-offsets to the start position)")
                    .arg(Arg::with_name("lexicon")
                        .long("lexicon")
                        .short('l')
                        .help("The lexicon to use, has one entry on each line. Entries may also be phrases/n-grams")
                        .takes_value(true))
                    .arg(Arg::with_name("query")
                        .long("query")
                        .short('q')
                        .help("A word/phrase to lookup; command-line alternative to providing a lexicon")
                        .takes_value(true)
                        .number_of_values(1)
                        .multiple(true))
                    .arg(Arg::with_name("all")
                        .long("all")
                        .short('a')
                        .help("Return all matches (also as substrings), rather than only exact matches")
                        .required(false))
                    .arg(Arg::with_name("verbose")
                        .long("verbose")
                        .short('v')
                        .help("Return output verbosely as TSV with each match on a separate row. Will output a header on the first line.")
                        .required(false))
                    .arg(Arg::with_name("no-matches")
                        .long("no-matches")
                        .short('M')
                        .help("Don't return matching indices, only return the number of matches")
                        .required(false))
                    .arg(Arg::with_name("freq")
                        .long("freq")
                        .short('f')
                        .help("An absolute frequency threshold, return only matches above this threshold, defaults to 1, set to 0 to return the entire lexicon")
                        .takes_value(true)
                        .default_value("1"))
                    .arg(Arg::with_name("text")
                        .help("The filename of the text to operate on (plain text UTF-8, max 4GB)")
                        .multiple_occurrences(true)
                        .required(true))
                    .get_matches();

    let freq_threshold = args
        .value_of("freq")
        .expect("frequency threshold")
        .parse::<usize>()
        .expect("Frequency threshold must be an integer value >= 0");

    if !args.is_present("lexicon") && !args.is_present("query") {
        eprintln!("ERROR: specify either --lexicon or --query");
        exit(1);
    }

    let mut lexicon = if args.is_present("lexicon") {
        eprintln!("Reading lexicon...");
        read_lexicon(args.value_of("lexicon").expect("value")).expect("Parsing lexicon")
    } else {
        Vec::new()
    };

    if args.is_present("query") {
        let queries: Vec<&str> = args.values_of("query").unwrap().collect();
        for query in queries {
            lexicon.push(query.to_string());
        }
    }

    eprintln!("Reading text...");
    let text = read_text(args.value_of("text").expect("value")).expect("Parsing text");

    eprintln!("Building suffix array (this may take a while)...");
    let suffixtable = build_suffixarray(&text);

    eprintln!("Searching...");
    if args.is_present("verbose") {
        println!("Text\tBeginUtf8Offset\tEndUtf8Offset");
    }
    for entry in lexicon.iter() {
        let matches = suffixtable.positions(entry);
        let length = entry.as_bytes().len() as u32;

        if args.is_present("all") {
            if matches.len() >= freq_threshold {
                if args.is_present("verbose") {
                    for begin in matches.iter() {
                        let end = *begin + length;
                        println!("{}\t{}\t{}", entry, *begin, end);
                    }
                } else {
                    print!("{}\t{}", entry, matches.len());
                    if !args.is_present("no-matches") {
                        for begin in matches.iter() {
                            print!("\t{}", begin);
                        }
                    }
                    println!();
                }
            }
        } else {
            //Filter matches that are substrings rather than exact matches
            //this is a simplification that ignores the UTF-8 nature of the text, but will work when
            //boundaries are simple ascii-like spaces, punctuation etc.
            //
            let bytetext: &[u8] = text.as_bytes();
            let matches_exact: Vec<u32> = matches
                .into_iter()
                .filter_map(|begin| {
                    let begin = *begin as usize;
                    if begin > 0 {
                        let c: char = bytetext[begin - 1] as char;
                        if c.is_alphanumeric() {
                            return None;
                        }
                    }
                    if (begin + length as usize) < bytetext.len() {
                        let c: char = bytetext[begin + length as usize] as char;
                        if c.is_alphanumeric() {
                            return None;
                        }
                    }
                    Some(begin as u32)
                })
                .collect();

            if matches_exact.len() >= freq_threshold {
                if args.is_present("verbose") {
                    for begin in matches_exact.iter() {
                        let end = begin + length;
                        println!("{}\t{}\t{}", entry, *begin, end);
                    }
                } else {
                    print!("{}\t{}", entry, matches_exact.len());
                    if !args.is_present("no-matches") {
                        for begin in matches_exact.iter() {
                            print!("\t{}", begin);
                        }
                    }
                    println!();
                }
            }
        }
    }
}
