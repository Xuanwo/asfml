use std::collections::{HashMap, HashSet};
use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

#[derive(Debug, Clone)]
pub struct Session {
    pub ponymail: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ListAddress {
    pub list: String,
    pub domain: String,
}

impl ListAddress {
    pub fn parse(input: &str) -> Result<Self> {
        let Some((list, domain)) = input.split_once('@') else {
            return Err(Error::InvalidListAddress(input.to_string()));
        };
        if list.is_empty() || domain.is_empty() || domain.contains('@') {
            return Err(Error::InvalidListAddress(input.to_string()));
        }
        Ok(Self {
            list: list.to_string(),
            domain: domain.to_string(),
        })
    }
}

impl fmt::Display for ListAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}@{}", self.list, self.domain)
    }
}

#[derive(Debug, Deserialize)]
pub struct Preferences {
    #[serde(default)]
    pub login: Login,
    #[serde(default)]
    pub lists: HashMap<String, HashMap<String, serde_json::Value>>,
}

impl Preferences {
    pub fn has_list_access(&self, list: &ListAddress) -> bool {
        self.lists
            .get(&list.domain)
            .is_some_and(|lists| lists.contains_key(&list.list))
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct Login {
    pub credentials: Option<LoginCredentials>,
}

#[derive(Debug, Deserialize)]
pub struct LoginCredentials {
    #[serde(default)]
    pub uid: String,
    pub fullname: Option<String>,
    pub name: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct StatsResponse {
    #[serde(default)]
    pub emails: Vec<EmailSummary>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EmailSummary {
    pub id: String,
    #[serde(default)]
    pub mid: Option<String>,
    #[serde(default)]
    pub subject: String,
    #[serde(default)]
    pub from: String,
    #[serde(default)]
    pub epoch: Option<i64>,
    #[serde(default, rename = "list")]
    pub list_name: Option<String>,
}

impl EmailSummary {
    pub fn mid(&self) -> &str {
        self.mid.as_deref().unwrap_or(&self.id)
    }

    pub fn formatted_date(&self) -> String {
        format_epoch(self.epoch)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Email {
    pub id: String,
    #[serde(default)]
    pub mid: Option<String>,
    #[serde(default)]
    pub subject: String,
    #[serde(default)]
    pub from: String,
    #[serde(default)]
    pub date: Option<String>,
    #[serde(default)]
    pub epoch: Option<i64>,
    #[serde(default, rename = "list")]
    pub list_name: Option<String>,
    #[serde(default)]
    pub body: String,
    #[serde(default, rename = "message-id")]
    pub message_id: Option<String>,
    #[serde(default, rename = "in-reply-to")]
    pub in_reply_to: Option<String>,
    #[serde(default)]
    pub references: Option<String>,
}

impl Email {
    pub fn mid(&self) -> &str {
        self.mid.as_deref().unwrap_or(&self.id)
    }

    pub fn message_id_key(&self) -> Option<&str> {
        non_empty(self.message_id.as_deref())
    }

    pub fn in_reply_to_key(&self) -> Option<&str> {
        non_empty(self.in_reply_to.as_deref())
    }

    pub fn formatted_date(&self) -> String {
        self.date
            .clone()
            .unwrap_or_else(|| format_epoch(self.epoch))
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ThreadResponse {
    pub thread: Email,
    #[serde(default)]
    pub emails: Vec<Email>,
}

impl ThreadResponse {
    pub fn find_email(&self, mid: &str) -> Option<&Email> {
        self.emails
            .iter()
            .find(|email| email.id == mid || email.mid.as_deref() == Some(mid))
            .or_else(|| {
                (self.thread.id == mid || self.thread.mid.as_deref() == Some(mid))
                    .then_some(&self.thread)
            })
    }

    pub fn direct_parent(&self, mid: &str) -> Result<&Email> {
        let target = self
            .find_email(mid)
            .ok_or_else(|| Error::EmailNotFound(mid.to_string()))?;
        let in_reply_to = target
            .in_reply_to_key()
            .ok_or_else(|| Error::ParentNotFound {
                in_reply_to: String::new(),
            })?;
        self.by_message_id()
            .get(in_reply_to)
            .copied()
            .ok_or_else(|| Error::ParentNotFound {
                in_reply_to: in_reply_to.to_string(),
            })
    }

    pub fn root_parent(&self, mid: &str) -> Result<&Email> {
        let by_message_id = self.by_message_id();
        let mut current = self
            .find_email(mid)
            .ok_or_else(|| Error::EmailNotFound(mid.to_string()))?;
        let mut seen = HashSet::new();

        loop {
            let Some(in_reply_to) = current.in_reply_to_key() else {
                return Ok(current);
            };
            if !seen.insert(in_reply_to.to_string()) {
                return Ok(current);
            }
            let Some(parent) = by_message_id.get(in_reply_to).copied() else {
                if current.id == mid || current.mid.as_deref() == Some(mid) {
                    return Err(Error::ParentNotFound {
                        in_reply_to: in_reply_to.to_string(),
                    });
                }
                return Ok(current);
            };
            current = parent;
        }
    }

    fn by_message_id(&self) -> HashMap<&str, &Email> {
        let mut map = HashMap::new();
        if let Some(message_id) = self.thread.message_id_key() {
            map.insert(message_id, &self.thread);
        }
        for email in &self.emails {
            if let Some(message_id) = email.message_id_key() {
                map.insert(message_id, email);
            }
        }
        map
    }
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.and_then(|value| {
        let value = value.trim();
        (!value.is_empty()).then_some(value)
    })
}

fn format_epoch(epoch: Option<i64>) -> String {
    epoch
        .and_then(|epoch| DateTime::<Utc>::from_timestamp(epoch, 0))
        .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_else(|| "-".to_string())
}

#[cfg(test)]
mod tests {
    use super::{Email, ListAddress, ThreadResponse};

    #[test]
    fn parse_list_address() {
        let addr = ListAddress::parse("private@opendal.apache.org").unwrap();
        assert_eq!(addr.list, "private");
        assert_eq!(addr.domain, "opendal.apache.org");
    }

    #[test]
    fn resolve_parent_and_root() {
        let root = email("root", "<root>", None);
        let child = email("child", "<child>", Some("<root>"));
        let grandchild = email("grandchild", "<grandchild>", Some("<child>"));
        let thread = ThreadResponse {
            thread: root.clone(),
            emails: vec![root, child, grandchild],
        };

        let parent = thread.direct_parent("grandchild").unwrap();
        assert_eq!(parent.mid(), "child");

        let root = thread.root_parent("grandchild").unwrap();
        assert_eq!(root.mid(), "root");
    }

    fn email(id: &str, message_id: &str, in_reply_to: Option<&str>) -> Email {
        Email {
            id: id.to_string(),
            mid: Some(id.to_string()),
            subject: String::new(),
            from: String::new(),
            date: None,
            epoch: None,
            list_name: None,
            body: String::new(),
            message_id: Some(message_id.to_string()),
            in_reply_to: in_reply_to.map(ToString::to_string),
            references: None,
        }
    }
}
