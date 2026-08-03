#!/bin/sh
set -eu

# Build the kloc container image. This is an expensive build: it compiles
# ~40 tree-sitter grammars plus the gigatoken tokenizer with fat LTO. A cold
# build takes ~15-20 min and ~8 GB of RAM (see "Building packages" in
# README.md). It is opt-in — not part of `cargo build` or `cargo test`.
#
# Usage: ./build-container.sh [--measure]
#   --measure   time the podman build and print the elapsed time and peak RSS.

measure=0
for arg in "$@"; do
    case "$arg" in
        --measure) measure=1 ;;
        -h|--help)
            echo "Usage: $0 [--measure]"; exit 0 ;;
        *)
            echo "Unknown argument: $arg" >&2
            echo "Usage: $0 [--measure]" >&2
            exit 2 ;;
    esac
done

version=$(grep '^version =' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/')

echo "==> Building kloc container (version $version) ..."
echo "    Estimated runtime: ~15-20 min on a cold build cache (a few minutes warm)."
if [ "$measure" -eq 1 ]; then
    /usr/bin/time -f "    build took %e s (peak RSS %M KB)" \
        podman build -f Containerfile -t "kloc:$version" .
else
    podman build -f Containerfile -t "kloc:$version" .
fi

echo "==> Tagging kloc:$version as kloc:latest ..."
podman tag "kloc:$version" kloc:latest

echo "==> Done."
podman images kloc
