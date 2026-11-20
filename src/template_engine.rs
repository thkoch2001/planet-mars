// SPDX-FileCopyrightText: 2025 Thomas Koch <thomas@koch.ro>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::Config;
use crate::feed_store::FeedStore;
use crate::to_checked_pathbuf;
use anyhow::Result;
use feed_rs::model::Feed;
use std::collections::HashMap;
use std::fs::File;
use tera::{Kwargs, State, Tera, TeraResult, value::Key};

pub fn build(config: &Config, feed_store: &FeedStore) -> Result<()> {
    let (feeds, entries): (HashMap<String, Feed>, _) =
        feed_store.collect(&config.feeds, config.max_entries);

    let tera = create_tera(&config.templates_dir, &feeds)?;
    let out_dir = to_checked_pathbuf(&config.out_dir);

    let mut context = tera::Context::new();
    context.insert("feeds", &feeds);
    context.insert("entries", &entries);
    context.insert("PKG_AUTHORS", env!("CARGO_PKG_AUTHORS"));
    context.insert("PKG_HOMEPAGE", env!("CARGO_PKG_HOMEPAGE"));
    context.insert("PKG_NAME", env!("CARGO_PKG_NAME"));
    context.insert("PKG_VERSION", env!("CARGO_PKG_VERSION"));

    for name in tera.get_template_names() {
        debug!("Processing template {name}");
        let file = File::create(format!("{}/{name}", out_dir.display()))?;
        tera.render_to(name, &context, file)?;
    }
    Ok(())
}

fn create_tera(templates_dir: &str, feeds: &HashMap<String, Feed>) -> Result<Tera> {
    let dir = to_checked_pathbuf(templates_dir);
    let mut tera = Tera::default();

    // disable autoescape as this would corrupt urls or the entry contents. todo check this!
    tera.autoescape_on(Vec::<&str>::new());
    tera.register_function("now", tera_contrib::dates::now);
    tera.register_filter("date", tera_contrib::dates::date);
    tera.register_filter("striptags", tera_contrib::regex::striptags);
    tera.register_function(
        "get_author",
        GetAuthorFunction {
            feeds: feeds.clone(),
        },
    );
    tera.load_from_glob(&format!("{}/*", &dir.display()))?;
    Ok(tera)
}

struct GetAuthorFunction {
    feeds: HashMap<String, Feed>,
}

impl tera::Function<TeraResult<String>> for GetAuthorFunction {
    fn call(&self, kwargs: Kwargs, _: &State) -> TeraResult<String> {
        let entry_val: tera::Map = kwargs.must_get("entry")?;
        let feed_url: String = entry_val
            .get(&Key::from("source"))
            .unwrap()
            .as_str()
            .unwrap()
            .to_string();
        let authors_val: &[tera::Value] = entry_val
            .get(&Key::from("authors"))
            .unwrap()
            .as_array()
            .unwrap();

        let mut authors: Vec<String> = Vec::new();
        for author_val in authors_val {
            let name: String = author_val
                .as_map()
                .unwrap()
                .get(&Key::from("name"))
                .unwrap()
                .as_str()
                .unwrap()
                .to_string();
            if is_valid_name(&name) {
                authors.push(name.clone());
            }
        }

        if authors.is_empty() {
            authors.append(&mut self.find_authors_from_feed(&feed_url));
        }
        Ok(authors.join(", "))
    }
}

impl GetAuthorFunction {
    fn find_authors_from_feed(&self, feed_url: &str) -> Vec<String> {
        let feed = self.feeds.get(feed_url).unwrap();

        feed.authors
            .clone()
            .into_iter()
            .map(|x| x.name.unwrap_or_default())
            .filter(is_valid_name)
            .collect()
    }
}

fn is_valid_name(n: &String) -> bool {
    !n.is_empty() && n != "unknown" && n != "author"
}
