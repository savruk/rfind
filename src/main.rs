extern crate ansi_term;
extern crate clap;
extern crate log;
extern crate threadpool;
extern crate walkdir;

use ansi_term::Style;
use async_std::{fs, io, io::prelude::*, println, task};
use clap::{App, Arg};
use env_logger;
use futures::stream::{self, StreamExt, TryStreamExt};
use log::{debug, info};
use std::collections::HashMap;
use walkdir::WalkDir;

type FoundItem = HashMap<usize, String>;
type FoundItems<'a> = HashMap<String, FoundItem>;

#[derive(Debug)]
enum Action {
    Search,
}
struct Rfind {
    search_string: String,
    file_extension: String,
    action: Action,
}

impl Rfind {
    pub fn new(search_string: String, file_extension: String, action: Action) -> Self {
        Self {
            search_string,
            file_extension,
            action,
        }
    }

    pub async fn run(&self) -> Result<(), io::Error>{
        debug!("run with {:?}", self.action);

        let res = match self.action {
            // Action::List => self.list_recursive(),
            Action::Search => self.search_recursive(),
        };
        res.await
    }

    async fn search_file(
        &self,
        search_str: &str,
        file_extension: &str,
        styled_search_string: &str,
        entry: walkdir::DirEntry,
    ) -> FoundItem {
        let mut found_item = FoundItem::new();
        let path = entry.path();
        let meta = match fs::metadata(path).await {
            Err(why) => {
                print_error(&why, path);
                return found_item;
            }
            Ok(meta) => meta,
        };
        if !meta.is_dir() {
            let display = path.display();
            if file_extension != "" {
                if !entry
                    .file_name()
                    .to_str()
                    .map(|s| s.ends_with(file_extension))
                    .unwrap_or(false)
                {
                    return found_item;
                }
            }
            // Open the path in read-only mode, returns `io::Result<File>`
            let file = match fs::File::open(path).await {
                // The `description` method of `io::Error` returns a string that
                // describes the error
                Err(why) => panic!("couldn't open {}: {}", display, why),
                Ok(file) => file,
            };

            let br = io::BufReader::new(file);
            let mut lines = br.lines();
            let mut i = 1;
            while let Some(line) = lines.next().await {
                let inline = line.unwrap_or(String::new());
                if inline != "" && inline.contains(search_str) {
                    found_item.insert(i, inline.replace(search_str, styled_search_string));
                }
                i += 1;
            }
        } else {
            println!("not dir: {:?}", path);
        }
        // debug!("found something: {:?}: {:?}", found_item, path);

        found_item
    }
    async fn search_recursive(&self) -> Result<(), io::Error> {
        let styled_search_string = Style::new()
            .underline()
            .paint(self.search_string.as_str())
            .to_string();
        let path_stream = stream::iter(WalkDir::new(".").into_iter());
        const MAX_CONCURRENT_JUMPERS: usize = 100;

        let fut = path_stream.try_for_each_concurrent(MAX_CONCURRENT_JUMPERS, |entry| async {
            // debug!("checking {:?}", entry);
            let item = self.search_file(
                self.search_string.as_str(),
                self.file_extension.as_str(),
                styled_search_string.as_str(),
                entry,
            )
            .await;
            debug!("found something {:?} {:?}", item[0], item[1]);

            Ok(())
        }).await;
        debug!("try_for_each_concurrent finished {:?}", fut);
        Ok(())
    }
}

fn print_error(err: &std::io::Error, path: &std::path::Path) {
    if let Some(inner_err) = err.get_ref() {
        println!("ERROR: {:?}: {:?}", inner_err, path);
    } else {
        println!("ERROR: {:?}: {:?}", err.kind(), path);
    }
}

// Open the path in read-only mode, returns `io::Result<File>`
fn main() {
    env_logger::init();
    let matches = App::new("RFIND")
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about("Does awesome things")
        .arg(
            Arg::with_name("SEARCH PATTERN")
                .help("Sets the input file to use")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("FILE EXTENSION")
                .help("Searches only in the matching files. e.g. 'py' 'html' "),
        )
        .arg(
            Arg::with_name("LIST FILES")
                .help("List all files")
                .short("l")
                .long("list")
                .takes_value(false),
        )
        .get_matches();
    let search_string = matches.value_of("SEARCH PATTERN").unwrap_or("").to_string();
    let file_extension = matches.value_of("FILE EXTENSION").unwrap_or("").to_string();
    let _list_files = matches.occurrences_of("LIST FILES");
    task::block_on(Rfind::new(search_string, file_extension, Action::Search).run());
}
