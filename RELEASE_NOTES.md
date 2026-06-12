# headroom 2.1.0 — AI-assisted hardening & performance

A reliability and performance release. The codebase was systematically audited with **Claude (Fable 5)**, which uncovered several latent bugs — including one that could silently modify your backups — all fixed here, alongside a round of performance work. No new flags, no breaking changes: existing commands behave the same, just safer and faster.

---

## Latent bugs found & fixed (Claude Fable 5 audit)

- **Backups are no longer re-processed** (#45). The default backup directory `<target>/backup` lives inside the scan root, so a second run would pick the backup copies up and gain-adjust them too — defeating the purpose of a backup. Backup directories created by headroom now contain a `.headroom-backup` marker file and are skipped during recursive scans. Explicitly pointing headroom at a backup directory still works. *Note: backups created by older versions are unmarked; re-running with `--backup` marks the default location going forward.*
- **Silent audio no longer saturates the gain math.** ffmpeg's loudnorm reports `-inf` for silent files, which previously overflowed into `i32::MAX` gain steps. Non-finite measurements are now rejected with a clear per-file error.
- **Backup path edge case.** With mixed-root inputs, `strip_prefix` could yield an absolute or empty relative path, making the backup copy target the source file itself. Now guarded with a bare-filename fallback.
- **Report column alignment.** Columns are padded before ANSI styles are applied, fixing misaligned analysis tables.

## Performance

- **Parallel gain processing.** File processing now runs in parallel via rayon (analysis already did), cutting wall-clock time on multi-file batches.
- **One less process per lossy file** (#47). The bitrate is parsed from the loudnorm run's existing ffmpeg output instead of spawning a separate ffprobe for every MP3/AAC file; ffprobe remains as a fallback.
- **Startup is never blocked by the update check** (#46). The version check runs on a background thread with a 3-second timeout, and the upgrade hint now prints *after* the run — where you'll actually see it.

## Other changes

- Refactor: per-format gain wrappers merged into a single `LossyFormat` enum
- Docs: rbsort sort-comparison write-up (`docs/rbsort-sort-comparison.md`), README updates
- Dependencies: mp3rgain 2.7.0 → 2.8.0, chrono 0.4.45, serde_json bump
- CI: actions/checkout pinned to v6.0.3

## Migration

**None.** Fully backward-compatible with v2.0.x.

---
