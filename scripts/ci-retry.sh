#!/usr/bin/env bash
# Runs a command that needs the network on a Linux CI runner, and tries again
# when it fails or hangs: `scripts/ci-retry.sh <command> [argument]...`.
#
# A runner sometimes cannot resolve a package mirror for a while ("Temporary
# failure resolving ..."), and a download can also hang without failing. This
# gives each attempt ten minutes and tries up to five times, waiting longer
# each time. When the runner's own name server keeps failing, it also asks
# public name servers (first through systemd-resolved, then directly), and
# prints what it sees so the log says why the command failed.
set -u

if [ "$#" -eq 0 ]; then
  echo "usage: $0 <command> [argument]..." >&2
  exit 2
fi

hosts=(azure.archive.ubuntu.com archive.ubuntu.com security.ubuntu.com)

for attempt in 1 2 3 4 5; do
  timeout --kill-after=30 600 "$@"
  status=$?
  if [ "$status" -eq 0 ]; then
    exit 0
  fi
  if [ "$status" -eq 124 ] || [ "$status" -eq 137 ]; then
    echo "::warning::'$1' did not finish in ten minutes (attempt $attempt of 5)"
  else
    echo "::warning::'$1' failed with status $status (attempt $attempt of 5)"
  fi
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

echo "::error::failed five times: $*"
exit 1
