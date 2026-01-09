# headroom

Audio loudness analyzer and gain adjustment tool for mastering and DJ workflows.

## What is this?

**headroom** simulates the behavior of Rekordbox's Auto Gain feature, but with a key difference: it identifies files with available headroom (True Peak below the target ceiling) and applies gain adjustment **without using a limiter**.

This tool is designed for DJs and producers who want to maximize loudness while preserving dynamics, ensuring tracks hit the optimal True Peak ceiling without clipping.

## Key Features

- **Smart True Peak ceiling**: Uses -0.5 dBTP for lossless/high-bitrate files, -1.0 dBTP for low-bitrate (see below)
- **Non-destructive workflow**: Original files are backed up before processing
- **Metadata preservation**: Files are overwritten in place, so Rekordbox tags, cue points, and other metadata remain intact
- **No limiter**: Pure gain adjustment only — dynamics are preserved
- **Lossless formats**: Supports FLAC, AIFF, AIF, and WAV
- **Interactive CLI**: Guided step-by-step process with confirmation prompts

## True Peak Ceiling Strategy

Based on [AES TD1008](https://www.aes.org/technical/documentDownloads.cfm?docID=731) recommendations, headroom uses different True Peak ceilings depending on the audio format:

| Format | Ceiling | Rationale |
|--------|---------|-----------|
| FLAC, AIFF, WAV | **-0.5 dBTP** | Lossless files will be distributed via high-bitrate streaming (Spotify Premium 320kbps, Apple Music 256kbps AAC) |
| ≥256 kbps lossy | **-0.5 dBTP** | High-bitrate codecs have minimal overshoot |
| <256 kbps lossy | **-1.0 dBTP** | Lower bitrates cause more codec overshoot |

From AES TD1008:
> "High rate (e.g., 256 kbps) coders may work satisfactorily with as little as −0.5 dB TP for the limiting threshold. However, lower bit rate coders tend to overshoot peaks even more, so the limiting threshold may need to be reduced below −1.0 dB TP."

This allows you to extract **+0.5 dB more headroom** from your lossless masters compared to the conservative -1.0 dBTP approach.

## How It Works

1. Scans the current directory for audio files
2. Measures LUFS (Integrated Loudness) and True Peak using ffmpeg
3. Determines the appropriate ceiling based on format/bitrate
4. Calculates headroom: `Target Ceiling - Current True Peak`
5. Reports only files that can be boosted (positive headroom)
6. Optionally applies gain adjustment with backup

### Example

```
$ cd ~/Music/DJ-Tracks
$ headroom

╭─────────────────────────────────────╮
│          headroom v0.2.0            │
│   Audio Loudness Analyzer & Gain    │
╰─────────────────────────────────────╯

▸ Target directory: /Users/xxx/Music/DJ-Tracks
? Scan this directory? [Y/n]

✓ Found 24 audio files
✓ Analyzed 24 files

Filename                 LUFS    True Peak    Target     Headroom
────────────────────────────────────────────────────────────────────
track01.aif             -13.3     -3.2 dBTP  -0.5 dBTP      +2.7 dB
track02.wav             -14.1     -2.5 dBTP  -0.5 dBTP      +2.0 dB
subfolder/track03.flac  -12.0     -4.0 dBTP  -0.5 dBTP      +3.5 dB

✓ Report saved: ./headroom_report_20250109_123456.csv

ℹ 3 files can be boosted
  • Target ceiling: -0.5 dBTP (lossless/high-bitrate)
? Apply gain adjustment to these files? [y/N] y
? Create backup before processing? [Y/n] y
✓ Backup directory: ./backup

✓ Done! 3 files processed.
```

## Installation

### macOS (Homebrew)

```bash
brew tap M-Igashi/tap
brew install headroom
```

This automatically installs ffmpeg as a dependency.

### Windows

#### Prerequisites

1. **Install ffmpeg** (required for audio analysis)

   Using winget:
   ```powershell
   winget install ffmpeg
   ```

   Or using Chocolatey:
   ```powershell
   choco install ffmpeg
   ```

   Or download manually from [ffmpeg.org](https://ffmpeg.org/download.html) and add to PATH.

2. **Install Rust** (required to build from source)

   Download and run the installer from [rustup.rs](https://rustup.rs/)

#### Build from Source

```powershell
# Clone the repository
git clone https://github.com/M-Igashi/headroom.git
cd headroom

# Build release binary
cargo build --release

# The binary will be at target\release\headroom.exe
```

#### Add to PATH (optional)

To run `headroom` from any directory, add it to your PATH:

```powershell
# Copy to a directory in your PATH, or add the build directory to PATH
copy target\release\headroom.exe C:\Users\YourName\bin\
```

Or add `C:\path\to\headroom\target\release` to your system PATH environment variable.

### Linux

#### Prerequisites

```bash
# Ubuntu/Debian
sudo apt install ffmpeg

# Fedora
sudo dnf install ffmpeg

# Arch
sudo pacman -S ffmpeg
```

#### Build from Source

```bash
# Install Rust if not already installed
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and build
git clone https://github.com/M-Igashi/headroom.git
cd headroom
cargo build --release

# Install to local bin
cp target/release/headroom ~/.local/bin/
```

## Usage

```bash
# Navigate to your audio directory
cd ~/Music/DJ-Tracks

# Run headroom
headroom
```

On Windows:
```powershell
cd C:\Users\YourName\Music\DJ-Tracks
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

| Filename | LUFS | True Peak (dBTP) | Target (dBTP) | Headroom (dB) |
|----------|------|------------------|---------------|---------------|
| track01.aif | -13.3 | -3.2 | -0.5 | +2.7 |

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
- Files already at or above the target ceiling are automatically skipped
- macOS resource fork files (`._*`) are automatically ignored
- The target ceiling (-0.5 or -1.0 dBTP) is determined per-file based on format/bitrate

## Why -0.5 dBTP for Lossless?

The traditional -1.0 dBTP recommendation accounts for:
1. **True Peak meter error** (~0.6 dB with 4x oversampling)
2. **Codec overshoot** during lossy encoding

However, for lossless files destined for high-bitrate streaming:
- Spotify Premium uses 320 kbps OGG Vorbis
- Apple Music uses 256 kbps AAC
- Both have minimal codec overshoot at these bitrates

AES TD1008 confirms that 256+ kbps codecs work "satisfactorily with as little as −0.5 dB TP". This gives you an extra +0.5 dB of loudness without sacrificing safety.

## Supported Formats

| Format | Extension | Encoding | Default Ceiling |
|--------|-----------|----------|-----------------|
| FLAC | .flac | FLAC | -0.5 dBTP |
| AIFF | .aiff, .aif | PCM 24-bit | -0.5 dBTP |
| WAV | .wav | PCM 24-bit | -0.5 dBTP |

## Changelog

### v0.2.0
- **Smart True Peak ceiling**: Uses -0.5 dBTP for lossless/high-bitrate files based on AES TD1008
- Added Target column to report table
- Improved summary messages showing applied ceiling

### v0.1.0
- Initial release
- Fixed -1.0 dBTP ceiling

## License

MIT
