#!/bin/sh
# Checks install.ps1 in a PowerShell container, without a network and without Windows.
#
# The repository is mounted read-only; cases.ps1 loads the installer's functions, replaces the
# ones that only work on Windows (the registry, the settings broadcast, the console check) and
# runs every case against stand-ins for cargo, rustup, rustup-init, rustc and vswhere in
# temporary folders. It also runs the real script under `irm | iex`, the scriptblock form and
# as a file to check how it starts and ends.
#
# Usage: tests/install-ps1/run.sh [image]   (default mcr.microsoft.com/powershell:7.5-ubuntu-24.04)
# Set ANALYZER=<folder holding the PSScriptAnalyzer module> to lint as well. The checks run without
# a network, so the module is saved into that folder first, in a container of its own:
#
#   mkdir -p /tmp/psanalyzer
#   podman run --rm -v /tmp/psanalyzer:/out:Z mcr.microsoft.com/powershell:7.5-ubuntu-24.04 \
#     pwsh -NoProfile -Command 'Save-Module PSScriptAnalyzer -Path /out -Force'
#   ANALYZER=/tmp/psanalyzer tests/install-ps1/run.sh
#
# Nothing is installed on the machine: the module stays in that folder and the container is thrown
# away. Without ANALYZER the cases still run and only the lint is skipped, which they say.
set -eu

here=$(cd "$(dirname "$0")" && pwd)
repo=$(cd "$here/../.." && pwd)
image=${1:-mcr.microsoft.com/powershell:7.5-ubuntu-24.04}

command -v podman >/dev/null 2>&1 || { echo "podman is needed to run these checks" >&2; exit 1; }

set -- --rm --network none -v "$repo:/src:ro,Z"
if [ -n "${ANALYZER:-}" ]; then
    set -- "$@" -v "$ANALYZER:/analyzer:ro,Z" -e PSModulePath=/analyzer
fi
podman run "$@" "$image" pwsh -NoProfile -NonInteractive -File /src/tests/install-ps1/cases.ps1
