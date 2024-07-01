extern crate clap;
extern crate suffix;

use clap::{App, Arg};
use std::collections::HashSet;
use std::fs::File;
use std::io::{stdin, BufRead, BufReader, Read};
use std::process::exit;
use suffix::SuffixTable;

type Lexicon = HashSet<String>;

///Read a lexicon, one entry per line, TSV is allowed with entry in first column (rest will just be ignored)
fn read_lexicon(filename: &str) -> Result<Lexicon, std::io::Error> {
    let mut lexicon = HashSet::new();
    let f = File::open(filename)?;
    let f_buffer = BufReader::new(f);
    for line in f_buffer.lines() {
        if let Ok(entry) = line {
            let field = entry.split("\t").next().unwrap().to_string();
            if !field.is_empty() {
                lexicon.insert(field);
            }
        }
    }
    Ok(lexicon)
}

fn read_text(filename: &str) -> Result<String, std::io::Error> {
    if filename == "-" {
        let mut text: String = String::new();
        stdin().lock().read_to_string(&mut text)?;
        Ok(text)
    } else {
        let mut f = File::open(filename)?;
        let mut text: String = String::new();
        f.read_to_string(&mut text)?;
        Ok(text)
    }
}

fn build_suffixarray(text: &str) -> SuffixTable {
    SuffixTable::new(text)
}

fn main() {
    let args = App::new("Lexmatch")
                    .version("0.3")
                    .author("Maarten van Gompel (proycon) <proycon@anaproy.nl>")
                    .about("Simple lexicon matcher powered by either suffix arrays or hash tables. In the former case, it matches lookups from a lexicon to a text and returns, for each, the number of hits and the hits themselves (byte-offsets to the start position)")
                    .arg(Arg::with_name("lexicon")
                        .long("lexicon")
                        .short('l')
                        .help("The lexicon to use, has one entry on each line. If the input is TSV, only the first columns is considered. Entries may also be phrases/n-grams unless --tokens is set. Multiple lexicons are supported (and will be reflected in the output)")
                        .multiple_occurrences(true)
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
                        .help("Return all matches (also as substrings), rather than only exact matches. This is already implied when using --tokens or --cjk.")
                        .required(false))
                    .arg(Arg::with_name("verbose")
                        .long("verbose")
                        .short('v')
                        .help("Return output verbosely as TSV with each match on a separate row. Will output a header on the first line. Implied when --tokens or --cjk is set.")
                        .required(false))
                    .arg(Arg::with_name("tokens")
                        .long("tokens")
                        .alias("hash")
                        .short('T')
                        .help("Do a simple token-based lookup using a hash-table instead of using suffix arrays. This is usually faster but more limited (no thresholds etc). Only works on languages with whitespace/punctuation, use --cjk instead for Chinese/Japanese/Korean text.")
                        .required(false))
                    .arg(Arg::with_name("coverage")
                        .long("coverage")
                        .help("With --tokens; compute how many tokens are covered by the lexicon. With --cjk; on a character basis.")
                        .required(false))
                    .arg(Arg::with_name("cjk")
                        .short('C')
                        .long("cjk")
                        .alias("greedy-chars")
                        .help("Do a greedy character-based lookup using a hash-table instead of using suffix arrays. The value corresponds to the maximum number of characters to consider. Use this instead of --tokens for languages like Chinese, Japanese, Korean, use --tokens if the language uses whitesapce and punctuation as token delimiter.")
                        .takes_value(true)
                        .required(false))
                    .arg(Arg::with_name("no-matches")
                        .long("count-only")
                        .alias("no-matches")
                        .short('M')
                        .help("Don't return matching indices, only return the number of matches. Does not work with --tokens or --cjk")
                        .required(false))
                    .arg(Arg::with_name("freq")
                        .long("freq")
                        .short('f')
                        .help("An absolute frequency threshold, return only matches above this threshold, defaults to 1, set to 0 to return the entire lexicon. Does not work with --tokens.")
                        .takes_value(true)
                        .default_value("1"))
                    .arg(Arg::with_name("textfile")
                        .help("The filename of the text to operate on (plain text UTF-8, max 4GB unless --tokens is set), use - for standard input.")
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

    if args.is_present("no-matches") && args.is_present("verbose") {
        eprintln!("ERROR: --count-only and --verbose are mutually exclusive");
        exit(1);
    }

    if (args.is_present("tokens") || args.is_present("cjk")) && args.value_of("freq") != Some("1") {
        eprintln!("ERROR: Frequency thresholds do not work with --tokens/--cjk");
        exit(1);
    }

    let mut lexicons: Vec<Lexicon> = if args.is_present("lexicon") {
        args.get_many("lexicon")
            .unwrap()
            .map(|s: &String| {
                eprintln!("Reading lexicon...");
                read_lexicon(s).expect("Parsing lexicon")
            })
            .collect()
    } else {
        vec![HashSet::new()]
    };

    let lexiconnames: Vec<&str> = if args.is_present("lexicon") {
        args.get_many("lexicon").unwrap().copied().collect()
    } else {
        vec!["custom"]
    };

    if args.is_present("query") {
        let queries: Vec<&str> = args.values_of("query").unwrap().collect();
        for query in queries {
            lexicons[0].insert(query.to_string());
        }
    }

    let texts: Vec<String> = args
        .get_many("textfile")
        .expect("Expected one or more input files")
        .map(|s: &String| s.clone())
        .collect();

    let do_coverage = args.is_present("coverage");

    if args.is_present("verbose") || args.is_present("tokens") {
        print!("Text");
        if lexicons.len() > 1 {
            println!("\tLexicon");
        }
        if texts.len() > 1 {
            println!("\tResource");
        }
        println!("\tBeginUtf8Offset\tEndUtf8Offset");
    }

    for textfile in texts.iter() {
        eprintln!("Reading text from {}...", textfile);
        let text = read_text(textfile).expect("Parsing text");

        if args.is_present("tokens") {
            let mut token = String::new();
            let mut begin = 0;
            let mut matchcount = 0;
            let mut totalcount = 0;
            for (i, c) in text.char_indices() {
                if c.is_alphanumeric() {
                    token.push(c);
                } else if !token.is_empty() {
                    let mut matched_lexicon = None;
                    totalcount += 1;
                    for (lexicon, lexiconname) in lexicons.iter().zip(lexiconnames.iter()) {
                        if lexicon.contains(&token) {
                            matched_lexicon = Some(*lexiconname);
                            break;
                        }
                    }
                    if let Some(matched_lexicon) = matched_lexicon {
                        matchcount += 1;
                        let end = begin + token.len();
                        print!("{}", token);
                        if lexicons.len() > 1 {
                            print!("\t{}", matched_lexicon);
                        }
                        if texts.len() > 1 {
                            print!("\t{}", textfile);
                        }
                        println!("\t{}\t{}", begin, end);
                    }
                    token.clear();
                    begin = i + 1;
                } else {
                    begin = i + 1;
                }
            }
            if !token.is_empty() {
                totalcount += 1;
                let mut matched_lexicon = None;
                for (lexicon, lexiconname) in lexicons.iter().zip(lexiconnames.iter()) {
                    if lexicon.contains(&token) {
                        matched_lexicon = Some(*lexiconname);
                        break;
                    }
                }
                if let Some(matched_lexicon) = matched_lexicon {
                    matchcount += 1;
                    let end = begin + token.len();
                    print!("{}", token);
                    if lexicons.len() > 1 {
                        print!("\t{}", matched_lexicon);
                    }
                    if texts.len() > 1 {
                        print!("\t{}", textfile);
                    }
                    println!("\t{}\t{}", begin, end);
                }
            }
            if do_coverage {
                println!(
                    "#coverage (tokens) = {}/{} = {}",
                    matchcount,
                    totalcount,
                    matchcount as f64 / totalcount as f64
                );
            }
        } else if args.is_present("cjk") {
            let mut matchcount = 0;
            let maxlen = args
                .value_of("cjk")
                .unwrap()
                .parse::<usize>()
                .expect("length for --cjk must be an integer");
            for begin in 0..text.len() {
                for l in (1..=maxlen).rev() {
                    if let Some((lastbyte, c)) = text[begin..].char_indices().nth(l - 1) {
                        let end = lastbyte + c.len_utf8();
                        let pattern = &text[begin..end];
                        let mut matched_lexicon = None;
                        for (lexicon, lexiconname) in lexicons.iter().zip(lexiconnames.iter()) {
                            if lexicon.contains(pattern) {
                                matched_lexicon = Some(*lexiconname);
                                break;
                            }
                        }
                        if let Some(matched_lexicon) = matched_lexicon {
                            if do_coverage {
                                matchcount += pattern.chars().count();
                            }
                            print!("{}", pattern);
                            if lexicons.len() > 1 {
                                print!("\t{}", matched_lexicon);
                            }
                            if texts.len() > 1 {
                                print!("\t{}", textfile);
                            }
                            println!("\t{}\t{}", begin, end);
                        }
                        break; //longest match only
                    }
                }
                if do_coverage {
                    let totalcount = text.chars().count();
                    println!(
                        "#coverage (chars) = {}/{} = {}",
                        matchcount,
                        totalcount,
                        matchcount as f64 / totalcount as f64
                    );
                }
            }
        } else {
            eprintln!("Building suffix array (this may take a while)...");
            let suffixtable = build_suffixarray(&text);

            eprintln!("Searching...");
            for (lexicon, lexiconname) in lexicons.iter().zip(lexiconnames.iter()) {
                for entry in lexicon.iter() {
                    let matches = suffixtable.positions(entry);
                    let length = entry.as_bytes().len() as u32;

                    if args.is_present("all") {
                        if matches.len() >= freq_threshold {
                            if args.is_present("verbose") {
                                for begin in matches.iter() {
                                    let end = *begin + length;
                                    print!("{}", entry);
                                    if lexicons.len() > 1 {
                                        print!("\t{}", lexiconname);
                                    }
                                    if texts.len() > 1 {
                                        print!("\t{}", textfile);
                                    }
                                    println!("\t{}\t{}", *begin, end);
                                }
                            } else {
                                print!("{}", entry);
                                if lexicons.len() > 1 {
                                    print!("\t{}", lexiconname);
                                }
                                if texts.len() > 1 {
                                    print!("\t{}", textfile);
                                }
                                print!("\t{}", matches.len());
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
                                    print!("{}", entry);
                                    if lexicons.len() > 1 {
                                        print!("\t{}", lexiconname);
                                    }
                                    if texts.len() > 1 {
                                        print!("\t{}", textfile);
                                    }
                                    println!("\t{}\t{}", *begin, end);
                                }
                            } else {
                                print!("{}", entry);
                                if lexicons.len() > 1 {
                                    print!("\t{}", lexiconname);
                                }
                                if texts.len() > 1 {
                                    print!("\t{}", textfile);
                                }
                                print!("\t{}", matches_exact.len());
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
        }
    }
}
