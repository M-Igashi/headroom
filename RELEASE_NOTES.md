## What's Changed

### Architecture Change: External mp3rgain CLI

v1.1.0 changes from using the mp3rgain Rust crate as a library dependency to calling the mp3rgain CLI tool externally.

### Highlights

- **mp3rgain as external dependency**: Now calls `mp3rgain` CLI instead of embedding as Rust crate
- **Homebrew dependency management**: `brew install headroom` automatically installs mp3rgain via tap
- **Cleaner architecture**: Separation of concerns between headroom (orchestration) and mp3rgain (MP3 manipulation)

### Breaking Changes

- Requires mp3rgain CLI to be installed (`brew install M-Igashi/tap/mp3rgain`)
- Non-Homebrew users need to install mp3rgain separately

### Dependencies

- ffmpeg (audio analysis & lossless format processing)
- mp3rgain (lossless MP3 gain adjustment)

---

## Installation

```bash
brew tap M-Igashi/tap
brew install headroom
```

Both ffmpeg and mp3rgain are installed automatically as dependencies.
