"""Sphinx configuration for the Reasonable project."""

from __future__ import annotations

from pathlib import Path

import better

try:  # Python 3.11+
    import tomllib
except ModuleNotFoundError:  # pragma: no cover
    import tomli as tomllib  # type: ignore


ROOT = Path(__file__).resolve().parent.parent


def _read_version() -> str:
    """Read the workspace version from Cargo.toml for a single source of truth."""
    cargo_toml = ROOT / "Cargo.toml"
    try:
        with cargo_toml.open("rb") as fh:
            cargo = tomllib.load(fh)
        return cargo.get("workspace", {}).get("package", {}).get("version", "0.0.0")
    except Exception:
        return "0.0.0"


project = "Reasonable"
author = "Gabe Fierro"
release = _read_version()
version = release

extensions = [
    "sphinx.ext.intersphinx",
]

templates_path = ["_templates"]
exclude_patterns: list[str] = [
    "_build",
    "_doctrees",
    "Thumbs.db",
    ".DS_Store",
    ".venv",
    ".venv/**",
    "*.md",
]

html_theme_path = [better.better_theme_path]
html_theme = "better"
html_static_path = ["_static"]
html_title = "Reasonable"
html_css_files = ["custom.css"]

html_theme_options = {
    "inlinecss": "",
    "cssfiles": [],
    "showheader": True,
    "showrelbartop": True,
    "showrelbarbottom": True,
    "linktotheme": True,
    "sidebarwidth": "230px",
    "body_max_width": "none",
}

intersphinx_mapping = {
    "python": ("https://docs.python.org/3", {}),
    "rdflib": ("https://rdflib.readthedocs.io/en/stable", {}),
}
