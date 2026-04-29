## Highlights

- **Update notification on startup.** headroom now checks GitHub Releases once per 24 hours and prints the appropriate upgrade command (Homebrew / winget / cargo) when a newer version is available. Disable with `--no-update-check` or `HEADROOM_NO_UPDATE_CHECK=1`.
- **Reliable AAC/M4A gain via mp3rgain v2.3.0.** Picks up [mp3rgain#120](https://github.com/M-Igashi/mp3rgain/pull/120) which fixes a short-window spectral parser bug that previously caused some processed `.m4a` files to fail in ffmpeg with `invalid band type` / `Number of bands exceeds limit`. Also benefits from [mp3rgain#121](https://github.com/M-Igashi/mp3rgain/pull/121) — AAC analysis is now 4-6× faster.

## Other Changes

- deps: bump rayon and the rust-minor-patch group (#33)
