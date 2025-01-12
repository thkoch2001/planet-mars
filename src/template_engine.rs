use crate::feed_store::FeedStore;
use crate::to_checked_pathbuf;
use crate::Config;
use anyhow::Result;
use std::fs::File;
use tera::Tera;

pub fn build(config: &Config, feed_store: &FeedStore) -> Result<()> {
    let tera = create_tera(&config.templates_dir)?;
    let out_dir = to_checked_pathbuf(&config.out_dir);

    let mut context = tera::Context::new();
    let (feeds, entries) = feed_store.collect(&config.feeds);
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

fn create_tera(templates_dir: &str) -> Result<Tera> {
    let dir = to_checked_pathbuf(templates_dir);
    let mut tera = tera::Tera::new(&format!("{}/*", &dir.display()))?;
    // disable autoescape as this would corrupt urls or the entriy contents. todo check this!
    tera.autoescape_on(vec![]);
    Ok(tera)
}
