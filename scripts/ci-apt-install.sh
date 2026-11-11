#!/usr/bin/env bash
# Installs apt packages on a Linux CI runner: `scripts/ci-apt-install.sh <package>...`.
#
# A plain `apt-get install` fails the job when the runner cannot reach the
# package mirror for a moment. This goes through `ci-retry.sh`, which tries
# again and falls back to public name servers.
set -u

if [ "$#" -eq 0 ]; then
  echo "usage: $0 <package>..." >&2
  exit 2
fi

here=$(cd "$(dirname "$0")" && pwd)

# One command, so a retry repeats the update as well as the install.
exec bash "$here/ci-retry.sh" sudo sh -c \
  'apt-get update -qq -o Acquire::Retries=3 && apt-get install -y -qq -o Acquire::Retries=3 "$@"' \
  apt "$@"
