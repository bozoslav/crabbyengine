### Fixed
- The web smoke runner waits a moment before it calls a failure a plain
  page error, so a missing asset or a panic is named as such even when
  the page error arrives first. The missing-asset self-test failed now
  and then on that race.
