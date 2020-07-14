use std::collections::BTreeMap;
use std::fs::File;
use std::io::prelude::*;
use toml::Value;

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
    input: Vec<SourceName>,
    filter: FilterType,
}

#[derive(Debug)]
struct Config {
    sources: Vec<RssSource>,
    filters: Vec<RssFilter>,
    output: Vec<SourceName>,
}

const CONFIG: &str = "hyposoapie.toml";

fn get_config_as_string() -> String {
    let mut f =
        File::open(CONFIG).unwrap_or_else(|_| panic!("Could not open config file {:?}", CONFIG));
    let mut toml_string = String::new();

    f.read_to_string(&mut toml_string)
        .expect("Could not read to string the config file");

    toml_string
}

fn get_sources(toml: &BTreeMap<String, Value>) -> Vec<RssSource> {
    match toml.get("sources").expect("No sources found in config") {
        Value::Table(x) => x
            .iter()
            .map(|(name, v)| RssSource {
                name: SourceName { name: name.clone() },
                url: v.to_string().replace("\"", ""),
            })
            .collect(),
        _ => panic!("No sources found in config"),
    }
}

fn get_filters(toml: &BTreeMap<String, Value>) -> Vec<RssFilter> {
    match toml.get("filter").expect("No filters found in config") {
        Value::Table(filters_map) => filters_map
            .into_iter()
            .map(|(name, v)| {
                let filter_table = v.as_table().unwrap_or_else(|| {
                    panic!("Expected to be able to get the Table for filter {}", &name)
                });

                let input = filter_table
                    .get("in")
                    .unwrap_or_else(|| panic!("No 'in' feed specified for filter {}", &name))
                    .as_array()
                    .expect("Could not unwrap the array of input rss-es")
                    .iter()
                    .map(|v| SourceName {
                        name: v.as_str().unwrap().to_string(),
                    })
                    .collect::<Vec<_>>();

                let filter = FilterType::Contains(
                    filter_table
                        .get("contains")
                        .expect("No 'contains' field in config")
                        .as_str()
                        .unwrap()
                        .to_string(),
                );

                RssFilter {
                    name: name.to_string(),
                    input,
                    filter,
                }
            })
            .collect::<Vec<_>>(),
        _ => panic!("Filter table contains errors!"),
    }
}

fn get_outputs(toml: &BTreeMap<String, Value>) -> Vec<SourceName> {
    toml.get("output")
        .unwrap_or_else(|| panic!("Unable to find 'output' table in configuration"))
        .as_table()
        .unwrap()
        .get("combine")
        .unwrap()
        .as_array()
        .unwrap()
        .into_iter()
        .map(|v| SourceName {
            name: v.to_string().replace("\"", ""),
        })
        .collect::<Vec<_>>()
}

fn parse_config() -> Config {
    let toml: BTreeMap<String, Value> = toml::from_str(&get_config_as_string()).unwrap();

    let sources = get_sources(&toml);
    let filters = get_filters(&toml);
    let output = get_outputs(&toml);

    Config {
        sources,
        filters,
        output,
    }
}

fn main() {
    let _config = parse_config();
}
