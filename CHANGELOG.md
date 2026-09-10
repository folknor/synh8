# Changelog

## Unreleased

- Bump rust-apt 0.10 -> 0.11.2: `commit()` now releases the APT lock on failure and
  returns an error instead of panicking when an install fails mid-transaction
- Better resolver error messages: errors are now separated from warnings (a stray APT
  warning can no longer displace the actual error as the headline), and empty
  messages are filtered correctly
- Rename debug binary `debug_cli` -> `debug-cli`

## 0.1.1

- Revert termion backend switch; restore crossterm (termion had rendering glitches on alt-tab)

## 0.1.0

- Initial release
