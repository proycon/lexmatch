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
fn read_lexicon(filename: &str, lowercase: bool) -> Result<Lexicon, std::io::Error> {
    let mut lexicon = HashSet::new();
    let f = File::open(filename)?;
    let f_buffer = BufReader::new(f);
    for line in f_buffer.lines() {
        if let Ok(entry) = line {
            let field = entry.split("\t").next().unwrap().to_string();
            if !field.is_empty() {
                lexicon.insert(if lowercase {
                    field.to_lowercase()
                } else {
                    field
                });
            }
        }
    }
    Ok(lexicon)
}

fn read_text(filename: &str, lowercase: bool) -> Result<String, std::io::Error> {
    if filename == "-" {
        let mut text: String = String::new();
        stdin().lock().read_to_string(&mut text)?;
        if lowercase {
            text = text.to_lowercase();
        }
        text.push('\n'); //ensure we always end with a newline
        Ok(text)
    } else {
        let mut f = File::open(filename)?;
        let mut text: String = String::new();
        f.read_to_string(&mut text)?;
        if lowercase {
            text = text.to_lowercase();
        }
        text.push('\n');
        Ok(text)
    }
}

fn build_suffixarray(text: &str) -> SuffixTable {
    SuffixTable::new(text)
}

#[inline]
fn print_verbose_match(
    match_text: &str,
    begin: usize,
    end: usize,
    matched_lexicons: &Vec<bool>,
    lexiconnames: &Vec<String>,
    texts_len: usize,
    textfile: &str,
) {
    print!("{}", match_text);
    if lexiconnames.len() > 1 {
        print!("\t");
        let mut first = true;
        for (matches, lexiconname) in matched_lexicons.iter().zip(lexiconnames.iter()) {
            if *matches {
                print!("{}{}", if !first { ";" } else { "" }, lexiconname);
                first = false;
            }
        }
    }
    if texts_len > 1 {
        print!("\t{}", textfile);
    }
    println!("\t{}\t{}", begin, end);
}

#[inline]
fn print_multi_match(
    match_text: &str,
    indices: &[u32],
    lexiconname: &str,
    lexicon_len: usize,
    texts_len: usize,
    textfile: &str,
    ignore_matches: bool,
) {
    print!("{}", match_text);
    print!("\t{}", indices.len());
    if lexicon_len > 1 {
        print!("\t{}", lexiconname);
    }
    if texts_len > 1 {
        print!("\t{}", textfile);
    }
    if !ignore_matches {
        //dynamic columns
        for begin in indices.iter() {
            print!("\t{}", begin);
        }
    }
    println!();
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
                    .arg(Arg::with_name("coverage-matrix")
                        .long("coverage-matrix")
                        .help("For each line in the input, compute the coverage in the lexicons")
                        .required(false))
                    .arg(Arg::with_name("min-token-length")
                        .long("min-token-length")
                        .help("Minimum token length to consider, shorter tokens will be ignored and not matched (applies --tokens, --coverage and --coverage-matrix)")
                        .takes_value(true)
                        .required(false))
                    .arg(Arg::with_name("cjk")
                        .short('C')
                        .long("cjk")
                        .alias("greedy-chars")
                        .help("Do a greedy character-based lookup using a hash-table instead of using suffix arrays. The value corresponds to the maximum number of characters to consider. Use this instead of --tokens for languages like Chinese, Japanese, Korean, use --tokens if the language uses whitesapce and punctuation as token delimiter.")
                        .takes_value(true)
                        .required(false))
                    .arg(Arg::with_name("no-case")
                        .long("no-case")
                        .alias("case-insensitive")
                        .short('i')
                        .help("Case insensitive matching. (Warning: This *MAY* result in rare cases result in offsets that no longer match the original text!)")
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

    if (!args.is_present("tokens") && !args.is_present("cjk")) && args.is_present("coverage") {
        eprintln!("ERROR: --coverage can only be used with --tokens or --cjk");
        exit(1);
    }

    let mut lexicons: Vec<Lexicon> = if args.is_present("lexicon") {
        args.get_many("lexicon")
            .unwrap()
            .map(|s: &String| {
                eprintln!("Reading lexicon...");
                read_lexicon(s, args.is_present("no-case")).expect("Parsing lexicon")
            })
            .collect()
    } else {
        vec![HashSet::new()]
    };

    let lexiconnames: Vec<String> = if args.is_present("lexicon") {
        args.get_many("lexicon")
            .unwrap()
            .map(|s: &String| s.clone())
            .collect()
    } else {
        vec!["query".to_string()]
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
    let min_token_length = args
        .value_of("min-token-length")
        .unwrap()
        .parse::<usize>()
        .expect("Value must be integer"); //only for coverage computation

    if args.is_present("verbose") || args.is_present("tokens") || args.is_present("cjk") {
        print!("Text");
        if lexicons.len() > 1 {
            println!("\tLexicon");
        }
        if texts.len() > 1 {
            println!("\tResource");
        }
        println!("\tBeginUtf8Offset\tEndUtf8Offset");
    }

    let mut matchcount = vec![0; lexicons.len()]; //indices correspond to the lexicon
    let mut matched_lexicon = vec![false; lexicons.len()]; //indices correspond to the lexicon
    let mut totalcount = 0;

    for textfile in texts.iter() {
        eprintln!("Reading text from {}...", textfile);
        let text = read_text(textfile, args.is_present("no-case")).expect("Parsing text");

        if args.is_present("coverage-matrix") {
            let mut token = String::new();
            print!("Line\t",);
            for lexiconname in lexiconnames.iter() {
                print!("\t{}", lexiconname);
            }
            if lexiconnames.len() > 1 {
                print!("\tTotal");
            }
            println!();
            for line in text.split("\n") {
                if !line.is_empty() {
                    totalcount = 0;
                    for item in &mut matchcount {
                        //reset matches
                        *item = 0;
                    }
                    print!("{}", line.trim_matches('\r'));
                    for c in line.chars() {
                        if c.is_alphanumeric() {
                            token.push(c);
                        } else if !token.is_empty() {
                            if token.chars().any(|c| c.is_alphabetic())
                                && (min_token_length <= 1
                                    || token.chars().count() >= min_token_length)
                            {
                                totalcount += 1;
                                for (j, lexicon) in lexicons.iter().enumerate() {
                                    if lexicon.contains(&token) {
                                        matchcount[j] += 1;
                                    }
                                }
                            }
                            token.clear();
                        }
                    }
                    let mut sumcount = 0;
                    for count in matchcount.iter() {
                        sumcount += *count;
                        print!(
                            "\t{}",
                            if totalcount == 0 {
                                0.0
                            } else {
                                *count as f64 / totalcount as f64
                            }
                        );
                    }
                    if lexiconnames.len() > 1 {
                        print!(
                            "\t{}",
                            if totalcount == 0 {
                                0.0
                            } else {
                                sumcount as f64 / totalcount as f64
                            }
                        );
                    }
                    println!();
                }
            }
        } else if args.is_present("tokens") {
            let mut token = String::new();
            let mut begin = 0;
            for (i, c) in text.char_indices() {
                if c.is_alphanumeric() {
                    token.push(c);
                } else if !token.is_empty() {
                    if token.chars().any(|c| c.is_alphabetic())
                        && (min_token_length <= 1 || token.chars().count() >= min_token_length)
                    {
                        let mut has_match = false;
                        for item in &mut matched_lexicon {
                            //reset matches
                            *item = false;
                        }
                        totalcount += 1;
                        for (j, lexicon) in lexicons.iter().enumerate() {
                            if lexicon.contains(&token) {
                                matched_lexicon[j] = true;
                                matchcount[j] += 1;
                                has_match = true;
                            }
                        }
                        if has_match {
                            print_verbose_match(
                                &token,
                                begin,
                                begin + token.len(),
                                &matched_lexicon,
                                &lexiconnames,
                                texts.len(),
                                textfile,
                            );
                        }
                    }
                    token.clear();
                    begin = i + 1;
                } else {
                    begin = i + 1;
                }
            }
        } else if args.is_present("cjk") {
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
                        let mut has_match = false;
                        for item in &mut matched_lexicon {
                            //reset matches
                            *item = false;
                        }
                        for (j, lexicon) in lexicons.iter().enumerate() {
                            if lexicon.contains(pattern) {
                                matched_lexicon[j] = true;
                                matchcount[j] += 1;
                                has_match = true;
                            }
                        }
                        if has_match {
                            print_verbose_match(
                                &pattern,
                                begin,
                                end,
                                &matched_lexicon,
                                &lexiconnames,
                                texts.len(),
                                textfile,
                            );
                        }
                        break; //longest match only
                    }
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
                                    print_verbose_match(
                                        &entry,
                                        *begin as usize,
                                        *begin as usize + length as usize,
                                        &matched_lexicon,
                                        &lexiconnames,
                                        texts.len(),
                                        textfile,
                                    );
                                }
                            } else {
                                print_multi_match(
                                    &entry,
                                    matches,
                                    &lexiconname,
                                    lexiconnames.len(),
                                    texts.len(),
                                    textfile,
                                    args.is_present("no-matches"),
                                );
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
                                    print_verbose_match(
                                        &entry,
                                        *begin as usize,
                                        end as usize,
                                        &matched_lexicon,
                                        &lexiconnames,
                                        texts.len(),
                                        textfile,
                                    );
                                }
                            } else {
                                print_multi_match(
                                    &entry,
                                    matches,
                                    &lexiconname,
                                    lexiconnames.len(),
                                    texts.len(),
                                    textfile,
                                    args.is_present("no-matches"),
                                );
                            }
                        }
                    }
                }
            }
        }
        if do_coverage {
            let mut sumcount = 0;
            for (i, lexiconname) in lexiconnames.iter().enumerate() {
                sumcount += matchcount[i];
                println!(
                    "#coverage ({} in {}) = {}/{} = {}",
                    if args.is_present("tokens") {
                        "tokens"
                    } else {
                        "characters"
                    },
                    lexiconname,
                    matchcount[i],
                    totalcount,
                    if totalcount == 0 {
                        0.0
                    } else {
                        matchcount[i] as f64 / totalcount as f64
                    }
                );
            }
            if lexiconnames.len() > 1 {
                println!(
                    "#coverage ({} against all) = {}/{} = {}",
                    if args.is_present("tokens") {
                        "tokens"
                    } else {
                        "characters"
                    },
                    sumcount,
                    totalcount,
                    if totalcount == 0 {
                        0.0
                    } else {
                        sumcount as f64 / totalcount as f64
                    }
                );
            }
        }
    }
}
