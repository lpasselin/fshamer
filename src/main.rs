use number_prefix::{decimal_prefix, PrefixNames, Prefixed, Standalone};
use std::collections::HashMap;
use std::os::unix::fs::MetadataExt;
use std::time::{Duration, Instant};
use walkdir::{DirEntry, WalkDir};

extern crate clap;
use clap::{App, Arg};

extern crate termion;
use termion::{clear, cursor};

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

fn update_print(root_path: &str, file_count: usize, storage: &HashMap<String, NodeDir>) {
    let mut order_storage: Vec<(&String, &NodeDir)> = storage.iter().collect();
    order_storage.sort_by_key(|a| a.1.size);
    order_storage.reverse();

    print!("{}", cursor::Up(DEFAULT_PRINT_N + 1));

    print!("\rTotal file count: ");
    match decimal_prefix(file_count as f64) {
        Standalone(bytes) => println!("{:>6.2}", bytes),
        Prefixed(prefix, n) => println!("{:>6.2} {}", n, prefix.symbol()),
    }

    let mut line_counter = 0;
    for v in order_storage.iter() {
        if line_counter >= DEFAULT_PRINT_N {
            break;
        }
        print!("{}", clear::CurrentLine);
        match decimal_prefix(v.1.size as f64) {
            Standalone(bytes) => print!("{:>6.2} B", bytes),
            Prefixed(prefix, n) => print!("{:>6.2} {}B", n, prefix.symbol()),
        }
        println!(" {:?}", v.0);
        line_counter += 1;
    }
    for _ in line_counter..DEFAULT_PRINT_N {
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
        .get_matches();

    let config = (
        matches.value_of("path").unwrap_or("."),
        match matches.value_of("depth") {
            Some(x) => x.parse::<usize>().expect("Depth is uint only"),
            None => DEFAULT_MAX_DEPTH,
        },
    );

    println!("Path: \"{}\"", config.0);
    println!("max depth: {}", config.1);
    println!("====================");

    let mut storage: HashMap<String, NodeDir> = HashMap::new();
    storage.insert(config.0.to_owned(), NodeDir { size: 0 });

    // init terminal space required
    for _ in 0..DEFAULT_PRINT_N {
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
            update_print(config.0, file_count, &storage);
            instant_next = Instant::now() + Duration::from_millis(DEFAULT_UPDATE_INTERVAL_MILLIS);
        }
        process_dir_entry(&mut storage, &entry);
    }
    update_print(config.0, file_count, &storage);
}
