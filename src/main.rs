use number_prefix::{decimal_prefix, PrefixNames, Prefixed, Standalone};
use std::collections::HashMap;
use std::os::unix::fs::MetadataExt;
use std::time::{Duration, Instant};
use walkdir::{DirEntry, WalkDir};

extern crate clap;
use clap::{App, Arg};

extern crate termion;
use termion::{clear, cursor, terminal_size};

const DEFAULT_PRINT_N: u16 = 32;
const DEFAULT_MAX_DEPTH: usize = 4;
const DEFAULT_UPDATE_INTERVAL_MILLIS: u64 = 100;

#[derive(Debug, Eq, PartialEq, PartialOrd)]
struct NodeDir {
    size: u64,
}

fn increment_size_ancestors(storage: &mut HashMap<String, NodeDir>, entry: &DirEntry) {
    let entry_size = entry.metadata().unwrap().size();
    for i in entry
        .path()
        .ancestors()
        .filter_map(|parent| parent.to_str())
    {
        storage.entry(i.to_string()).and_modify(|parent| {
            // println!("ancestor{:?}", i);
            parent.size += entry_size
        });
    }
}

fn process_dir_entry(storage: &mut HashMap<String, NodeDir>, entry: &DirEntry) {
    // println!("path: {}", entry.path().display());
    increment_size_ancestors(storage, entry);

    if entry.file_type().is_dir() {
        // println!("depth: {}  {}", entry.depth(), entry.path().display());
        storage.insert(
            entry.path().to_str().unwrap().to_owned(),
            NodeDir {
                size: entry.metadata().unwrap().size(),
            },
        );
    }
}

fn update_print(
    root_path: &str,
    n_lines: u16,
    file_count: usize,
    storage: &HashMap<String, NodeDir>,
    only_biggest: bool,
) {
    let mut order_storage: Vec<(&String, &NodeDir)> = storage.iter().collect();
    order_storage.sort_by_key(|a| a.1.size);
    order_storage.reverse();

    // build vector of paths be printed
    // let biggest match only_biggest {}
    let mut biggest: Vec<(&String, &NodeDir)> = Vec::new();
    for v in order_storage.iter() {
        if biggest.len() as u16 >= n_lines {
            break;
        }
        if only_biggest {
            biggest.retain(|x| !v.0.starts_with(x.0));
        }
        biggest.push(*v);
    }

    print!("{}", cursor::Up(n_lines + 1));

    print!("\rTotal file count: ");
    match decimal_prefix(file_count as f64) {
        Standalone(bytes) => println!("{:>6.2}", bytes),
        Prefixed(prefix, n) => println!("{:>6.2} {}", n, prefix.symbol()),
    }

    for v in biggest.iter() {
        print!("{}", clear::CurrentLine);
        match decimal_prefix(v.1.size as f64) {
            Standalone(bytes) => print!("{:>6.2} B", bytes),
            Prefixed(prefix, n) => print!("{:>6.2} {}B", n, prefix.symbol()),
        }
        println!(" {:?}", v.0);
    }
    for _ in biggest.len() as u16..n_lines {
        println!();
    }
}

fn main() {
    // TODO remove entry from tree (HashMap) when processed and size is smaller than the smallest in our top 10.
    let matches = App::new("My Super Program")
        .author("lpasselin <louisphilippeasselin@gmail.com>")
        .about("Finds biggest directories")
        .arg(
            Arg::with_name("path")
                .short("p")
                .long("path")
                .help("Specify root path. default=\".\"")
                .takes_value(true)
                .value_name("PATH"),
        )
        .arg(
            Arg::with_name("depth")
                .short("d")
                .long("depth")
                .help("Sets max recursive depth. default=3")
                .takes_value(true)
                .value_name("NUM"),
        )
        .arg(
            Arg::with_name("lines")
                .short("l")
                .long("lines")
                .help("Sets max printed lines. default=terminal height")
                .takes_value(true)
                .value_name("NUM_PRINT"),
        )
        .arg(
            Arg::with_name("no_parent")
                .short("n")
                .long("no_parent")
                .help("Does not print parents."),
        )
        .get_matches();

    let config = (
        matches.value_of("path").unwrap_or("."),
        match matches.value_of("depth") {
            Some(x) => x.parse::<usize>().expect("Depth is uint only"),
            None => DEFAULT_MAX_DEPTH,
        },
        match matches.value_of("lines") {
            Some(x) => x.parse::<u16>().expect("Depth is uint only"),
            None => terminal_size().unwrap().1 as u16 - 5,
        },
        matches.is_present("no_parent"),
    );

    println!("Path: \"{}\"", config.0);
    println!("max depth: {}", config.1);
    println!("====================");

    let mut storage: HashMap<String, NodeDir> = HashMap::new();
    storage.insert(config.0.to_owned(), NodeDir { size: 0 });

    // init terminal space required
    for _ in 0..config.2 {
        println!();
    }
    let mut file_count = 0;
    let mut instant_next = Instant::now() + Duration::from_millis(DEFAULT_UPDATE_INTERVAL_MILLIS);
    let walkdir = WalkDir::new(config.0)
        .max_depth(config.1)
        .same_file_system(true)
        .into_iter()
        .filter_map(|e| e.ok());
    for entry in walkdir {
        file_count += 1;
        if instant_next < Instant::now() {
            update_print(config.0, config.2, file_count, &storage, config.3);
            instant_next = Instant::now() + Duration::from_millis(DEFAULT_UPDATE_INTERVAL_MILLIS);
        }
        process_dir_entry(&mut storage, &entry);
    }
    update_print(config.0, config.2, file_count, &storage, config.3);
}
