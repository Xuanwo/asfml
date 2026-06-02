# AGENTS.md

This file records project-specific context for future agents working on
`asfml`. It should capture decisions and operational lessons that are not
obvious from reading the code alone.

## Project Intent

`asfml` is a focused CLI for reading Apache Pony Mail archives on
`lists.apache.org`.

Keep the scope narrow:

- Support listing, searching, reading a mail, reading its parent/root parent,
  and reading its containing thread.
- Use Pony Mail `mid` values as the stable message identifier.
- Support private lists through a manually provided `ponymail` session cookie.
- Do not implement OAuth/OIDC login, browser automation, browser cookie-store
  extraction, config profiles, or mbox download unless a maintainer explicitly
  reopens those decisions.

The current design is documented in `docs/design.md`; treat that file as the
source of truth for command semantics.

## Maintenance Expectations

Use English for code, comments, commit messages, PR text, and repository
documentation.

When changing behavior, prefer a complete root-cause fix over a local patch.
Do not stop at a plan if the requested change is clear and low risk. Report
the actual commands and verification results.

When publishing or touching remote state, verify the remote state with `gh`,
registry CLIs, or direct registry queries. Do not infer success from local tags
or workflow names.

## Architecture Rules

Keep core behavior in `asfml-core`. The top-level `asfml` crate is the CLI
surface only.

Specifically, keep these responsibilities in `asfml-core`:

- Pony Mail API calls.
- Stable response models.
- Cookie parsing.
- Session storage.
- Parent/root/thread resolution.
- Error types that represent API/auth/session failures.

The CLI may parse arguments, read stdin, prompt for hidden cookie input, choose
output format, and render output. If a feature needs meaningful logic or tests,
it belongs in `asfml-core` first.

Do not add a config layer casually. `list` and `search` intentionally require a
positional `<list@domain>` so the operator does not accidentally query the wrong
mailing list.

## Pony Mail Behavior To Remember

Private Pony Mail archives do not behave like a normal authenticated API.
Without a valid authorized session, `stats.lua` for a private list can return
empty results instead of `401` or `403`.

Therefore:

- Before treating private-list `list` or `search` results as meaningful,
  validate the session with `preferences.lua` and verify the list is visible.
- Do not interpret an empty private-list search as "no matches" until access is
  known.
- `email.lua?id=` should use Pony Mail `mid`; do not depend on RFC
  `Message-ID` lookup even if Pony Mail documentation suggests it may work.
- Parent/root resolution should be done through `thread.lua` data and
  `In-Reply-To` relationships, not by searching for RFC message IDs.

## Authentication And Secrets

The accepted authentication model is manual cookie import:

```shell
asfml auth set
asfml auth set < cookies.txt
```

Accepted inputs include a raw `ponymail` value, `ponymail=<value>`, a full
`Cookie:` header, or Netscape cookies.txt content. Store only the `ponymail`
cookie for `lists.apache.org`.

Session storage is file-backed by default:

- macOS: `~/Library/Application Support/asfml/session.json`
- Linux: `$XDG_CONFIG_HOME/asfml/session.json` or `~/.config/asfml/session.json`
- Windows: `%APPDATA%\asfml\session.json`

`ASFML_SESSION_FILE` is only an override. Do not require it for normal use.
Do not reintroduce keyring storage unless a maintainer explicitly asks for it.

Never print cookie values in logs, errors, test output, snapshots, or debug
artifacts.

## Testing Policy

Unit tests must always run without network and without secrets. They should use
checked-in fixtures and sanitized snapshots.

Private list behavior is sensitive. Do not add or restore tests that perform
live access to private Apache lists. Do not commit real private mail data.
Only sanitized snapshots are acceptable, and they should preserve response
shape and relationship edge cases while redacting ids, subjects, senders,
message IDs, and bodies.

Public network integration tests are allowed only behind an explicit
environment gate:

```shell
ASFML_RUN_PUBLIC_API_TESTS=1 cargo test --test public_api
```

Default verification for ordinary code changes:

```shell
cargo fmt --all -- --check
cargo test --workspace
```

For larger changes, also run:

```shell
cargo clippy --workspace --all-targets -- -D warnings
```

## Release Model

Releases are tag-driven with tags named `vX.Y.Z`. The main release workflow
builds native artifacts and publishes crates.io packages; downstream
`workflow_run` workflows publish npm, PyPI, and Homebrew artifacts.

Important registry setup:

- crates.io trusted publishing is configured for both `asfml-core` and
  `asfml`, using GitHub environment `crates-io`.
- npm package name is `asfml`, using GitHub environment `npm`.
- PyPI package name is `asfml`, using GitHub environment `pypi`.
- Homebrew pushes to `Xuanwo/homebrew-tap` using
  `HOMEBREW_TAP_GITHUB_TOKEN`.

When verifying a release, check each surface independently:

- GitHub release and assets.
- crates.io packages for both crates.
- npm package and dist-tags.
- PyPI project files.
- Homebrew formula commit in `Xuanwo/homebrew-tap`.
- Relevant GitHub Actions runs.

Do not assume that a successful upstream release workflow means all downstream
package managers succeeded.

Once a version is published to crates.io, retesting the same tag has limited
value because crates.io versions are immutable. If downstream configuration is
fixed after the fact, it is usually better to finish the setup and validate on
the next release tag.

## npm Notes

The npm package uses a main `asfml` package plus platform-specific optional
packages with platform tags. Trusted publishing may require a recent npm CLI.
If configuring trusted publishing manually, prefer the npm CLI flow shown by
the registry and be prepared for 2FA.

The package name is exactly `asfml`; do not rename it or publish under a scope.

## PyPI Notes

The PyPI package name is exactly `asfml`. Use stable release tags such as
`v0.1.0`; prerelease or non-PEP-440-compatible versions can break PyPI
publishing.

## Homebrew Notes

Homebrew support publishes a formula to `Xuanwo/homebrew-tap`.

Lessons from the first release:

- Detect formula changes with `git status --porcelain -- Formula/asfml.rb`, not
  only `git diff --quiet`, because the formula may be a new untracked file.
- Generate formula files with exactly one trailing newline and no trailing
  blank line. `brew style` fails on trailing empty lines.
- The tap CI may fail `brew doctor` on Homebrew's tap trust transition unless
  `HOMEBREW_NO_REQUIRE_TAP_TRUST=1` is set in the test job environment.
- `brew tap xuanwo/tap` only ensures the tap exists. If the tap is already
  present, it may not update the local checkout to the latest formula.

For local verification after publishing:

```shell
brew update
brew info asfml
brew install asfml
asfml --version
```

If a local machine already has the tap but cannot find `asfml`, inspect and
update the tap checkout directly:

```shell
git -C "$(brew --repository xuanwo/tap)" status --short --branch
git -C "$(brew --repository xuanwo/tap)" pull --ff-only
```

Avoid running many Homebrew commands in parallel while debugging. If a local
Homebrew command appears stuck, inspect processes before starting another one:

```shell
ps -axo pid,ppid,stat,etime,command | rg '/opt/homebrew|Homebrew|brew.sh|shims/shared/curl|brew '
```

Only terminate the specific stuck Homebrew process group you started.

## Documentation Boundaries

Use `README.md` for user-facing install and quick-start examples.
Use `docs/design.md` for CLI semantics and implementation design.
Use `docs/release.md` for the intended release workflow.
Use this file for agent operating context, non-obvious decisions, and lessons
from prior maintenance or release work.
