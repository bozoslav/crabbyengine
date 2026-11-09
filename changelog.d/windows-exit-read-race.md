### Fixed

- Windows: the last lines a game printed right before `std::process::exit` could still be lost while the control endpoint captured output, when the copy thread was killed just as its read returned. The read's byte count is now recorded by the kernel in memory the exit hook can see.
