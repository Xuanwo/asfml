# Release

Releases are driven by tags named `vX.Y.Z`.

Pushing a release tag runs `.github/workflows/release.yml`, which:

- injects the tag version into Cargo manifests;
- builds native binaries for Linux, macOS, and Windows on x86_64 and arm64;
- publishes GitHub release archives, checksums, and `manifest.json`;
- publishes `asfml-core` to crates.io first, waits for it to become visible,
  then publishes the `asfml` CLI crate.

After the GitHub release workflow succeeds, downstream `workflow_run` workflows
publish package-manager artifacts:

- npm: publishes `asfml` plus platform-specific alias packages under the same
  package name with platform-suffixed versions;
- PyPI: publishes platform wheels for the `asfml` package;
- Homebrew: updates `Formula/asfml.rb` in `Xuanwo/homebrew-tap`.

## Required Setup

Configure trusted publishing before the first release:

- crates.io trusted publishing for `asfml-core` and `asfml`, using the GitHub
  environment named `crates-io`;
- npm trusted publishing for package `asfml`, using the GitHub environment named
  `npm`;
- PyPI trusted publishing for project `asfml`, using the GitHub environment
  named `pypi`;
- repository secret `HOMEBREW_TAP_GITHUB_TOKEN`, with permission to push to
  `Xuanwo/homebrew-tap`.

Use stable release tags such as `v0.1.0` for PyPI compatibility.
