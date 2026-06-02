# asfml Design

`asfml` is a small CLI for reading Apache Pony Mail archives through the
`lists.apache.org` API.

The MVP focuses on authenticated listing, search, and message/thread
inspection. It does not implement OAuth login, browser cookie extraction, local
configuration, or mbox download.

## Goals

- List recent emails from a specific mailing list.
- Search a specific mailing list.
- Read a specific archived email by Pony Mail `mid`.
- Read the direct parent of a specific email.
- Read the root parent of a specific email.
- Read the thread that contains a specific email.
- Keep authentication explicit and auditable.

## Non-Goals

- Do not implement OAuth/OIDC login.
- Do not automate browsers.
- Do not read browser cookie stores.
- Do not support persistent user config.
- Do not support mbox export in the MVP.
- Do not accept cookies via command-line arguments by default, to avoid shell
  history leaks.

## Authentication

`asfml` uses a manually provided Pony Mail session cookie.

The user logs in to `https://lists.apache.org` in a browser, copies either the
raw `ponymail` cookie value, a `ponymail=<value>` cookie, or a full `Cookie:`
header, and pastes it into:

```shell
asfml auth set
```

The CLI accepts:

- `ponymail=<value>`
- `<value>`
- `Cookie: ponymail=<value>; other=...`
- Netscape cookies.txt content

Only the `ponymail` cookie for `lists.apache.org` is retained. Other cookies are
discarded.

The cookie is stored either in the system keyring or in an explicit file-backed
session store. It must never be printed in logs or error messages.

## Authentication Commands

```text
asfml auth set
asfml auth status [<list@domain>]
asfml auth clear
```

Session storage defaults to auto-detection:

- if the default file-backed session exists, use the file store
- otherwise, use the system keyring

```shell
asfml auth set
asfml auth status private@opendal.apache.org
```

For Linux/headless environments without a Secret Service provider, use the file
store explicitly:

```shell
asfml --store file auth set
asfml --store file auth status private@opendal.apache.org
asfml --store file list private@opendal.apache.org
```

`ASFML_SESSION_STORE=file` makes the file store the default for all commands.
`ASFML_SESSION_FILE=/path/to/session.json` overrides the file-store path. Without
`ASFML_SESSION_FILE`, the file store uses the platform configuration directory:

- Linux: `$XDG_CONFIG_HOME/asfml/session.json` or `~/.config/asfml/session.json`
- macOS: `~/Library/Application Support/asfml/session.json`
- Windows: `%APPDATA%\asfml\session.json`

On Unix, the session file is written with `0600` permissions.

`auth set` prompts for one hidden input line when stdin is a terminal. If stdin
is piped, it reads the full stdin stream, which supports importing Netscape
cookies.txt content:

```shell
asfml auth set < cookies.txt
```

It then validates the session before storing it.

Example success output:

```text
Stored session for lists.apache.org.
Logged in as Xuanwo <xuanwo@apache.org>.
```

`auth status` calls:

```text
GET https://lists.apache.org/api/preferences.lua
```

It must verify the remote session, not only local session-store state.

When a list address is provided, it must also check that the requested list is
visible to the session. This is required because private list searches can
return empty results instead of an HTTP authorization error.

Example:

```text
Logged in as Xuanwo <xuanwo@apache.org>.
Access: private@opendal.apache.org yes
```

## User Commands

```text
asfml list <list@domain> [--since 30d] [--limit 50] [--format table|json]

asfml search <list@domain> [--since 1y] [--limit 50] [--format table|json] <query>

asfml read <mid> [--parent | --root | --thread] [--format text|json]
```

`<list@domain>` is a positional argument for `list` and `search`. There is no
default list because the MVP has no config layer and because accidentally
reading the wrong list can be misleading.

`list` returns recent emails from a mailing list. `search` returns emails that
match a query. `read` reads one email by default, or the email's parent, root
parent, or thread when a mode flag is supplied.

`--parent`, `--root`, and `--thread` are mutually exclusive.

## List

`list` calls `stats.lua` without a query:

```text
GET https://lists.apache.org/api/stats.lua?list=<list>&domain=<domain>&d=<range>&emailsOnly=true
```

Default output is a compact table:

```text
DATE                 MID                                  SUBJECT
2026-05-28 09:12     qd7m1k6h9hmjt5hdqb28y3vzh561x3bj      [DISCUSS] ...
```

`--format json` returns the parsed API data in a stable asfml schema, not the
raw Pony Mail response.

Before listing a private list, `asfml` must validate that the current session
can see the list. Empty `stats.lua` results are not sufficient evidence that the
list has no recent email.

## Search

`search` calls:

```text
GET https://lists.apache.org/api/stats.lua?list=<list>&domain=<domain>&d=<range>&emailsOnly=true&q=<query>
```

Default output uses the same table schema as `list`:

```text
DATE                 MID                                  SUBJECT
2026-05-28 09:12     qd7m1k6h9hmjt5hdqb28y3vzh561x3bj      [DISCUSS] ...
```

`--format json` returns the parsed API data in a stable asfml schema, not the
raw Pony Mail response.

Before searching a private list, `asfml` must validate that the current session
can see the list. Empty `stats.lua` results are not sufficient evidence that no
matching email exists.

## Read

`read <mid>` calls:

```text
GET https://lists.apache.org/api/email.lua?id=<mid>
```

Default output:

```text
Subject: ...
From: ...
Date: ...
List: private@opendal.apache.org
MID: ...

<body>
```

The CLI uses Pony Mail `mid` values, not RFC `Message-ID` headers, as primary
identifiers. Pony Mail documentation says `email.lua?id=` can accept RFC
`Message-ID`, but current `lists.apache.org` behavior is not reliable enough to
depend on it.

## Parent And Root

Parent lookup must be resolved inside the thread, not by calling `email.lua`
with an RFC `Message-ID`.

Resolution flow:

1. Call `email.lua?id=<mid>` for the target email.
2. Call `thread.lua?id=<mid>` for the containing thread.
3. Build a `message-id -> email` map from all returned thread emails.
4. For `read <mid> --parent`, use the target email's `in-reply-to` header to
   find the direct parent in that map.
5. For `read <mid> --root`, repeatedly follow parent links until the earliest
   available ancestor is found. If the Pony Mail thread tree has a root node,
   it can be used as a consistency check.

If the parent is not present in the archive, the command must say so explicitly:

```text
Parent not found in archive.
In-Reply-To: <...>
```

It must not present this as an empty search result.

## Thread

`read <mid> --thread` calls:

```text
GET https://lists.apache.org/api/thread.lua?id=<mid>
```

The command accepts a Pony Mail email `mid` and reads the thread that contains
that email.

Default output is optimized for reading the full discussion:

```text
Thread: ...
Messages: 6

[1/6] 2026-05-28 09:12  Alice
Subject: ...

<body>

[2/6] 2026-05-28 10:03  Bob
Subject: Re: ...

<body>
```

## Error Behavior

Missing session:

```text
No session found.
Run `asfml auth set` and paste your lists.apache.org ponymail cookie.
```

Expired or invalid session:

```text
Session expired or invalid.
Run `asfml auth set` with a fresh cookie from lists.apache.org.
```

No private list access:

```text
Logged in, but private@opendal.apache.org is not visible to this session.
```

Missing parent:

```text
Parent not found in archive.
In-Reply-To: <...>
```

API shape change:

```text
Pony Mail API response changed while reading <endpoint>.
Run with `--debug` to save the raw response.
```

## Rust Structure

```text
crates/asfml-core/
  src/lib.rs       # public core API
  src/auth.rs      # keyring-backed and file-backed session storage
  src/cookie.rs    # cookie parsing
  src/client.rs    # Pony Mail API client
  src/models.rs    # stable asfml data models and thread relation logic
  src/error.rs     # typed errors

src/
  main.rs          # clap dispatch and stdin interaction
  output.rs        # text/table/json rendering
```

All API access, authentication storage, cookie parsing, response models, and
parent/root/thread resolution live in `asfml-core`. The top-level `asfml` crate
is only the CLI surface.

The API client depends on an already resolved session:

```rust
pub struct Session {
    pub ponymail: String,
}

pub struct PonyMailClient {
    base: url::Url,
    session: Session,
}
```

Authentication, cookie parsing, API calls, and output rendering should remain
separate. This keeps future auth changes local to the auth layer.

## Suggested Dependencies

- `clap` for CLI parsing
- `reqwest` for HTTP
- `serde` and `serde_json` for data models
- `url` for URL construction
- `keyring` for keyring-backed session storage
- `thiserror` for typed errors
- `comfy-table` for table output
- `rpassword` for hidden cookie input

## Security Rules

- Never print the cookie.
- Redact cookie values in debug logs and errors.
- Do not pass cookies in query strings.
- Do not accept cookies through normal command-line flags in the default flow.
- Store only the `ponymail` cookie.
- Validate session state with `preferences.lua` before relying on it.

## Testing

Unit tests live in `asfml-core` and always run with `cargo test`. They use
checked-in public Pony Mail fixtures from `dev@opendal.apache.org`, including:

- recent list results
- `release` search results
- a single release discussion email
- the thread containing that email

They also include sanitized private-list snapshots derived from real
`private@opendal.apache.org` accesses. These snapshots preserve response shape,
list identity, and thread relation edge cases, but replace real ids, subjects,
senders, message ids, and bodies with redacted values.

Networked integration tests live in the top-level CLI test suite. They are
controlled by `ASFML_RUN_PUBLIC_API_TESTS=1`:

```shell
ASFML_RUN_PUBLIC_API_TESTS=1 cargo test --test public_api
```

Without that environment variable, integration tests do not access
`lists.apache.org`.

Private-list integration tests are separate and require a valid local session:

```shell
ASFML_RUN_PRIVATE_API_TESTS=1 cargo test --test private_api
```

These tests verify `auth status`, `list`, `search`, `read`, and `read --thread`
against `private@opendal.apache.org`, but they do not print private message
content.
