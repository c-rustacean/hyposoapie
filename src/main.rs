use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs::File;
use std::io::prelude::*;
use std::rc::Rc;
use toml::Value;

#[derive(Debug, Clone)]
struct SourceName {
    name: String,
}

#[derive(Debug, Clone)]
struct RssSource {
    name: SourceName,
    url: String,
}

#[derive(Debug, Clone)]
enum FilterType {
    Contains(String),
}

#[derive(Debug, Clone)]
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

trait Inputs {
    fn inputs(&self) -> Vec<String>;
}

impl Inputs for RssFilter {
    fn inputs(&self) -> Vec<String> {
        self.input.iter().map(|i| i.name().to_owned()).collect()
    }
}

impl Inputs for RssSource {
    fn inputs(&self) -> Vec<String> {
        vec![self.url.clone()]
    }
}

trait Name {
    fn name(&self) -> &str;
}

macro_rules! name {
    ($t:ty) => {
        impl Name for $t {
            fn name(&self) -> &str {
                &(self.name)
            }
        }
    };
}

name!(SourceName);
name!(RssFilter);

impl Name for RssSource {
    fn name(&self) -> &str {
        self.name.name()
    }
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
    let sources: Vec<_> = match toml.get("sources").expect("No sources found in config") {
        Value::Table(x) => x
            .iter()
            .map(|(name, v)| RssSource {
                name: SourceName { name: name.clone() },
                url: v.to_string().replace("\"", ""),
            })
            .collect(),
        _ => panic!("No sources found in config"),
    };

    if sources.len() == 0 {
        panic!("Sources section from config is empty")
    } else {
        sources
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
                    .unwrap_or_else(|| panic!("No \"in\" feed specified for filter \"{}\"", &name))
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
                        .unwrap_or_else(|| {
                            panic!("No \"contains\" field in config for filter \"{}\"", &name)
                        })
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
    let output_feeds = toml
        .get("output")
        .unwrap_or_else(|| panic!("Unable to find 'output' table in configuration"))
        .as_table()
        .unwrap()
        .get("combine")
        .unwrap_or_else(|| panic!("No \"combine\" field found in config output section"))
        .as_array()
        .unwrap_or_else(|| panic!("Field \"combine\" in \"output\" section should be an array"))
        .into_iter()
        .map(|v| SourceName {
            name: v.to_string().replace("\"", ""),
        })
        .collect::<Vec<_>>();

    if output_feeds.len() == 0 {
        panic!("No feeds in output config would generate an empty page")
    } else {
        output_feeds
    }
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

impl Config {
    fn is_filter(&self, name: &str) -> bool {
        self.filters.iter().any(|x| x.name() == name)
    }

    fn is_source(&self, name: &str) -> bool {
        self.sources.iter().any(|x| x.name() == name)
    }
}

struct QueueElement {
    name: String,
    next: Option<Rc<QueueElement>>,
}

impl QueueElement {
    fn new(name: String, next: &Option<Rc<QueueElement>>) -> Self {
        if let Some(rc_qe) = next {
            let rc_qe = rc_qe.clone();
            QueueElement {
                name,
                next: Some(rc_qe),
            }
        } else {
            QueueElement { name, next: None }
        }
    }
}

fn main() {
    let config = parse_config();

    // TODO: Create the chain of dependencies/processing from config
    // Idea: implement a trait for RSS feed type RssSource, RssFilter and output(?) so processing
    //       the entire chain is iterating over the trait

    let queue_head: Option<QueueElement> = None;
    let mut to_process: HashMap<String, Option<Vec<String>>> = HashMap::new();
    let mut _seen_filters: HashSet<String> = HashSet::new();
    let mut prev_qe = None;

    for s in config.output.iter().map(|i| i.name().to_string()) {
        let qe = QueueElement::new(s, &prev_qe);

        // TODO: check name is unseen
        // TODO: Add in HashSet the name,
        // TODO: link the qe in the list
    }

    println!("Config: {:#?}", config);
    println!("To do: {:#?}", to_process);
}
