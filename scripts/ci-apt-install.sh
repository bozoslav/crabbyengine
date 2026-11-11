#!/usr/bin/env bash
# Installs apt packages on a Linux CI runner: `scripts/ci-apt-install.sh <package>...`.
#
# A plain `apt-get install` fails the job when the runner cannot reach the
# package mirror for a moment ("Temporary failure resolving ..."). This tries
# up to five times, waiting longer each time. When the runner's own name
# server keeps failing, it also asks public name servers (first through
# systemd-resolved, then directly), and prints what it sees so the log says
# why an install failed.
set -u

if [ "$#" -eq 0 ]; then
  echo "usage: $0 <package>..." >&2
  exit 2
fi

hosts=(azure.archive.ubuntu.com archive.ubuntu.com security.ubuntu.com)

install() {
  sudo apt-get update -qq -o Acquire::Retries=3
  sudo apt-get install -y -qq -o Acquire::Retries=3 "$@"
}

for attempt in 1 2 3 4 5; do
  if install "$@"; then
    exit 0
  fi
  echo "::warning::apt-get install failed (attempt $attempt of 5)"
  for host in "${hosts[@]}"; do
    if address=$(getent hosts "$host"); then
      echo "resolves: $address"
    else
      echo "does not resolve: $host"
    fi
  done
  case "$attempt" in
    2)
      echo "Adding public name servers to systemd-resolved"
      sudo mkdir -p /etc/systemd/resolved.conf.d
      printf '[Resolve]\nDNS=1.1.1.1 8.8.8.8\nFallbackDNS=1.0.0.1 8.8.4.4\n' |
        sudo tee /etc/systemd/resolved.conf.d/ci-public-dns.conf >/dev/null
      sudo systemctl restart systemd-resolved || true
      ;;
    3)
      echo "Pointing /etc/resolv.conf at public name servers"
      sudo rm -f /etc/resolv.conf
      printf 'nameserver 1.1.1.1\nnameserver 8.8.8.8\n' | sudo tee /etc/resolv.conf >/dev/null
      ;;
  esac
  if [ "$attempt" -lt 5 ]; then
    sleep $((attempt * 10))
  fi
done

echo "::error::apt-get install failed five times: $*"
exit 1
