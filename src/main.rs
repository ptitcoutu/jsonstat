use crate::json_stat_extractor::{extract_stat_from_json, JsonStat};
use std::env::args;
use std::fs::File;
use std::io::{stdin, BufReader};

mod json_stat_extractor;

fn main() {
    let mut args = args();
    let args_length = args.len();
    let json_stat: JsonStat = if args_length > 1 {
        let file_name = args.nth(1).unwrap();
        println!("will parse {file_name}");
        let file = File::open(file_name).unwrap();
        let file_reader = BufReader::new(file);
        extract_stat_from_json(file_reader)
    } else {
        extract_stat_from_json(stdin())
    };
    let json_stat_in_json = serde_json::to_string_pretty(&json_stat).unwrap();
    println!("{json_stat_in_json}")
}
