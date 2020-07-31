use std::cell::RefCell;
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

type QuElem = Option<Rc<QueueElement>>;

#[derive(Debug, Clone, PartialEq)]
struct QueueElement {
    name: String,
    next: RefCell<QuElem>,
}

impl QueueElement {
    fn new(name: String, next: &QuElem) -> Self {
        QueueElement {
            name,
            next: RefCell::new(if let Some(rc_qe) = next.clone() {
                let rc_qe = rc_qe.clone();
                Some(rc_qe)
            } else {
                None
            }),
        }
    }

    fn set_next(&self, next: &QuElem) {
        let link = next.clone();
        *self.next.borrow_mut() = link.clone();
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_queue_new_link() {
        let foo = Rc::new(QueueElement::new("foo".to_string(), &None));
        let bar = QueueElement::new("bar".to_owned(), &Some(foo.clone()));

        assert_eq!(&bar.next.borrow().as_ref().unwrap().as_ref(), &foo.as_ref());

        eprintln!("bar = {:#?}", bar);
    }

    #[test]
    fn test_queue_set_link() {
        let foo = Rc::new(QueueElement::new("foo".to_string(), &None));
        let bar = QueueElement::new("bar".to_owned(), &None);

        bar.set_next(&Some(foo.clone()));

        assert_eq!(&bar.next.borrow().as_ref().unwrap().as_ref(), &foo.as_ref());

        eprintln!("bar = {:#?}", bar);
    }
}

fn main() {
    let config = parse_config();

    // TODO: Create the chain of dependencies/processing from config
    // Idea: implement a trait for RSS feed type RssSource, RssFilter and output(?) so processing
    //       the entire chain is iterating over the trait

    let mut queue_head: QuElem = None;
    // let mut to_process: HashMap<String, Option<Vec<String>>> = HashMap::new();
    let mut seen_filters: HashSet<String> = HashSet::new();
    let mut opt_prev_qe: Option<Rc<QueueElement>> = None;

    for s in config.output.iter().map(|i| i.name().to_string()) {
        eprintln!(
            "s = {}\n\tqueue_head ??? = {:#?}\n\topt_prev_qe = {:#?}",
            &s,
            // &queue_head.unwrap() as const ptr*,
            &queue_head,
            // &opt_prev_qe.unwrap().as_ptr(),
            &opt_prev_qe
        );

        if !seen_filters.contains(&s) {
            seen_filters.insert(s.clone());

            let rc_qe = Rc::new(QueueElement::new(s, &None));

            if queue_head.is_none() {
                queue_head = Some(rc_qe.clone());
            }

            let opt_this_qe = Some(rc_qe.clone());
            if let Some(prev_qe) = opt_prev_qe {
                prev_qe.set_next(&opt_this_qe);
            }
            opt_prev_qe = opt_this_qe;
        }

        // TODO: check name is unseen
        // TODO: Add in HashSet the name,
        // TODO: link the qe in the list
    }

    println!("Config: {:#?}", config);
    // println!("To do: {:#?}", to_process);
    println!("Queue: {:#?}", queue_head);
}
