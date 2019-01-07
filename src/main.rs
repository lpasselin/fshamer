use number_prefix::{decimal_prefix, PrefixNames, Prefixed, Standalone};
use std::collections::HashMap;
use std::os::unix::fs::MetadataExt;
use std::time::{Duration, Instant};
use structopt::StructOpt;
use walkdir::{DirEntry, WalkDir};

extern crate termion;
use termion::{clear, cursor, terminal_size};

const DEFAULT_PRINT_N: u16 = 28;

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

fn update_print(config: &Config, file_count: usize, storage: &HashMap<String, NodeDir>) {
    let mut order_storage: Vec<(&String, &NodeDir)> = storage.iter().collect();
    order_storage.sort_by_key(|a| a.1.size);
    order_storage.reverse();

    // build vector of paths be printed
    let mut biggest: Vec<(&String, &NodeDir)> = Vec::new();
    for v in order_storage.iter() {
        if biggest.len() as u16 >= config.nb_line {
            break;
        }
        if config.no_parent {
            biggest.retain(|x| !v.0.starts_with(x.0));
        }
        biggest.push(*v);
    }

    print!("{}", cursor::Up(config.nb_line + 1));
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
    for _ in biggest.len() as u16..config.nb_line {
        println!();
    }
}

#[derive(StructOpt, Debug)]
#[structopt(name = "fshamer", about = "Finds largest directories")]
#[structopt(raw(setting = "structopt::clap::AppSettings::ColoredHelp"))]
#[structopt(rename_all = "kebab-case")]
struct Config {
    // Specify root path
    #[structopt(short, long, default_value = ".")]
    path: String,

    #[structopt(short, long, default_value = "200")]
    // Specify interval in ms to print during processing. 0 means never. 500 is decent.
    interval: u64,

    #[structopt(short, long, default_value = "0")]
    // Sets number of lines (folders) to print. 0 is max(terminal_height, 28).
    nb_line: u16,

    #[structopt(short = "s", long)]
    // Does not print parents of largest folders
    no_parent: bool,
}

fn edit_config(config: &mut Config) {
    if config.nb_line == 0 {
        config.nb_line = match terminal_size() {
            Ok(x) => {
                if x.1 > DEFAULT_PRINT_N {
                    DEFAULT_PRINT_N
                } else {
                    x.1 - 2
                }
            } // magic -2, otherwise no space for total file count because of newline at end of command
            _ => DEFAULT_PRINT_N,
        }
    }
}

fn init_terminal_space(config: &Config) {
    for _ in 0..config.nb_line + 1 {
        println!();
    }
}

fn main() {
    let mut config = Config::from_args();
    edit_config(&mut config);

    let mut storage: HashMap<String, NodeDir> = HashMap::new();
    storage.insert(config.path.to_owned(), NodeDir { size: 0 });

    let mut file_count = 0;
    let mut instant_next = Instant::now();
    if config.interval != 0 {
        instant_next = Instant::now() + Duration::from_millis(config.interval);
        init_terminal_space(&config);
    }

    let walkdir = WalkDir::new(&config.path)
        .same_file_system(true)
        .into_iter()
        .filter_map(|e| e.ok());
    for entry in walkdir {
        file_count += 1;
        process_dir_entry(&mut storage, &entry);
        if config.interval != 0 && instant_next < Instant::now() {
            update_print(&config, file_count, &storage);
            instant_next = Instant::now() + Duration::from_millis(config.interval);
        }
    }

    if config.interval == 0 {
        init_terminal_space(&config);
    }
    update_print(&config, file_count, &storage);
}
