# asfml

`asfml` is a CLI for reading Apache Pony Mail archives from `lists.apache.org`.

It supports public archives directly and private archives through a manually
provided Pony Mail session cookie.

## Installation

```bash
brew tap xuanwo/tap && brew install asfml   # Homebrew
cargo install asfml                         # Cargo
uv tool install asfml                       # Python / uv
npm install -g asfml                        # npm
```

## Quick Start

Run `asfml` directly without installing it first:

```bash
uvx asfml list dev@opendal.apache.org --limit 5
npx asfml list dev@opendal.apache.org --limit 5
```

Or use an installed binary:

```bash
asfml list dev@opendal.apache.org --limit 5
asfml search dev@opendal.apache.org release --limit 5
asfml read <mid>
```

## Usage

List recent messages:

```bash
asfml list dev@opendal.apache.org --limit 5
```

Search messages:

```bash
asfml search dev@opendal.apache.org release --limit 5
```

Read a message:

```bash
asfml read <mid>
asfml read <mid> --parent
asfml read <mid> --root
asfml read <mid> --thread
```

Authenticate for private archives:

```bash
asfml auth set
asfml auth status private@opendal.apache.org
```

`asfml auth set` accepts a raw `ponymail` cookie value, `ponymail=<value>`, a
full `Cookie:` header, or Netscape cookies.txt content.

## License

Licensed under the Apache License, Version 2.0.
