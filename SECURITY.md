# Security policy

## Supported versions

Crabby Engine hasn't had a release yet. Security fixes go to `main`.

## Reporting a vulnerability

Please **don't** open a public issue. Report it privately through
[GitHub's private vulnerability reporting](https://github.com/bozoslav/ponko/security/advisories/new)
instead. Include what's affected, how to reproduce it, and what an attacker
could do with it.

We aim to reply within a week. Once a fix is ready, it's released, and
then the advisory is published, crediting you unless you'd rather not be
named.

## Areas worth knowing about

- **The control endpoint** (`CRABBY_CONTROL=1`) lets tools and AI agents
  read and change a running game. It's off by default, listens only on
  localhost, and needs a random token written to `~/.crabby/instances`.
  Never expose it to a network.
- **The C API** (`ponko_ffi`) trusts its caller like any C library:
  passing invalid pointers is undefined behavior, not a vulnerability.
- **Save files and data files** are parsed as JSON. Crashes or hangs on
  malformed input are bugs worth reporting.
