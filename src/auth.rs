use std::io::{self, IsTerminal, Read};

use crate::client::PonyMailClient;
use crate::cookie::parse_ponymail_cookie;
use crate::error::{Error, Result};
use crate::models::{ListAddress, Session};

const KEYRING_SERVICE: &str = "asfml";
const KEYRING_ACCOUNT: &str = "lists.apache.org";

pub fn read_cookie_from_stdin() -> Result<String> {
    let input = if io::stdin().is_terminal() {
        rpassword::prompt_password(
            "Paste Cookie header or ponymail cookie value from lists.apache.org: ",
        )?
    } else {
        let mut input = String::new();
        io::stdin().read_to_string(&mut input)?;
        input
    };

    parse_ponymail_cookie(&input)
}

pub fn store_session(session: &Session) -> Result<()> {
    entry()?.set_password(&session.ponymail)?;
    Ok(())
}

pub fn load_session() -> Result<Session> {
    match entry()?.get_password() {
        Ok(ponymail) => Ok(Session { ponymail }),
        Err(keyring::Error::NoEntry) => Err(Error::NoSession),
        Err(err) => Err(err.into()),
    }
}

pub fn clear_session() -> Result<()> {
    match entry()?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(err) => Err(err.into()),
    }
}

pub fn validate_session(client: &PonyMailClient, list: Option<&ListAddress>) -> Result<String> {
    let prefs = client.preferences()?;
    let login = prefs
        .login
        .credentials
        .as_ref()
        .ok_or(Error::InvalidSession)?;
    if let Some(list) = list {
        if !prefs.has_list_access(list) {
            return Err(Error::NoListAccess(list.to_string()));
        }
    }

    let name = login
        .fullname
        .as_deref()
        .or(login.name.as_deref())
        .unwrap_or(&login.uid);
    Ok(match login.email {
        Some(ref email) if !email.is_empty() => format!("{name} <{email}>"),
        _ => name.to_string(),
    })
}

fn entry() -> Result<keyring::Entry> {
    Ok(keyring::Entry::new(KEYRING_SERVICE, KEYRING_ACCOUNT)?)
}
