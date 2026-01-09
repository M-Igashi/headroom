# headroom

Audio loudness analyzer and gain adjustment tool for mastering and DJ workflows.

## What is this?

**headroom** simulates the behavior of Rekordbox's Auto Gain feature, but with a key difference: it identifies files with available headroom (True Peak below the target ceiling) and applies gain adjustment **without using a limiter**.

This tool is designed for DJs and producers who want to maximize loudness while preserving dynamics, ensuring tracks hit the optimal True Peak ceiling without clipping.

## Key Features

- **Smart True Peak ceiling**: Uses -0.5 dBTP for lossless files, -1.0 dBTP for MP3 (see below)
- **MP3 support**: Uses mp3gain for truly lossless, reversible gain adjustment (1.5dB steps)
- **Non-destructive workflow**: Original files are backed up before processing
- **Metadata preservation**: Files are overwritten in place, so Rekordbox tags, cue points, and other metadata remain intact
- **No limiter**: Pure gain adjustment only — dynamics are preserved
- **Interactive CLI**: Guided step-by-step process with confirmation prompts

## Supported Formats

| Format | Extension | Method | Ceiling | Notes |
|--------|-----------|--------|---------|-------|
| FLAC | .flac | ffmpeg | -0.5 dBTP | Arbitrary precision |
| AIFF | .aiff, .aif | ffmpeg | -0.5 dBTP | Arbitrary precision |
| WAV | .wav | ffmpeg | -0.5 dBTP | Arbitrary precision |
| MP3 | .mp3 | mp3gain | -1.0 dBTP | 1.5dB steps, truly lossless |

## True Peak Ceiling Strategy

Based on [AES TD1008](https://www.aes.org/technical/documentDownloads.cfm?docID=731) recommendations:

| Format | Ceiling | Rationale |
|--------|---------|-----------|
| Lossless (FLAC, AIFF, WAV) | **-0.5 dBTP** | Will be distributed via high-bitrate streaming (Spotify 320kbps, Apple Music 256kbps) |
| MP3 | **-1.0 dBTP** | Conservative ceiling for lossy format |

From AES TD1008:
> "High rate (e.g., 256 kbps) coders may work satisfactorily with as little as −0.5 dB TP for the limiting threshold. However, lower bit rate coders tend to overshoot peaks even more, so the limiting threshold may need to be reduced below −1.0 dB TP."

## MP3 Gain Adjustment

MP3 files are processed using **mp3gain**, which modifies the "global gain" field in MP3 frames:

- **Truly lossless**: No re-encoding, no quality loss
- **Reversible**: Undo information is saved in APEv2 tags
- **1.5dB steps**: Due to MP3 format specification (gain = 2^(n/4))

The 1.5dB step limitation means:
- Headroom of 2.0dB → 1.5dB applied (1 step)
- Headroom of 3.5dB → 3.0dB applied (2 steps)
- Files with <1.5dB headroom are skipped

## How It Works

1. Scans the current directory for audio files
2. Measures LUFS (Integrated Loudness) and True Peak using ffmpeg
3. Determines the appropriate ceiling based on format
4. Calculates headroom: `Target Ceiling - Current True Peak`
5. Reports only files that can be boosted
6. Optionally applies gain adjustment with backup

### Example

```
$ cd ~/Music/DJ-Tracks
$ headroom

╭─────────────────────────────────────╮
│          headroom v0.3.0            │
│   Audio Loudness Analyzer & Gain    │
╰─────────────────────────────────────╯

▸ Target directory: /Users/xxx/Music/DJ-Tracks
? Include MP3 files? (uses mp3gain, 1.5dB steps) [y/N] y

✓ Found 24 audio files
✓ Analyzed 24 files

Filename                 LUFS    True Peak    Target     Headroom    Effective
────────────────────────────────────────────────────────────────────────────────
track01.aif             -13.3     -3.2 dBTP  -0.5 dBTP      +2.7 dB      +2.7 dB
track02.mp3             -14.1     -4.5 dBTP  -1.0 dBTP      +3.5 dB      +3.0 dB
subfolder/track03.flac  -12.0     -4.0 dBTP  -0.5 dBTP      +3.5 dB      +3.5 dB

✓ Report saved: ./headroom_report_20250109_123456.csv

ℹ 3 files can be boosted
  • 2 lossless files → -0.5 dBTP ceiling (ffmpeg)
  • 1 MP3 files → -1.0 dBTP ceiling (mp3gain, 1.5dB steps)
? Apply gain adjustment to these files? [y/N] y
? Create backup before processing? [Y/n] y
✓ Backup directory: ./backup

✓ Done! 3 files processed.
  ℹ MP3 gain is lossless and reversible (undo info saved in tags)
```

## Installation

### macOS (Homebrew)

```bash
brew tap M-Igashi/tap
brew install headroom
```

For MP3 support:
```bash
brew install mp3gain
```

### Windows

#### Prerequisites

1. **Install ffmpeg** (required for audio analysis)

   Using winget:
   ```powershell
   winget install ffmpeg
   ```

2. **Install mp3gain** (optional, for MP3 support)

   Download from [mp3gain.sourceforge.net](http://mp3gain.sourceforge.net/)

3. **Install Rust** (required to build from source)

   Download and run the installer from [rustup.rs](https://rustup.rs/)

#### Build from Source

```powershell
git clone https://github.com/M-Igashi/headroom.git
cd headroom
cargo build --release
# Binary at target\release\headroom.exe
```

### Linux

#### Prerequisites

```bash
# Ubuntu/Debian
sudo apt install ffmpeg mp3gain

# Fedora
sudo dnf install ffmpeg mp3gain

# Arch
sudo pacman -S ffmpeg mp3gain
```

#### Build from Source

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
git clone https://github.com/M-Igashi/headroom.git
cd headroom
cargo build --release
cp target/release/headroom ~/.local/bin/
```

## Usage

```bash
cd ~/Music/DJ-Tracks
headroom
```

The tool will guide you through:
1. Asking whether to include MP3 files
2. Scanning and analyzing files
3. Reviewing the report
4. Optionally applying gain adjustments
5. Creating backups (recommended)

## Output

### CSV Report

| Filename | Format | LUFS | True Peak (dBTP) | Target (dBTP) | Headroom (dB) | Effective Gain (dB) |
|----------|--------|------|------------------|---------------|---------------|---------------------|
| track01.aif | Lossless | -13.3 | -3.2 | -0.5 | +2.7 | +2.7 |
| track02.mp3 | MP3 | -14.1 | -4.5 | -1.0 | +3.5 | +3.0 |

### Backup Structure

```
./
├── track01.aif              ← Modified
├── track02.mp3              ← Modified  
├── subfolder/
│   └── track03.flac         ← Modified
└── backup/                  ← Created by headroom
    ├── track01.aif          ← Original
    ├── track02.mp3          ← Original
    └── subfolder/
        └── track03.flac     ← Original
```

## Important Notes

- **Files are overwritten in place** after backup — Rekordbox metadata remains linked
- Only files with **positive effective gain** are shown and processed
- MP3 files require at least **1.5dB headroom** to be processed
- macOS resource fork files (`._*`) are automatically ignored

## Technical Details

### Why 1.5dB Steps for MP3?

The MP3 format stores a "global gain" value as an 8-bit integer (0-255). When decoding, samples are multiplied by `2^(gain/4)`:

- +1 to global gain = `2^(1/4)` = **+1.5 dB**
- -1 to global gain = `2^(-1/4)` = **-1.5 dB**

This is a fundamental limitation of the MP3 format, not a tool limitation. The only way to achieve finer precision would be re-encoding, which causes quality loss.

### Why mp3gain Instead of Re-encoding?

| Method | Precision | Quality Loss | Reversible |
|--------|-----------|--------------|------------|
| mp3gain (header modification) | 1.5dB steps | **None** | **Yes** |
| ffmpeg re-encoding | Arbitrary | Slight | No |
| ReplayGain tags | Arbitrary | None | Yes |

ReplayGain tags are not supported by Rekordbox/CDJ, so mp3gain is the best option for DJ workflows.

## Changelog

### v0.3.0
- **MP3 support**: Uses mp3gain for truly lossless gain adjustment (1.5dB steps)
- Added "Effective Gain" column showing actual gain to be applied
- Interactive prompt to include/exclude MP3 files

### v0.2.0
- **Smart True Peak ceiling**: Uses -0.5 dBTP for lossless files based on AES TD1008
- Added Target column to report table

### v0.1.0
- Initial release
- Fixed -1.0 dBTP ceiling

## License

MIT
