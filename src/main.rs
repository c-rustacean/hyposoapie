use std::collections::BTreeMap;
use std::fs::File;
use std::io::prelude::*;
use toml::Value;

#[derive(Debug, Clone)]
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

#[derive(Debug)]
struct QueueItem<'cfg> {
    name: &'cfg str,
    item_type: QueueItemType,
    is_output: bool,
}

#[derive(PartialEq, Copy, Clone, Debug)]
enum QueueItemType {
    Source,
    Filter,
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

                // TODO: Deal with filters with list of 'contains' words.
                //       Semantic that makes sense is 'OR' as 'AND' can
                //       be obtained via chaining filters
                let filter = FilterType::Contains(
                    filter_table
                        .get("contains")
                        .unwrap_or_else(|| {
                            panic!("No \"contains\" field in config for filter \"{}\"", &name)
                        })
                        .as_str()
                        .unwrap_or_else(|| {
                            panic!(
                                "\n\n    Could not turn to str in filter_table \"{}\".
    Maybe the filed is not a single string, but array?\n\n",
                                &name
                            )
                        })
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

fn compute_process_queue<'a>(config: &'a Config) -> Vec<QueueItem<'a>> {
    // Create the chain of dependencies/processing from config

    use std::collections::HashMap;
    use std::collections::HashSet;

    let mut process_queue = config.output.iter().map(|x| x.name()).collect::<Vec<_>>();

    let outputs = process_queue.len();

    let filter_names = config
        .filters
        .iter()
        .map(|x| x.name())
        .collect::<HashSet<_>>();

    let feed_names = config
        .sources
        .iter()
        .map(|x| x.name())
        .collect::<HashSet<_>>();

    let feeds_and_filters = feed_names
        .iter()
        .chain(filter_names.iter())
        .collect::<HashSet<_>>();

    if feeds_and_filters.len() < feed_names.len() + filter_names.len() {
        panic!("Duplicate name used for both feed and filter is not supported");
    }

    // dbg!(&feed_names);
    // dbg!(&filter_names);
    // dbg!(&feeds_and_filters);

    let mut seen: HashSet<&str> = HashSet::new();
    let mut queue_types: HashMap<&'a str, QueueItemType> = HashMap::new();
    let mut queue_extension: Vec<&str>;

    let mut next_index = 0usize;

    loop {
        queue_extension = Vec::new();

        for &item in process_queue.iter().skip(next_index) {
            use QueueItemType::*;

            if seen.contains(&item) {
                continue;
            }

            if filter_names.contains(&item) {
                let mut unique_filter_inputs = config
                    .filters
                    .iter()
                    .filter(|&x| x.name() == item)
                    .map(|x| &x.input)
                    .flatten()
                    .map(|x| x.name())
                    .filter(|&name| !seen.contains(&name))
                    .collect::<Vec<_>>();

                queue_types.insert(&item, Filter);

                queue_extension.append(&mut unique_filter_inputs);
            } else if feed_names.contains(&item) {
                let mut unique_feed_inputs = config
                    .sources
                    .iter()
                    .filter(|&x| x.name() == item)
                    .map(|x| x.name())
                    .filter(|&name| !seen.contains(&name))
                    .collect::<Vec<_>>();

                queue_types.insert(&item, Source);

                queue_extension.append(&mut unique_feed_inputs);
            } else {
                panic!("\n\n    Found an unknown filter or feed name: '{}'\n    Please check the configuration file!\n", &item);
            }

            // we've dealt with the current item but queue_extension might have other items
            seen.insert(&item);
        }

        next_index = process_queue.len();

        if queue_extension.len() == 0 {
            break;
        } else {
            // check we insert only unique names
            let mut new_items = HashSet::from(seen.clone());
            for new_item in queue_extension {
                if new_items.insert(new_item) {
                    process_queue.push(new_item);
                }
            }
        }
    }

    // later introduced items in the queue should be the resolvable ones
    process_queue.reverse();

    assert_eq!(process_queue.len(), queue_types.len());
    dbg!(&process_queue);

    let just_inputs = process_queue.len() - outputs;

    process_queue
        .iter()
        .enumerate()
        .map(|(index, &name)| QueueItem {
            name,
            item_type: queue_types.get(name).copied().unwrap(),
            is_output: index >= just_inputs,
        })
        .collect()
}

fn main() {
    let config = parse_config();

    // Idea: implement a trait for RSS feed type RssSource, RssFilter and output(?) so processing
    //       the entire chain is iterating over the trait

    let _process_queue = dbg!(compute_process_queue(&config));

    // TODO: Detect cycles in chains. How?
    //       Maybe each of the outputs from config should
    //       be processed independently, so we re-see an
    //       item. we have a cycle?

    // TODO: Make sure that if filter1 depends on filter2, and both are
    //       outputs, the order in the processing queue is reflecting the
    //       dependency (filter2 before filter1)

    // Solution (for both):
    // 1. iterate through the process_queue and resolve items
    // 2. if all items were processed, we're done
    // 3. count the number of resolved items and compare to the
    //    previous count (initially 0); if they are identical,
    //    then we're stuck - we have a cycle; repeat otherwise
}
