#[macro_use]
extern crate log;

use crate::feed_store::FeedStore;
use crate::fetcher::Fetcher;
use anyhow::Result;
use clap::Parser;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use url::Url;

//mod atom_serializer;
mod feed_store;
mod fetcher;
mod template_engine;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(
        short,
        long,
        default_value_t = String::from("mars.toml")
    )]
    config: String,
    #[arg(long, default_value_t = false)]
    no_fetch: bool,
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

pub fn to_checked_pathbuf(dir: &str) -> PathBuf {
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

fn fetch(config: &Config, feed_store: &FeedStore) -> Result<bool> {
    let fetcher = Fetcher::new(&config.bot_name, &config.from);
    let mut rebuild = false;
    for feed in &config.feeds {
        let url = match Url::parse(&feed.url) {
            Ok(x) => x,
            Err(e) => {
                error!("Error parsing url '{}': {e:?}", feed.url);
                continue;
            }
        };
        rebuild |= fetcher.fetch(url, feed_store)?;
    }
    info!("Done fetching. Rebuild needed: {rebuild}");
    Ok(rebuild)
}

fn main() -> Result<()> {
    env_logger::init();
    info!("starting up");

    let args = Args::parse();
    let config_path = &args.config;
    if !fs::exists(config_path)? {
        panic!("Configuration file {config_path} does not exist!");
    }
    let config: Config = toml::from_str(&fs::read_to_string(config_path)?)?;
    // only check here to avoid fetching with broken config
    // todo: get a config lib that provides validation!
    let _ = to_checked_pathbuf(&config.templates_dir);
    let _ = to_checked_pathbuf(&config.out_dir);

    let feed_store = FeedStore::new(&config.feed_dir);
    let should_build = if args.no_fetch {
        true
    } else {
        fetch(&config, &feed_store)?
    };

    if should_build {
        template_engine::build(&config, &feed_store)?;
    }
    Ok(())
}
