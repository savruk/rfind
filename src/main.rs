extern crate walkdir;
extern crate clap;
extern crate ansi_term;
extern crate threadpool;

use clap::{Arg, App};
use std::fs;
use ansi_term::Style;
use walkdir::{WalkDir, DirEntry};
use std::io::BufReader;
use std::io::BufRead;
use std::io::Write;

use std::error::Error;
use std::collections::HashMap;
use std::sync::mpsc;
use threadpool::ThreadPool;
use std::io;

type FoundItem = HashMap<usize, String>;
type FoundItems<'a> = HashMap<String, FoundItem>;

fn search_file(search_str: String, file_extension: String, styled_search_string: String, entry: DirEntry) -> FoundItem {
    let mut found_item = FoundItem::new();
    let path = entry.path();
    let meta = fs::metadata(path).unwrap();
    if !meta.is_dir() {
        let display = path.display();
        if file_extension.as_str() != "" {
            if !entry.file_name().to_str().map(|s| s.ends_with(file_extension.as_str())).unwrap_or(false) {
                return found_item
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
            if inline != "" && inline.contains(search_str.as_str()) {
                found_item.insert(i + 1, inline.replace(search_str.as_str(), styled_search_string.as_str()));
            }
        }
    }
    found_item
}

fn search_recursive(search_str: &str, file_extension: &str) {
    let styled_search_string = Style::new().underline().paint(search_str.to_string()).to_string();
    let (tx, rx) = mpsc::channel();
    let pool = ThreadPool::new(4);
    for entry in WalkDir::new(".").into_iter().filter_map(|e| e.ok()) {
        let thread_tx = tx.clone();
        let ssc = search_str.to_string().clone();
        let f = file_extension.to_string().clone();
        let sss = styled_search_string.clone();
        let entry_clone = entry.clone();
        pool.execute(move || {
            let file_name = entry_clone.path().display().to_string();
            let found_item = search_file(ssc, f, sss, entry_clone);
            if found_item.len() > 0 {
                let mut items = FoundItems::new();
                items.insert(file_name, found_item);
                thread_tx.send(items);
                //                    .unwrap_or_else(|x: mpsc::SendError<FoundItems>| {println!("{:?}", x)});
            }
        });
    }
    drop(tx);
    for ff in rx {
        for (file_name, f_items) in ff.iter() {
            let stdout = io::stdout();
            let mut handle = stdout.lock();
            let mut buf = io::BufWriter::new(handle);
            buf.write_fmt(format_args!("{}\n", file_name));
            let mut vec: Vec<_> = f_items.iter().collect();
            vec.sort_by(|a, b| a.0.cmp(b.0));
            for (line_num, line) in vec {
                buf.write_fmt(format_args!("{}: {}\n", line_num, line));
            }
            buf.write_fmt(format_args!("{}\n", ""));
        }
    }
}

fn list_recursive(search_str: &str, file_extension: &str) {
    let stdout = io::stdout();
    let handle = stdout.lock();
    let mut buf = io::BufWriter::new(handle);

    for entry in WalkDir::new(".").into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        let meta = fs::metadata(path).unwrap();
        if !meta.is_dir() {
            let display = path.display();
            let fname = entry.file_name().to_str();
            //println!("{}", fname.unwrap_or(""));
            if file_extension != ""  && search_str != "" {
                if fname.map(|s| s.ends_with(file_extension)).unwrap_or(false) && fname.unwrap_or("").contains(search_str) {
                    buf.write_fmt(format_args!("{}\n", display.to_string()));
                }
            } else if file_extension != "" {
                if fname.map(|s| s.ends_with(file_extension)).unwrap_or(false)  {
                    buf.write_fmt(format_args!("{}\n", display.to_string()));
                }
            } else if search_str != "" {
                if fname.unwrap_or("").contains(search_str) {
                    buf.write_fmt(format_args!("{}\n", display.to_string()));
                }
            }
            else{
                buf.write_fmt(format_args!("{}\n", display.to_string()));
            }
        }

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
            .takes_value(true)
        )
        .arg(Arg::with_name("FILE EXTENSION")
            .help("Searches only in the matching files. e.g. 'py' 'html' "))
        .arg(
            Arg::with_name("LIST FILES")
            .help("List all files")
            .short("l")
            .long("list")
            .takes_value(false)
        )
        .get_matches();
    let search_string = matches.value_of("SEARCH PATTERN").unwrap_or("");
    let file_extension = matches.value_of("FILE EXTENSION").unwrap_or("");
    let list_files = matches.occurrences_of("LIST FILES");
    if list_files > 0 {
        list_recursive(search_string, file_extension)
    } else{
        search_recursive(search_string, file_extension)
    }
}

