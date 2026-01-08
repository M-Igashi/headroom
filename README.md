# headroom

Audio loudness analyzer and gain adjustment tool for mastering and DJ workflows.

## What is this?

**headroom** simulates the behavior of Rekordbox's Auto Gain feature, but with a key difference: it identifies files with available headroom (True Peak below -1 dBTP) and applies gain adjustment **without using a limiter**.

This tool is designed for DJs and producers who want to maximize loudness while preserving dynamics, ensuring tracks hit the -1 dBTP ceiling without clipping.

## Key Features

- **Non-destructive workflow**: Original files are backed up before processing
- **Metadata preservation**: Files are overwritten in place, so Rekordbox tags, cue points, and other metadata remain intact
- **No limiter**: Pure gain adjustment only — dynamics are preserved
- **Lossless formats**: Supports FLAC, AIFF, AIF, and WAV
- **Interactive CLI**: Guided step-by-step process with confirmation prompts

## How It Works

1. Scans the current directory for audio files
2. Measures LUFS (Integrated Loudness) and True Peak using ffmpeg
3. Calculates headroom: `-1 dBTP - Current True Peak`
4. Reports only files that can be boosted (positive headroom)
5. Optionally applies gain adjustment with backup

### Example

```
$ cd ~/Music/DJ-Tracks
$ headroom

╭─────────────────────────────────────╮
│          headroom v0.1.0            │
│   Audio Loudness Analyzer & Gain    │
╰─────────────────────────────────────╯

▸ Target directory: /Users/xxx/Music/DJ-Tracks
? Scan this directory? [Y/n]

✓ Found 24 audio files
✓ Analyzed 24 files

Filename               LUFS    True Peak     Headroom
────────────────────────────────────────────────────────
track01.aif           -13.3       -3.2 dBTP      +2.2 dB
track02.wav           -14.1       -2.5 dBTP      +1.5 dB
subfolder/track03.flac -12.0      -4.0 dBTP      +3.0 dB

✓ Report saved: ./headroom_report_20250109_123456.csv

ℹ 3 files can be boosted to -1 dBTP
? Apply gain adjustment to these files? [y/N] y
? Create backup before processing? [Y/n] y
✓ Backup directory: ./backup

✓ Done! 3 files processed.
```

## Installation

### Homebrew (macOS)

```bash
brew tap M-Igashi/tap
brew install headroom
```

### Requirements

- **ffmpeg** is required for audio analysis and processing

```bash
brew install ffmpeg
```

## Usage

```bash
# Navigate to your audio directory
cd ~/Music/DJ-Tracks

# Run headroom
headroom
```

The tool will guide you through:
1. Confirming the target directory
2. Scanning and analyzing files
3. Reviewing the report (only files with headroom > 0)
4. Optionally applying gain adjustments
5. Creating backups (recommended)

## Output

### CSV Report

A CSV report is automatically generated with the following columns:

| Filename | LUFS | True Peak (dBTP) | Headroom (dB) |
|----------|------|------------------|---------------|
| track01.aif | -13.3 | -3.2 | +2.2 |

### Backup Structure

Backups preserve the original directory structure:

```
./
├── track01.aif              ← Modified
├── subfolder/
│   └── track03.flac         ← Modified
└── backup/                  ← Created by headroom
    ├── track01.aif          ← Original
    └── subfolder/
        └── track03.flac     ← Original
```

## Important Notes

- **Files are overwritten in place** after backup — this ensures Rekordbox metadata (cue points, hot cues, beat grids, etc.) stored in the Rekordbox database remains linked to the same file path
- Only files with **positive headroom** are shown in reports and processed
- Files already at or above -1 dBTP are automatically skipped
- macOS resource fork files (`._*`) are automatically ignored

## Supported Formats

| Format | Extension | Encoding |
|--------|-----------|----------|
| FLAC | .flac | FLAC |
| AIFF | .aiff, .aif | PCM 24-bit |
| WAV | .wav | PCM 24-bit |

## License

MIT

## Author

Masanari Higashi
