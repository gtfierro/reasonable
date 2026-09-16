Debian Packaging
================

Overview
--------
- We use `cargo-deb` to build a `.deb` for the CLI (`reasonable`).
- Metadata is defined in `cli/Cargo.toml` under `[package.metadata.deb]`.
- The Python bindings (`python3-reasonable`) can be built either with
  `cargo-deb` (metadata in `python/Cargo.toml`) or with `wheel2deb`; both are
  described below.

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

Python bindings (.deb) with cargo-deb
-------------------------------------
`cargo deb -p pyreasonable` builds `python3-reasonable`, which installs the
extension module as an importable Python package:

```
/usr/lib/python3/dist-packages/reasonable/__init__.py
/usr/lib/python3/dist-packages/reasonable/reasonable.abi3.so
```

Details worth knowing (all configured in `python/Cargo.toml` under
`[package.metadata.deb]`):

- The package is named `python3-reasonable`, per Debian convention, even though
  the crate is `pyreasonable`.
- The build enables the crate's `abi3` feature (`pyo3/abi3-py39`), so a single
  binary works with any `python3` the distribution ships. This is what makes it
  safe to install into the unversioned `dist-packages` directory.
- The default `cargo-deb` behaviour for a `cdylib` crate would install
  `/usr/lib/libreasonable.so`, which Python cannot import; the explicit `assets`
  list replaces it with the package layout shown above.
- `Depends` includes `python3 (>= 3.9~)` and `python3-rdflib (>= 6.1.1)` in
  addition to the auto-detected shared library dependencies.

Build (local)
-------------
- Makefile target: `make deb-python-cargo`
- Scripted: `./scripts/build_python_deb_cargo.sh`
- Direct: `cargo deb -p pyreasonable`
- Output: `target/debian/python3-reasonable_<version>_<arch>.deb`

Install and verify
------------------
```bash
sudo apt-get install ./target/debian/python3-reasonable_*.deb
# run from any directory other than a reasonable source checkout, otherwise the
# repo's ./reasonable/ directory shadows the installed package
cd /tmp && python3 -c "import reasonable; print(reasonable.__version__)"
```

Cross-building
--------------
The bindings cross-compile without a target Python interpreter because of the
stable ABI build, so the same two-step flow used for the CLI works:

```bash
cross build -p pyreasonable --release --features abi3 --target aarch64-unknown-linux-gnu
cargo deb -p pyreasonable --no-build --target aarch64-unknown-linux-gnu
```

Note that `--features abi3` has to be passed explicitly when building outside of
`cargo-deb`; the feature list in `[package.metadata.deb]` only applies to builds
that `cargo-deb` runs itself.

Python bindings (.deb) with wheel2deb
-------------------------------------
Alternatively, `wheel2deb` converts the Python wheel into a Debian package.

Because `wheel2deb` resolves Python requirements against distro package metadata,
runtime dependency lower bounds in `python/pyproject.toml` must stay compatible
with the oldest supported Debian/Ubuntu `python3-*` package versions. In
particular, `rdflib` is pinned to `>=6.1.1` so the conversion works on current
Ubuntu runners where `python3-rdflib` is 6.1.1.

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

Install locally (Python)
------------------------
- `sudo apt-get install ./dist/wheel2deb/output/*.deb`
