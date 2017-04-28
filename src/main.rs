extern crate walkdir;
extern crate clap;
extern crate ansi_term;

use clap::{Arg, App};
use std::fs;
use ansi_term::Style;
use walkdir::{WalkDir, DirEntry};
use std::io::BufReader;
use std::io::BufRead;
use std::error::Error;
use std::collections::HashMap;


type FoundItem<'a> = HashMap<String, HashMap<usize, String>>;

fn search_file(search_str: &str, file_extension: &str, styled_search_string: &str, entry: &DirEntry, found_items: &mut FoundItem) {
    let path = entry.path();
    let meta = fs::metadata(path).unwrap();
    if !meta.is_dir() {
        let display = path.display();
        if file_extension != "" {
            if !entry.file_name().to_str().map(|s| s.ends_with(file_extension)).unwrap_or(false) {
                return
            }
        }
        // Open the path in read-only mode, returns `io::Result<File>`
        let file = match fs::File::open(&path) {
            // The `description` method of `io::Error` returns a string that
            // describes the error
            Err(why) => panic!("couldn't open {}: {}", display,
                               why.description()),
            Ok(file) => file,
        };

        let br = BufReader::new(&file);
        for (i, line) in br.lines().enumerate() {
            let inline = line.unwrap_or(String::new());
            if inline != "" && inline.contains(search_str) {
                let item = found_items.entry(display.to_string()).or_insert(HashMap::new());
                item.insert(i + 1, inline.replace(search_str, styled_search_string));
            }
        }
    }
}

fn search_recursive(search_str: &str, file_extension: &str) {
    let styled_search_string = Style::new().underline().paint(search_str.to_string()).to_string();

    let mut found_items = FoundItem::new();
    for entry in WalkDir::new(".").into_iter().filter_map(|e| e.ok()) {
        search_file(search_str, file_extension, &styled_search_string, &entry, &mut found_items);
    }
    for (file_name, items) in found_items.iter() {
        println!("{}", file_name);
        let mut vec: Vec<_> = items.iter().collect();
        vec.sort_by(|a, b| a.0.cmp(b.0));
        for (line_num, line) in vec {
            println!("{}: {}", line_num, line);
        }
        println!();
    }
}
// Open the path in read-only mode, returns `io::Result<File>`
fn main() {
    let matches = App::new("RFIND")
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about("Does awesome things")
        .arg(Arg::with_name("SEARCH PATTERN")
            .help("Sets the input file to use")
            .required(true)
            .index(1))
        .arg(Arg::with_name("FILE EXTENSION")
            .help("Searches only in the matching files. e.g. 'py' 'html' "))
        .get_matches();
    let search_string = matches.value_of("SEARCH PATTERN").unwrap();
    let file_extension = match matches.value_of("FILE EXTENSION") {
        Some(file_extension) => file_extension,
        None => ""
    };
    search_recursive(search_string, file_extension)
}

