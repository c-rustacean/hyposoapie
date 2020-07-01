# Intro

Replicate Yahoo! Pipes idea in command line and generate static pages.

## Requirements

* static pages
* refresh via some cron job
* as simple as possible (server side and in terms of implementation)

## Minimal PoC

* local rss (2-3 of them) (in file):
  * rss crate: https://crates.io/crates/rss
  * find-rss(?): https://crates.io/crates/find-rss
        * not maintained? alternatives?
* filter defined via some config file (toml), just a "contains keyword" initially
  * https://crates.io/crates/toml
* output rss with filtered turned into html/md (this could be a plugin-able exporter - occasion for using traits)?
  * for markdown output, just generate "by hand" as the output is simple enough



## From Andy's brain:

* schema is   xmlns="http://purl.org/rss/1.0/", and a proper parser should ignore the rest of the NS-es
* interesting "extension" is the dc ns: xmlns:dc="http://purl.org/dc/elements/1.1/"



## Brain dump of the config file format (toml)

The config file should be as simple as something like:

```toml
[sources]
rss1 = "https://bbc.co.uk"
rss2 = "https://orf.at"
rss3 = feed.xml

[filter]
electric = in:"rss1" contains:"electric car"
tesla = in:"rss1" contains:"tesla"

[filter]
unique = join:electric, tesla

[output]
combine = unique
```

This is "the UI", so make it simple!
