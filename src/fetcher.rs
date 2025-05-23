// SPDX-FileCopyrightText: 2025 Thomas Koch <thomas@koch.ro>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::Result;
use std::time::Instant;
use ureq::Agent;
use ureq::tls::{TlsConfig, TlsProvider};
use url::Url;

use crate::FeedStore;

pub struct Fetcher {
    agent: Agent,
    /// FROM header for requests
    from: String,
}

impl Fetcher {
    pub fn new(bot_name: &str, from: &str) -> Fetcher {
        let ua_name = format!(
            "{bot_name}/{} {} software: {}",
            env!("CARGO_PKG_VERSION"),
            env!("CARGO_PKG_HOMEPAGE"),
            env!("CARGO_PKG_NAME")
        );
        info!("useragent: {ua_name}");
        let agent = Agent::config_builder()
            .http_status_as_error(false)
            .user_agent(ua_name)
            .tls_config(
                TlsConfig::builder()
                    .provider(TlsProvider::NativeTls)
                    .build(),
            )
            .build()
            .into();
        Fetcher {
            agent,
            from: from.to_string(),
        }
    }

    pub fn fetch(&self, url: Url, feed_store: &FeedStore) -> Result<bool> {
        let fetchdata = feed_store.load_fetchdata(&url)?;
        let mut builder = self
            .agent
            .get(url.to_string())
            .header("FROM", self.from.clone());
        if !fetchdata.etag.is_empty() {
            builder = builder.header("If-None-Match", fetchdata.etag);
        }
        if !fetchdata.last_modified.is_empty() {
            builder = builder.header("If-Modified-Since", fetchdata.last_modified);
        }

        let start_instant = Instant::now();
        let result = builder.call();
        let duration = start_instant.elapsed();

        let response = result?;
        debug!(
            "fetched with status {} in {} ms: {url}",
            response.status(),
            duration.as_millis()
        );
        let status = response.status();
        match status.as_u16() {
            304 => Ok(false), // Not Modified -> nothing to do
            200 => feed_store.store(&url, response),
            _ => {
                warn!(
                    "HTTP Status {} not implemented for {url}",
                    response.status()
                );
                Ok(false)
            }
        }
    }
}
