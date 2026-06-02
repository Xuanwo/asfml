use comfy_table::{Cell, Table, presets::UTF8_FULL};
use serde::Serialize;

use asfml_core::{Email, EmailSummary, Error, Result, ThreadResponse};

#[derive(Debug, Clone, Copy, Eq, PartialEq, clap::ValueEnum)]
pub enum TableFormat {
    Table,
    Json,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, clap::ValueEnum)]
pub enum ReadFormat {
    Text,
    Json,
}

pub fn print_summaries(emails: &[EmailSummary], format: TableFormat) -> Result<()> {
    match format {
        TableFormat::Json => print_json(emails),
        TableFormat::Table => {
            let mut table = Table::new();
            table.load_preset(UTF8_FULL);
            table.set_header(["DATE", "MID", "SUBJECT"]);
            for email in emails {
                table.add_row([
                    Cell::new(email.formatted_date()),
                    Cell::new(email.mid()),
                    Cell::new(&email.subject),
                ]);
            }
            println!("{table}");
            Ok(())
        }
    }
}

pub fn print_email(email: &Email, format: ReadFormat) -> Result<()> {
    match format {
        ReadFormat::Json => print_json(email),
        ReadFormat::Text => {
            println!("Subject: {}", email.subject);
            println!("From: {}", email.from);
            println!("Date: {}", email.formatted_date());
            if let Some(list) = &email.list_name {
                println!("List: {list}");
            }
            println!("MID: {}", email.mid());
            if let Some(message_id) = &email.message_id {
                println!("Message-ID: {message_id}");
            }
            if let Some(in_reply_to) = email.in_reply_to_key() {
                println!("In-Reply-To: {in_reply_to}");
            }
            println!();
            print!("{}", email.body);
            if !email.body.ends_with('\n') {
                println!();
            }
            Ok(())
        }
    }
}

pub fn print_thread(thread: &ThreadResponse, format: ReadFormat) -> Result<()> {
    match format {
        ReadFormat::Json => print_json(thread),
        ReadFormat::Text => {
            let emails = if thread.emails.is_empty() {
                vec![&thread.thread]
            } else {
                thread.emails.iter().collect::<Vec<_>>()
            };
            println!("Thread: {}", thread.thread.subject);
            println!("Messages: {}", emails.len());
            println!();
            for (idx, email) in emails.iter().enumerate() {
                println!(
                    "[{}/{}] {}  {}",
                    idx + 1,
                    emails.len(),
                    email.formatted_date(),
                    email.from
                );
                println!("Subject: {}", email.subject);
                println!("MID: {}", email.mid());
                println!();
                print!("{}", email.body);
                if !email.body.ends_with('\n') {
                    println!();
                }
                if idx + 1 != emails.len() {
                    println!();
                }
            }
            Ok(())
        }
    }
}

pub fn print_error(error: &Error) {
    match error {
        Error::NoSession => {
            eprintln!("No session found.");
            eprintln!("Run `asfml auth set` and paste your lists.apache.org ponymail cookie.");
        }
        Error::InvalidSession => {
            eprintln!("Session expired or invalid.");
            eprintln!("Run `asfml auth set` with a fresh cookie from lists.apache.org.");
        }
        Error::NoListAccess(list) => {
            eprintln!("Logged in, but {list} is not visible to this session.");
        }
        Error::ParentNotFound { in_reply_to } => {
            eprintln!("Parent not found in archive.");
            if !in_reply_to.is_empty() {
                eprintln!("In-Reply-To: {in_reply_to}");
            }
        }
        Error::ApiShapeChanged { endpoint, .. } => {
            eprintln!("Pony Mail API response changed while reading {endpoint}.");
            eprintln!("Run with `--debug` to save the raw response.");
        }
        _ => eprintln!("{error}"),
    }
}

fn print_json<T: Serialize + ?Sized>(value: &T) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}
