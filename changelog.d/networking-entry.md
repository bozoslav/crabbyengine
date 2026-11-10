### Added
- `ctx.net` is now a `Networking` service with one field per transport:
  `ctx.net.lockstep` (the `Net` session), `ctx.net.messages`
  (`Option<NetMessages>`), `ctx.net.snapshots` (`NetSnapshots`: one
  `SnapshotStream<Pose>` per entity id with its duplicate filter) and
  `ctx.net.interest` (`Option<Interest>`). It dereferences to the lockstep
  session, so `ctx.net.set_input(..)` and the other `Net` calls compile
  unchanged.
- `Game::messages(endpoint)` next to `Game::lockstep(net)`. The engine polls
  `ctx.net.messages` once per frame before any system (once per tick in
  headless runs, also while paused or while lockstep waits), so systems
  only read events and send; `ctx.net.message_error()` reports a failed
  poll.
- The `ponko::net` module holds everything networking, with a guide to
  which transport fits which game. The crate root still exports the same
  names.

### Changed
- `Ctx::net` has the type `Networking` instead of `Net`. Code that assigns
  or replaces the whole field (`ctx.net = Net::host(..)`) now assigns
  `ctx.net.lockstep`, or uses `Networking::from(net)`.
- `examples/net_messages.rs` is two games that talk through
  `ctx.net.messages`; neither polls the endpoint.

### Fixed
- A lockstep host relaying large inputs from several players could send
  packets far over the 1400-byte limit (up to about 8 KB with eight players
  and 1 KB inputs). Inputs that don't fit one packet now go out in several,
  in the same flush. The packet format is unchanged.
- The lockstep UDP transport no longer loops on a socket that returns only
  errors; after 64 in a row it reports no packet and the silence timeout
  ends the session.
- Joining a lockstep session reports "nobody answered" on the wait that
  reached the timeout instead of one pass later.
