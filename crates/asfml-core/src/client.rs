use reqwest::blocking::Client;
use url::Url;

use crate::error::{Error, Result};
use crate::models::{Email, ListAddress, Preferences, Session, StatsResponse, ThreadResponse};

const DEFAULT_BASE: &str = "https://lists.apache.org/";

pub struct PonyMailClient {
    http: Client,
    base: Url,
    session: Option<Session>,
}

impl PonyMailClient {
    pub fn new(session: Option<Session>) -> Result<Self> {
        Ok(Self {
            http: Client::builder().user_agent("asfml/0.1.0").build()?,
            base: Url::parse(DEFAULT_BASE)?,
            session,
        })
    }

    pub fn preferences(&self) -> Result<Preferences> {
        self.get_json("api/preferences.lua", &[])
    }

    pub fn list(
        &self,
        list: &ListAddress,
        since: &str,
        limit: usize,
    ) -> Result<Vec<crate::models::EmailSummary>> {
        let mut emails = self.stats(list, since, None)?;
        emails.truncate(limit);
        Ok(emails)
    }

    pub fn search(
        &self,
        list: &ListAddress,
        query: &str,
        since: &str,
        limit: usize,
    ) -> Result<Vec<crate::models::EmailSummary>> {
        let mut emails = self.stats(list, since, Some(query))?;
        emails.truncate(limit);
        Ok(emails)
    }

    pub fn email(&self, mid: &str) -> Result<Email> {
        let email: Email = self.get_json("api/email.lua", &[("id", mid)])?;
        if email.id.is_empty() {
            return Err(Error::EmailNotFound(mid.to_string()));
        }
        Ok(email)
    }

    pub fn thread(&self, mid: &str) -> Result<ThreadResponse> {
        self.get_json("api/thread.lua", &[("id", mid)])
    }

    fn stats(
        &self,
        list: &ListAddress,
        since: &str,
        query: Option<&str>,
    ) -> Result<Vec<crate::models::EmailSummary>> {
        let d = format!("lte={since}");
        let mut params = vec![
            ("list", list.list.as_str()),
            ("domain", list.domain.as_str()),
            ("d", d.as_str()),
            ("emailsOnly", "true"),
        ];
        if let Some(query) = query {
            params.push(("q", query));
        }

        let response: StatsResponse = self.get_json("api/stats.lua", &params)?;
        Ok(response.emails)
    }

    fn get_json<T>(&self, path: &str, params: &[(&str, &str)]) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        let mut url = self.base.join(path)?;
        {
            let mut pairs = url.query_pairs_mut();
            for (key, value) in params {
                pairs.append_pair(key, value);
            }
        }

        let mut request = self.http.get(url);
        if let Some(session) = &self.session {
            request = request.header(
                reqwest::header::COOKIE,
                format!("ponymail={}", session.ponymail),
            );
        }

        let response = request.send()?.error_for_status()?;
        let text = response.text()?;
        serde_json::from_str(&text).map_err(|err| Error::ApiShapeChanged {
            endpoint: endpoint_name(path),
            reason: err.to_string(),
        })
    }
}

fn endpoint_name(path: &str) -> &'static str {
    match path {
        "api/preferences.lua" => "preferences.lua",
        "api/stats.lua" => "stats.lua",
        "api/email.lua" => "email.lua",
        "api/thread.lua" => "thread.lua",
        _ => "unknown endpoint",
    }
}
