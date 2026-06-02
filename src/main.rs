mod output;

use std::io::{self, IsTerminal, Read};

use asfml_core::{
    Error, ListAddress, PonyMailClient, Result, Session, clear_session, load_session,
    parse_ponymail_cookie, store_session, validate_session,
};
use clap::{Args, Parser, Subcommand};

use crate::output::{
    ReadFormat, TableFormat, print_email, print_error, print_summaries, print_thread,
};

#[derive(Debug, Parser)]
#[command(
    version,
    about = "Read Apache Pony Mail archives from lists.apache.org"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    #[command(about = "Manage the stored lists.apache.org session")]
    Auth(AuthCommand),
    #[command(about = "List recent emails from a mailing list")]
    List(ListCommand),
    #[command(about = "Search emails in a mailing list")]
    Search(SearchCommand),
    #[command(about = "Read an email, its parent/root, or its thread")]
    Read(ReadCommand),
}

#[derive(Debug, Args)]
struct AuthCommand {
    #[command(subcommand)]
    command: AuthSubcommand,
}

#[derive(Debug, Subcommand)]
enum AuthSubcommand {
    #[command(about = "Store a manually copied ponymail cookie")]
    Set,
    #[command(about = "Validate the stored session")]
    Status { list: Option<String> },
    #[command(about = "Delete the stored session")]
    Clear,
}

#[derive(Debug, Args)]
struct ListCommand {
    list: String,

    #[arg(long, default_value = "30d")]
    since: String,

    #[arg(long, default_value_t = 50)]
    limit: usize,

    #[arg(long, value_enum, default_value_t = TableFormat::Table)]
    format: TableFormat,
}

#[derive(Debug, Args)]
struct SearchCommand {
    list: String,

    query: String,

    #[arg(long, default_value = "1y")]
    since: String,

    #[arg(long, default_value_t = 50)]
    limit: usize,

    #[arg(long, value_enum, default_value_t = TableFormat::Table)]
    format: TableFormat,
}

#[derive(Debug, Args)]
struct ReadCommand {
    mid: String,

    #[arg(long, conflicts_with_all = ["root", "thread"])]
    parent: bool,

    #[arg(long, conflicts_with_all = ["parent", "thread"])]
    root: bool,

    #[arg(long, conflicts_with_all = ["parent", "root"])]
    thread: bool,

    #[arg(long, value_enum, default_value_t = ReadFormat::Text)]
    format: ReadFormat,
}

fn main() {
    if let Err(error) = run() {
        print_error(&error);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Auth(command) => handle_auth(command),
        Command::List(command) => handle_list(command),
        Command::Search(command) => handle_search(command),
        Command::Read(command) => handle_read(command),
    }
}

fn handle_auth(command: AuthCommand) -> Result<()> {
    match command.command {
        AuthSubcommand::Set => {
            let ponymail = read_cookie_from_stdin()?;
            let session = Session { ponymail };
            let client = PonyMailClient::new(Some(session.clone()))?;
            let user = validate_session(&client, None)?;
            store_session(&session)?;
            println!("Stored session for lists.apache.org.");
            println!("Logged in as {user}.");
            Ok(())
        }
        AuthSubcommand::Status { list } => {
            let session = load_session()?;
            let client = PonyMailClient::new(Some(session))?;
            let list = parse_optional_list(list)?;
            let user = validate_session(&client, list.as_ref())?;
            println!("Logged in as {user}.");
            if let Some(list) = list {
                println!("Access: {list} yes");
            }
            Ok(())
        }
        AuthSubcommand::Clear => {
            clear_session()?;
            println!("Cleared session for lists.apache.org.");
            Ok(())
        }
    }
}

fn handle_list(command: ListCommand) -> Result<()> {
    let list = ListAddress::parse(&command.list)?;
    let client = client_for_list(&list)?;
    let emails = client.list(&list, &command.since, command.limit)?;
    print_summaries(&emails, command.format)
}

fn handle_search(command: SearchCommand) -> Result<()> {
    let list = ListAddress::parse(&command.list)?;
    let client = client_for_list(&list)?;
    let emails = client.search(&list, &command.query, &command.since, command.limit)?;
    print_summaries(&emails, command.format)
}

fn handle_read(command: ReadCommand) -> Result<()> {
    let client = client_with_optional_session()?;
    if command.parent {
        let thread = client.thread(&command.mid)?;
        let parent = thread.direct_parent(&command.mid)?;
        print_email(parent, command.format)
    } else if command.root {
        let thread = client.thread(&command.mid)?;
        let root = thread.root_parent(&command.mid)?;
        print_email(root, command.format)
    } else if command.thread {
        let thread = client.thread(&command.mid)?;
        print_thread(&thread, command.format)
    } else {
        let email = client.email(&command.mid)?;
        print_email(&email, command.format)
    }
}

fn client_for_list(list: &ListAddress) -> Result<PonyMailClient> {
    let session = match load_session() {
        Ok(session) => Some(session),
        Err(Error::NoSession) => None,
        Err(error) => return Err(error),
    };
    let client = PonyMailClient::new(session)?;
    let prefs = client.preferences()?;
    if prefs.has_list_access(list) {
        return Ok(client);
    }
    if list.list == "private" {
        return Err(Error::NoSession);
    }
    Err(Error::NoListAccess(list.to_string()))
}

fn client_with_optional_session() -> Result<PonyMailClient> {
    let session = match load_session() {
        Ok(session) => Some(session),
        Err(Error::NoSession) => None,
        Err(error) => return Err(error),
    };
    PonyMailClient::new(session)
}

fn parse_optional_list(list: Option<String>) -> Result<Option<ListAddress>> {
    list.as_deref().map(ListAddress::parse).transpose()
}

fn read_cookie_from_stdin() -> Result<String> {
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
