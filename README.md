# Fusemovie

Fusemovie is a desktop video editor written in Rust, with the Studio look of the Fuse suite
(Home, new-project screen, top bar, timeline, AI panels).

It is a fork of [storytold/filmcraft](https://github.com/storytold/filmcraft), licensed
**MIT OR Apache-2.0** like upstream. The original README is kept in
[docs/upstream-README.md](docs/upstream-README.md).

## Download

Windows installer (MSI and zip): the pre-release
[`latest-studio-redesign`](https://github.com/fernandinhomartins40/fusemovie/releases/tag/latest-studio-redesign)
on this repository's Releases page.

## Build

```sh
cargo run --release -p filmcraft                  # the desktop app
cargo xtask ci                                    # fmt, clippy -D warnings, tests, layers, assets
```

## More

Plan, rules and status of the fork: [docs/fusemovie/](docs/fusemovie/). Contributor rules: [AGENTS.md](AGENTS.md).
