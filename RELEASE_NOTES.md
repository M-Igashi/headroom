## What's Changed

Updated mp3rgain CLI invocation to match v0.3.0 flag-based interface.

### Changes

- Updated mp3rgain command from subcommand style (`mp3rgain apply -g <steps> <file>`) to flag style (`mp3rgain -g <steps> <file>`)
- Requires mp3rgain v0.3.0 or later

### Compatibility

This release requires **mp3rgain v0.3.0** which uses mp3gain-compatible flags instead of subcommands.

---

## Installation

```bash
brew tap M-Igashi/tap
brew install headroom
```

ffmpeg and mp3rgain are installed automatically as dependencies.
