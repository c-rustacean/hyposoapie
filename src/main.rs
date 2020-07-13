use serde_derive::Deserialize;
use toml::Value;
// use std::io;
use std::fs::File;
use std::io::prelude::*;

#[derive(Debug)]
struct SourceName {
    name: String,
}

#[derive(Debug)]
struct RssSource {
    name: SourceName,
    url: String,
}

#[derive(Debug)]
enum FilterType {
    Contains(String),
}

#[derive(Debug)]
struct RssFilter {
    name: String,
    input: SourceName,
    filter: FilterType,
}

#[derive(Debug)]
struct Config {
    sources: Vec<RssSource>,
    filters: Vec<RssFilter>,
    output: Vec<SourceName>,
}

const CONFIG: &'static str = "hyposoapie.toml";

fn main() {
    let mut f = File::open(CONFIG).expect(&format!("Could not open config file {:?}", CONFIG));
    let mut toml_string = String::new();

    f.read_to_string(&mut toml_string)
        .expect("Could not read to string the config file");

    let toml: std::collections::BTreeMap<String, Value> = toml::from_str(&toml_string).unwrap();

    let sources: Vec<RssSource> = match toml.get("sources").expect("No sources found in config") {
        Value::Table(x) => x
            .iter()
            .map(|(name, v)| RssSource {
                name: SourceName { name: name.clone() },
                url: v.to_string(),
            })
            .collect(),
        _ => panic!("No sources found in config"),
    };
    let filters = toml.get("filters");
    let output = toml.get("output");

    println!("Sources: {:#?}", sources);
    println!("Filters: {:#?}", filters);
    println!("Output: {:#?}", output);
}
