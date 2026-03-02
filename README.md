# headroom

Audio loudness analyzer and gain adjustment tool for mastering and DJ workflows.

## What is this?

**headroom** simulates the behavior of Rekordbox's Auto Gain feature, but with a key difference: it identifies files with available headroom (True Peak below the target ceiling) and applies gain adjustment **without using a limiter**.

This tool is designed for DJs and producers who want to maximize loudness while preserving dynamics, ensuring tracks hit the optimal True Peak ceiling without clipping.

## Key Features

- **Single binary**: mp3rgain is built-in as a library — only ffmpeg required as external dependency
- **Smart True Peak ceiling**: Based on AES TD1008, uses -0.5 dBTP for high-quality files, -1.0 dBTP for low-bitrate
- **Multiple processing methods**: ffmpeg for lossless formats, built-in mp3rgain for lossless MP3/AAC gain, ffmpeg re-encode for precise gain
- **Non-destructive workflow**: Original files are backed up before processing
- **Metadata preservation**: Files are overwritten in place, so Rekordbox tags, cue points, and other metadata remain intact
- **No limiter**: Pure gain adjustment only — dynamics are preserved
- **Interactive CLI**: Guided step-by-step process with two-stage confirmation

## Processing Methods

headroom selects the optimal method for each file based on format and headroom:

| Format | Method | Precision | Quality Loss |
|--------|--------|-----------|--------------|
| FLAC, AIFF, WAV | ffmpeg | Arbitrary | None |
| MP3, AAC/M4A | mp3rgain (built-in) | 1.5dB steps | **None** (global_gain modification) |
| MP3, AAC/M4A | ffmpeg re-encode | Arbitrary | Inaudible at ≥256kbps |

### Three-Tier Approach for Lossy Formats (MP3/AAC)

Each MP3 and AAC/M4A file is categorized into one of three tiers:

1. **Native Lossless** — ≥1.5 dB headroom to bitrate-aware ceiling
   - Truly lossless global_gain header modification in 1.5dB steps
   - Uses built-in [mp3rgain](https://github.com/M-Igashi/mp3rgain) library
   - Applied automatically (no user confirmation needed)

2. **Re-encode** — headroom exists but <1.5 dB to ceiling
   - Uses ffmpeg for arbitrary precision gain
   - MP3: `libmp3lame` with `-q:a 0` / AAC: `libfdk_aac` (falls back to built-in `aac`)
   - Preserves original bitrate; requires explicit user confirmation

3. **Skip** — no headroom available

## True Peak Ceiling

Based on [AES TD1008](https://www.aes.org/technical/documentDownloads.cfm?docID=731) recommendations. The ceiling depends on bitrate, not format:

| Bitrate | Ceiling | Native lossless requires |
|---------|---------|------------------------|
| Lossless (FLAC, AIFF, WAV) | **-0.5 dBTP** | — |
| Lossy ≥256kbps | **-0.5 dBTP** | TP ≤ -2.0 dBTP |
| Lossy <256kbps | **-1.0 dBTP** | TP ≤ -2.5 dBTP |

## How It Works

1. Scans the current directory for audio files (FLAC, AIFF, WAV, MP3, AAC/M4A)
2. Measures LUFS (Integrated Loudness) and True Peak using ffmpeg
3. Categorizes files by processing method:
   - **Green**: Lossless files (ffmpeg)
   - **Yellow**: MP3/AAC files with enough headroom for native lossless gain
   - **Magenta**: MP3/AAC files requiring re-encode
4. Displays categorized report
5. Two-stage confirmation:
   - First: "Apply lossless gain adjustment?" (lossless + native MP3/AAC)
   - Second: "Also process files with re-encoding?" (MP3/AAC requiring re-encode)
6. Creates backups and processes files

### Example

```
$ cd ~/Music/DJ-Tracks
$ headroom

╭─────────────────────────────────────╮
│          headroom v1.7.1            │
│   Audio Loudness Analyzer & Gain    │
╰─────────────────────────────────────╯

▸ Target directory: /Users/xxx/Music/DJ-Tracks

✓ Found 28 audio files
✓ Analyzed 28 files

● 3 lossless files (ffmpeg, precise gain)
  Filename        LUFS    True Peak    Target        Gain
  track01.flac   -13.3    -3.2 dBTP   -0.5 dBTP   +2.7 dB
  track02.aif    -14.1    -4.5 dBTP   -0.5 dBTP   +4.0 dB
  track03.wav    -12.5    -2.8 dBTP   -0.5 dBTP   +2.3 dB

● 2 MP3 files (native lossless, 1.5dB steps, target: -2.0 dBTP)
  Filename        LUFS    True Peak    Target        Gain
  track04.mp3    -14.0    -5.5 dBTP   -2.0 dBTP   +3.0 dB
  track05.mp3    -13.5    -6.0 dBTP   -2.0 dBTP   +3.0 dB

● 2 AAC/M4A files (native lossless, 1.5dB steps)
  Filename        LUFS    True Peak    Target        Gain
  track08.m4a    -13.0    -4.0 dBTP   -1.0 dBTP   +3.0 dB
  track09.m4a    -12.5    -4.5 dBTP   -1.0 dBTP   +3.0 dB

● 2 MP3 files (re-encode required for precise gain)
  Filename        LUFS    True Peak    Target        Gain
  track06.mp3    -12.0    -1.5 dBTP   -0.5 dBTP   +1.0 dB
  track07.mp3    -11.5    -1.2 dBTP   -0.5 dBTP   +0.7 dB

● 1 AAC/M4A files (re-encode required)
  Filename        LUFS    True Peak    Target        Gain
  track10.m4a    -12.5    -1.8 dBTP   -0.5 dBTP   +1.3 dB

✓ Report saved: ./headroom_report_20250109_123456.csv

? Apply lossless gain adjustment to 3 lossless + 2 MP3 (lossless gain) + 2 AAC/M4A (lossless gain) files? [y/N] y

ℹ 2 MP3 + 1 AAC/M4A files have headroom but require re-encoding for precise gain.
  • Re-encoding causes minor quality loss (inaudible at 256kbps+)
  • Original bitrate will be preserved
? Also process these files with re-encoding? [y/N] y

? Create backup before processing? [Y/n] y
✓ Backup directory: ./backup

✓ Done! 10 files processed.
  • 3 lossless files (ffmpeg)
  • 2 MP3 files (native, lossless)
  • 2 AAC/M4A files (native, lossless)
  • 2 MP3 files (re-encoded)
  • 1 AAC/M4A files (re-encoded)
```

## Installation

headroom requires ffmpeg. Package managers install it automatically.

| Platform | Command |
|----------|---------|
| **macOS (Homebrew)** | `brew install M-Igashi/tap/headroom` |
| **Windows (winget)** | `winget install M-Igashi.headroom` |
| **Windows (Scoop)** | `scoop bucket add headroom https://github.com/M-Igashi/scoop-bucket && scoop install headroom` |
| **Cargo** | `cargo install headroom` (ffmpeg must be installed separately) |

Pre-built binaries are available on the [Releases](https://github.com/M-Igashi/headroom/releases) page (ffmpeg must be installed separately).

### Build from Source

```bash
git clone https://github.com/M-Igashi/headroom.git
cd headroom
cargo build --release
```

## Usage

```bash
cd ~/Music/DJ-Tracks
headroom
```

The tool will guide you through:
1. Scanning and analyzing all audio files
2. Reviewing the categorized report
3. Confirming lossless processing
4. Optionally enabling MP3/AAC re-encoding
5. Creating backups (recommended)

## Output

### CSV Report

| Filename | Format | Bitrate (kbps) | LUFS | True Peak (dBTP) | Target (dBTP) | Headroom (dB) | Method | Effective Gain (dB) |
|----------|--------|----------------|------|------------------|---------------|---------------|--------|---------------------|
| track01.flac | Lossless | - | -13.3 | -3.2 | -0.5 | +2.7 | ffmpeg | +2.7 |
| track04.mp3 | MP3 | 320 | -14.0 | -5.5 | -2.0 | +3.5 | mp3rgain | +3.0 |
| track06.mp3 | MP3 | 320 | -12.0 | -1.5 | -0.5 | +1.0 | re-encode | +1.0 |
| track08.m4a | AAC | 256 | -13.0 | -4.0 | -1.0 | +3.0 | native | +3.0 |
| track10.m4a | AAC | 256 | -12.5 | -1.8 | -0.5 | +1.3 | re-encode | +1.3 |

### Backup Structure

```
./
├── track01.flac             ← Modified
├── track04.mp3              ← Modified
├── track08.m4a              ← Modified
├── subfolder/
│   └── track06.mp3          ← Modified
└── backup/                  ← Created by headroom
    ├── track01.flac         ← Original
    ├── track04.mp3          ← Original
    ├── track08.m4a          ← Original
    └── subfolder/
        └── track06.mp3      ← Original
```

## Important Notes

- **Files are overwritten in place** after backup — Rekordbox metadata remains linked
- Only files with **positive effective gain** are shown and processed
- MP3/AAC native lossless requires at least **1.5dB headroom** to be processed
- MP3/AAC re-encoding is **opt-in** and requires explicit confirmation
- macOS resource fork files (`._*`) are automatically ignored

## Technical Details

### Why 1.5dB Steps?

Both MP3 and AAC store a "global_gain" value as an integer. Each ±1 increment changes the gain by `2^(1/4)` = **±1.5 dB**. This is a format-level constraint, not a tool limitation.

headroom uses the built-in [mp3rgain](https://github.com/M-Igashi/mp3rgain) library to directly modify this field — no decoding or re-encoding involved.

### Bitrate-Aware Ceiling for Native Lossless

Since native lossless gain only works in 1.5dB steps, at least 1.5dB of headroom to the target ceiling is required:
- **≥256kbps**: Target -0.5 dBTP → requires TP ≤ -2.0 dBTP
- **<256kbps**: Target -1.0 dBTP → requires TP ≤ -2.5 dBTP

Example: 320kbps file at -3.5 dBTP gets 2 steps (+3.0dB) → -0.5 dBTP (optimal)

### Re-encode Quality

At ≥256kbps, re-encoding introduces quantization noise below -90dB — far below audible threshold. Only gain is applied (no EQ, compression, or dynamics processing), and original bitrate is preserved.

## License

MIT
