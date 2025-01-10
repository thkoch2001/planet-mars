#[macro_use]
extern crate log;

use crate::feed_store::FeedStore;
use crate::fetcher::Fetcher;
use clap::Parser;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use url::Url;

mod feed_store;
mod fetcher;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(
        short,
        long,
        default_value_t = String::from("mars.toml")
    )]
    config: String,
}

#[derive(Deserialize)]
struct Config {
    /// to be used as part of the fetchers username header
    bot_name: String,
    /// where to store downloaded feeds and their metadata
    feed_dir: String,
    /// feeds to be agregated
    feeds: Vec<FeedConfig>,
    /// Email adress to use for the from header when fetching feeds
    from: String,
    /// where to build the output files
    out_dir: String,
    /// templates folder
    templates_dir: String,
}

pub fn to_checked_pathbuf(dir: String) -> PathBuf {
    let dir: PathBuf = PathBuf::from(dir);

    let m = dir
        .metadata()
        .unwrap_or_else(|_| panic!("Could not get metadata of dir: {}", dir.display()));
    assert!(m.is_dir(), "Not a dir: {}", dir.display());
    dir
}

#[derive(Deserialize)]
struct FeedConfig {
    url: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    info!("starting up");

    let args = Args::parse();
    let config_path = &args.config;
    if !fs::exists(config_path)? {
        panic!("Configuration file {config_path} does not exist!");
    }
    let config: Config = toml::from_str(&fs::read_to_string(config_path)?)?;
    let templates_dir = to_checked_pathbuf(config.templates_dir);
    let out_dir = to_checked_pathbuf(config.out_dir);

    let feed_store = FeedStore::new(config.feed_dir);
    let fetcher = Fetcher::new(&config.bot_name, &config.from);

    let mut rebuild = false;
    for feed in &config.feeds {
        let url = Url::parse(&feed.url)?;
        rebuild |= fetcher.fetch(url, &feed_store);
    }
    info!("Done fetching. Rebuild needed: {rebuild}");
    if rebuild {
        let entries = feed_store.collect(&config.feeds);
        let mut tera = match tera::Tera::new(&format!("{}/*", &templates_dir.display())) {
            Ok(t) => t,
            Err(e) => {
                println!("Parsing error(s): {}", e);
                ::std::process::exit(1);
            }
        };
        tera.autoescape_on(vec![]);
        let mut context = tera::Context::new();
        context.insert("entries", &entries);
        for name in tera.get_template_names() {
            debug!("Processing template {name}");
            let file = fs::File::create(&format!("{}/{name}", out_dir.display()))?;
            let _ = tera.render_to(name, &context, file)?;
        }
    }
    Ok(())
}
