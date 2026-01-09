# headroom

Audio loudness analyzer and gain adjustment tool for mastering and DJ workflows.

⭐ **If this tool helps your DJ workflow, please consider giving it a star!** It helps others discover the project.

## What is this?

**headroom** simulates the behavior of Rekordbox's Auto Gain feature, but with a key difference: it identifies files with available headroom (True Peak below the target ceiling) and applies gain adjustment **without using a limiter**.

This tool is designed for DJs and producers who want to maximize loudness while preserving dynamics, ensuring tracks hit the optimal True Peak ceiling without clipping.

## Key Features

- **Smart True Peak ceiling**: Based on AES TD1008, uses -0.5 dBTP for high-quality files, -1.0 dBTP for low-bitrate
- **Three processing methods**: ffmpeg for lossless, mp3gain for lossless MP3, re-encode for precise MP3 gain
- **Non-destructive workflow**: Original files are backed up before processing
- **Metadata preservation**: Files are overwritten in place, so Rekordbox tags, cue points, and other metadata remain intact
- **No limiter**: Pure gain adjustment only — dynamics are preserved
- **Interactive CLI**: Guided step-by-step process with two-stage confirmation

## Supported Formats & Processing Methods

| Format | Extension | Method | Precision | Notes |
|--------|-----------|--------|-----------|-------|
| FLAC | .flac | ffmpeg | Arbitrary | Lossless re-encode |
| AIFF | .aiff, .aif | ffmpeg | Arbitrary | Lossless re-encode |
| WAV | .wav | ffmpeg | Arbitrary | Lossless re-encode |
| MP3 | .mp3 | mp3gain | 1.5dB steps | Truly lossless (header modification) |
| MP3 | .mp3 | ffmpeg re-encode | Arbitrary | For files needing precise gain |

## MP3 Processing: Three-Tier Approach

headroom intelligently chooses the best method for each MP3 file:

### 1. mp3gain (Lossless, -2.0 dBTP ceiling)
For MP3 files with **≥3.5 dB headroom** to -2.0 dBTP:
- Truly lossless header modification
- 1.5 dB step increments
- Fully reversible with `mp3gain -u`
- More conservative ceiling (-2.0 dBTP) accounts for step limitation

### 2. Re-encode (Precise, original ceiling)
For MP3 files with headroom but **<3.5 dB to -2.0 dBTP**:
- Uses ffmpeg for arbitrary precision gain
- Preserves original bitrate
- Requires explicit user confirmation

### 3. Skip (No headroom)
Files already at or above the target ceiling are not processed.

### Why Re-encode MP3 is Safe at High Bitrates

A common concern is quality loss when re-encoding MP3. However, for **gain adjustment only** at high bitrates (≥256 kbps), the degradation is **inaudible to human ears**:

**Technical explanation:**
- MP3 encoding introduces quantization noise primarily in high frequencies (>16kHz)
- At 320kbps, the available bit budget is sufficient to preserve nearly all audible content
- A single re-encode with only gain adjustment (no EQ, no dynamics) produces a waveform nearly identical to the original
- ABX testing consistently shows listeners cannot distinguish 320kbps→320kbps re-encodes

**What matters:**
- Original bitrate: ≥256kbps recommended
- Processing: Gain only (no additional filtering)
- Encode quality: headroom uses libmp3lame with highest quality settings

**Why we still offer the choice:**
- Some users prefer the "zero re-encode" principle
- mp3gain is truly bit-perfect for the audio data
- Re-encode is opt-in with clear explanation

## True Peak Ceiling Strategy

Based on [AES TD1008](https://www.aes.org/technical/documentDownloads.cfm?docID=731) recommendations:

| Format | Method | Ceiling | Rationale |
|--------|--------|---------|-----------|
| Lossless (FLAC, AIFF, WAV) | ffmpeg | **-0.5 dBTP** | Will be distributed via high-bitrate streaming |
| MP3 (mp3gain) | mp3gain | **-2.0 dBTP** | Conservative ceiling for 1.5dB step limitation |
| MP3 ≥256kbps (re-encode) | ffmpeg | **-0.5 dBTP** | High-bitrate codecs have minimal overshoot |
| MP3 <256kbps (re-encode) | ffmpeg | **-1.0 dBTP** | Lower bitrates cause more codec overshoot |

## How It Works

1. Scans the current directory for audio files (FLAC, AIFF, WAV, MP3)
2. Measures LUFS (Integrated Loudness) and True Peak using ffmpeg
3. Categorizes files by processing method:
   - **Green**: Lossless files (ffmpeg)
   - **Yellow**: MP3 files with enough headroom for mp3gain
   - **Magenta**: MP3 files requiring re-encode
4. Displays categorized report
5. Two-stage confirmation:
   - First: "Apply lossless gain adjustment?" (lossless + mp3gain)
   - Second: "Also process MP3 files with re-encoding?" (optional)
6. Creates backups and processes files

### Example

```
$ cd ~/Music/DJ-Tracks
$ headroom

╭─────────────────────────────────────╮
│          headroom v0.5.0            │
│   Audio Loudness Analyzer & Gain    │
╰─────────────────────────────────────╯

▸ Target directory: /Users/xxx/Music/DJ-Tracks

✓ Found 24 audio files
✓ Analyzed 24 files

● 3 lossless files (ffmpeg, precise gain)
  Filename        LUFS    True Peak    Target        Gain
  track01.flac   -13.3    -3.2 dBTP   -0.5 dBTP   +2.7 dB
  track02.aif    -14.1    -4.5 dBTP   -0.5 dBTP   +4.0 dB
  track03.wav    -12.5    -2.8 dBTP   -0.5 dBTP   +2.3 dB

● 2 MP3 files (mp3gain, lossless, 1.5dB steps, target: -2.0 dBTP)
  Filename        LUFS    True Peak    Target        Gain
  track04.mp3    -14.0    -5.5 dBTP   -2.0 dBTP   +3.0 dB
  track05.mp3    -13.5    -6.0 dBTP   -2.0 dBTP   +3.0 dB

● 2 MP3 files (re-encode required for precise gain)
  Filename        LUFS    True Peak    Target        Gain
  track06.mp3    -12.0    -1.5 dBTP   -0.5 dBTP   +1.0 dB
  track07.mp3    -11.5    -1.2 dBTP   -0.5 dBTP   +0.7 dB

✓ Report saved: ./headroom_report_20250109_123456.csv

? Apply lossless gain adjustment to 3 lossless + 2 MP3 (lossless gain) files? [y/N] y

ℹ 2 MP3 files have headroom but require re-encoding for precise gain.
  • Re-encoding causes minor quality loss (inaudible at 256kbps+)
  • Original bitrate will be preserved
? Also process these MP3 files with re-encoding? [y/N] y

? Create backup before processing? [Y/n] y
✓ Backup directory: ./backup

✓ Done! 7 files processed.
  • 3 lossless files (ffmpeg)
  • 2 MP3 files (mp3gain, lossless)
  • 2 MP3 files (re-encoded)
```

## Installation

### macOS (Homebrew)

```bash
brew tap M-Igashi/tap
brew install headroom
```

### Windows

#### Prerequisites

1. **Install ffmpeg** (required for audio analysis)

   Using winget:
   ```powershell
   winget install ffmpeg
   ```

2. **Install mp3gain** (optional, for lossless MP3 gain)

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
1. Scanning and analyzing all audio files
2. Reviewing the categorized report
3. Confirming lossless processing
4. Optionally enabling MP3 re-encoding
5. Creating backups (recommended)

## Output

### CSV Report

| Filename | Format | Bitrate (kbps) | LUFS | True Peak (dBTP) | Target (dBTP) | Headroom (dB) | Method | Effective Gain (dB) |
|----------|--------|----------------|------|------------------|---------------|---------------|--------|---------------------|
| track01.flac | Lossless | - | -13.3 | -3.2 | -0.5 | +2.7 | ffmpeg | +2.7 |
| track04.mp3 | MP3 | 320 | -14.0 | -5.5 | -2.0 | +3.5 | mp3gain | +3.0 |
| track06.mp3 | MP3 | 320 | -12.0 | -1.5 | -0.5 | +1.0 | re-encode | +1.0 |

### Backup Structure

```
./
├── track01.flac             ← Modified
├── track04.mp3              ← Modified  
├── subfolder/
│   └── track06.mp3          ← Modified
└── backup/                  ← Created by headroom
    ├── track01.flac         ← Original
    ├── track04.mp3          ← Original
    └── subfolder/
        └── track06.mp3      ← Original
```

## Important Notes

- **Files are overwritten in place** after backup — Rekordbox metadata remains linked
- Only files with **positive effective gain** are shown and processed
- MP3 mp3gain requires at least **1.5dB headroom to -2.0 dBTP** to be processed losslessly
- MP3 re-encoding is **opt-in** and requires explicit confirmation
- macOS resource fork files (`._*`) are automatically ignored

## Technical Details

### Why 1.5dB Steps for mp3gain?

The MP3 format stores a "global gain" value as an 8-bit integer (0-255). When decoding, samples are multiplied by `2^(gain/4)`:

- +1 to global gain = `2^(1/4)` = **+1.5 dB**
- -1 to global gain = `2^(-1/4)` = **-1.5 dB**

This is a fundamental limitation of the MP3 format, not a tool limitation.

### Why -2.0 dBTP for mp3gain?

With 1.5dB step limitation, we use a more conservative -2.0 dBTP ceiling:
- Ensures the stepped gain doesn't overshoot the safe zone
- Example: File at -3.2 dBTP gets 1 step (+1.5dB) → -1.7 dBTP (safe)
- If we used -0.5 dBTP ceiling, same file would need +2.7dB, but get +1.5dB (waste of potential)

### MP3 Re-encode Quality

When re-encoding is chosen:
- Uses `libmp3lame` encoder (highest quality)
- Preserves original bitrate
- Uses `-q:a 0` (best VBR quality)
- Only applies volume filter (no other processing)

At 320kbps, the re-encode introduces quantization noise below -90dB—far below audible threshold.

### Processing Method Comparison

| Method | Precision | Quality Loss | Reversible | Use Case |
|--------|-----------|--------------|------------|----------|
| ffmpeg (lossless) | Arbitrary | None | Backup only | FLAC, AIFF, WAV |
| mp3gain | 1.5dB steps | **None** | **Yes** | MP3 with ≥3.5dB headroom |
| ffmpeg re-encode | Arbitrary | Inaudible at ≥256kbps | Backup only | MP3 needing precise gain |

## Changelog

### v0.5.2
- **Fix version display**: Banner now shows correct version
- **Fix release workflow**: Remove caveats, add mp3gain as dependency

### v0.5.1
- **Minimum gain threshold**: Skip files with <0.05 dB headroom (avoids processing files with negligible gain)

### v0.5.0
- **Removed MP3 prompt**: All formats scanned by default
- **Three-tier MP3 processing**: mp3gain (lossless) vs re-encode (precise)
- **Categorized report**: Files grouped by processing method with color coding
- **Two-stage confirmation**: Lossless processing first, then optional re-encode
- **Conservative mp3gain ceiling**: -2.0 dBTP for better step utilization

### v0.4.0
- **Smart True Peak ceiling**: Bitrate-aware ceiling for MP3 files

### v0.3.0
- **MP3 support**: Uses mp3gain for truly lossless gain adjustment (1.5dB steps)
- **Bitrate-aware ceiling**: MP3 ≥256kbps uses -0.5 dBTP, <256kbps uses -1.0 dBTP
- Added "Effective Gain" column showing actual gain to be applied
- Interactive prompt to include/exclude MP3 files

### v0.2.0
- **Smart True Peak ceiling**: Uses -0.5 dBTP for lossless files based on AES TD1008
- Added Target column to report table

### v0.1.0
- Initial release
- Fixed -1.0 dBTP ceiling

## Contributing

Found a bug or have a feature request? Please [open an issue](https://github.com/M-Igashi/headroom/issues)!

If headroom has been useful for your DJ sets, consider ⭐ starring the repo — it really helps!

## License

MIT
