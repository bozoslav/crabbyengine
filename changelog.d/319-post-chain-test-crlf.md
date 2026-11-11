### Fixed
- The post chain's source-order test reads its file with `\n` line ends,
  so it passes on a Windows checkout with `\r\n`.
