#!/bin/sh
set -eu

version=$(grep '^version =' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/')

echo "==> Building kloc container (version $version) ..."
podman build -f Containerfile -t "kloc:$version" .

echo "==> Tagging kloc:$version as kloc:latest ..."
podman tag "kloc:$version" kloc:latest

echo "==> Done."
podman images kloc
