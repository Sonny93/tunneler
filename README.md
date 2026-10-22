# Tunneler

SSH tunnel manager for Windows and Linux: a light daemon with an embedded SSH client and a tray icon, plus a window you open when you need it.

Status: early development.

## Install

Download from the [releases page](https://github.com/Sonny93/tunneler/releases). The binaries are not signed.

- Windows: installer (`-setup.exe`) or portable zip. [Details](https://sonny93.github.io/tunneler/guide/getting-started#windows)
- Debian and Ubuntu: `sudo apt install ./tunneler_<version>_amd64.deb`
- Other Linux: `tar.gz` archive or AppImage. [Details](https://sonny93.github.io/tunneler/guide/getting-started#linux)

## Documentation

<https://sonny93.github.io/tunneler/>

## Development

```sh
cargo test --locked          # Rust crates
cd app && pnpm check         # front end
cd docs && pnpm build        # documentation site
```

See [Development](https://sonny93.github.io/tunneler/contributing/development) for the rest.
