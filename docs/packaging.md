Debian Packaging
================

Overview
--------
- We use `cargo-deb` to build a `.deb` for the CLI (`reasonable`).
- Metadata is defined in `cli/Cargo.toml` under `[package.metadata.deb]`.

Prerequisites
-------------
- Rust toolchain with Cargo
- cargo-deb: `cargo install cargo-deb`

Build
-----
- Makefile target: `make deb`
- Direct: `cargo deb -p reasonable-cli`

Outputs
-------
- Packages are written to `target/debian/`, e.g. `reasonable_0.3.0_amd64.deb`.

CI Artifacts (multi-arch)
-------------------------
- GitHub Actions builds `.deb` for common architectures using `cross`:
  - amd64 (`x86_64-unknown-linux-gnu`)
  - arm64 (`aarch64-unknown-linux-gnu`)
  - armhf (`armv7-unknown-linux-gnueabihf`)
  - i386 (`i686-unknown-linux-gnu`)
- See workflow: `.github/workflows/deb.yml`.
- Download artifacts from the corresponding workflow run under “Artifacts”.

Notes
-----
- The Debian package name is set to `reasonable` (so `apt` shows `reasonable`).
- Dependencies are auto-detected by `cargo-deb` (`depends = "$auto"`).
- Section is `utils`; adjust in `cli/Cargo.toml` if desired.

Install locally
---------------
- `sudo dpkg -i target/debian/reasonable_*_amd64.deb`
- If dependencies are missing: `sudo apt -f install`

Uninstall
---------
- `sudo apt remove reasonable`

Python bindings (.deb)
----------------------
We use `wheel2deb` to convert the Python wheel into a Debian package.

Prerequisites
-------------
- Python 3 + pip
- wheel2deb
- Debian build deps: apt-file, dpkg-dev, fakeroot, build-essential, devscripts, debhelper
- Run `apt-file update` once on the build host

Build (local)
-------------
1. Scripted: `./scripts/build_python_deb.sh`
2. Build the wheel: `cd python && maturin build --release --locked --out ../dist/wheel2deb`
3. Convert to .deb: `cd dist/wheel2deb && wheel2deb`
4. Packages are written to `dist/wheel2deb/output/`, e.g. `python3-reasonable_0.3.0_amd64.deb`

Docker build
------------
- `bash scripts/build_python_deb_docker.sh`
- Packages are written to `dist/deb-system/`

Install locally (Python)
------------------------
- `sudo apt-get install ./dist/output/*.deb`
