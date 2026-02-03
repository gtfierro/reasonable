#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
IMAGE="${IMAGE:-debian:bookworm}"
WORKDIR="/workspace"
OUTDIR="${WORKDIR}/dist/deb-system"

docker run --rm -t \
  -v "${ROOT}:${WORKDIR}" \
  -w "${WORKDIR}" \
  "${IMAGE}" \
  bash -lc '
    set -euo pipefail
    export DEBIAN_FRONTEND=noninteractive

    export HOME=/root
    export RUSTUP_HOME=/root/.rustup
    export CARGO_HOME=/root/.cargo

    apt-get update
    apt-get install -y --no-install-recommends \
      build-essential \
      ca-certificates \
      cargo \
      curl \
      debhelper \
      dh-python \
      dpkg-dev \
      patchelf \
      pkg-config \
      python3 \
      python3-all \
      python3-dev \
      python3-pip \
      python3-setuptools \
      python3-venv \
      python3-wheel \
      python3-rdflib \
      rustc

    RUSTUP_INIT_SKIP_PATH_CHECK=1 curl https://sh.rustup.rs -sSf | sh -s -- -y --profile minimal
    export PATH="/root/.cargo/bin:${PATH}"
    rustup default stable

    dpkg-buildpackage -uc -us -b

    mkdir -p "'"${OUTDIR}"'"
    for deb in /python3-reasonable_*_*.deb; do
      if [ -f "${deb}" ]; then
        cp -f "${deb}" "'"${OUTDIR}"'"/
      fi
    done

    dpkg -i "'"${OUTDIR}"'"/python3-reasonable_*_*.deb
    (cd /tmp && PYTHONPATH= python3 - <<'"'"'PY'"'"'
import reasonable

r = reasonable.PyReasoner()
out = r.reason()
print(f"reasonable {reasonable.__version__} ok, triples={len(out)}")
PY
    )
  '
