# headroom

Audio loudness analyzer and gain adjustment tool for mastering and DJ workflows.

⭐ **If this tool helps your DJ workflow, please consider giving it a star!** It helps others discover the project.

## What is this?

**headroom** simulates the behavior of Rekordbox's Auto Gain feature, but with a key difference: it identifies files with available headroom (True Peak below the target ceiling) and applies gain adjustment **without using a limiter**.

This tool is designed for DJs and producers who want to maximize loudness while preserving dynamics, ensuring tracks hit the optimal True Peak ceiling without clipping.

## Key Features

- **External tools**: Uses [mp3rgain](https://github.com/M-Igashi/mp3rgain) CLI for lossless MP3 gain, ffmpeg for analysis & lossless formats
- **Smart True Peak ceiling**: Based on AES TD1008, uses -0.5 dBTP for high-quality files, -1.0 dBTP for low-bitrate
- **Three processing methods**: ffmpeg for lossless, native implementation for lossless MP3, re-encode for precise MP3 gain
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
| MP3 | .mp3 | native | 1.5dB steps | Truly lossless (global_gain modification) |
| MP3 | .mp3 | ffmpeg re-encode | Arbitrary | For files needing precise gain |

## MP3 Processing: Three-Tier Approach

headroom intelligently chooses the best method for each MP3 file:

### 1. Native Lossless (Pure Rust, bitrate-aware ceiling)
For MP3 files with ≥1.5 dB headroom to bitrate-aware ceiling:
- Truly lossless global_gain header modification
- 1.5 dB step increments (MP3 format specification)
- Uses mp3rgain CLI tool
- ≥256kbps: -0.5 dBTP ceiling (requires TP ≤ -2.0 dBTP)
- <256kbps: -1.0 dBTP ceiling (requires TP ≤ -2.5 dBTP)

### 2. Re-encode (Precise, bitrate-aware ceiling)
For MP3 files with headroom but <1.5 dB to ceiling:
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
- Native lossless is truly bit-perfect for the audio data
- Re-encode is opt-in with clear explanation

## True Peak Ceiling Strategy

Based on [AES TD1008](https://www.aes.org/technical/documentDownloads.cfm?docID=731) recommendations:

| Format | Method | Ceiling | Rationale |
|--------|--------|---------|-----------|
| Lossless (FLAC, AIFF, WAV) | ffmpeg | **-0.5 dBTP** | Will be distributed via high-bitrate streaming |
| MP3 ≥256kbps (native) | native | **-0.5 dBTP** | Requires TP ≤ -2.0 dBTP for 1.5dB steps |
| MP3 <256kbps (native) | native | **-1.0 dBTP** | Requires TP ≤ -2.5 dBTP for 1.5dB steps |
| MP3 ≥256kbps (re-encode) | ffmpeg | **-0.5 dBTP** | High-bitrate codecs have minimal overshoot |
| MP3 <256kbps (re-encode) | ffmpeg | **-1.0 dBTP** | Lower bitrates cause more codec overshoot |

## How It Works

1. Scans the current directory for audio files (FLAC, AIFF, WAV, MP3)
2. Measures LUFS (Integrated Loudness) and True Peak using ffmpeg
3. Categorizes files by processing method:
   - **Green**: Lossless files (ffmpeg)
   - **Yellow**: MP3 files with enough headroom for native lossless gain
   - **Magenta**: MP3 files requiring re-encode
4. Displays categorized report
5. Two-stage confirmation:
   - First: "Apply lossless gain adjustment?" (lossless + native MP3)
   - Second: "Also process MP3 files with re-encoding?" (optional)
6. Creates backups and processes files

### Example

```
$ cd ~/Music/DJ-Tracks
$ headroom

╭─────────────────────────────────────╮
│          headroom v1.0.0            │
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

● 2 MP3 files (native lossless, 1.5dB steps, target: -2.0 dBTP)
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
  • 2 MP3 files (native, lossless)
  • 2 MP3 files (re-encoded)
```

## Installation

### macOS (Homebrew)

```bash
brew tap M-Igashi/tap
brew install headroom
```

That's it! ffmpeg is installed automatically as a dependency.

### Windows

#### Prerequisites

1. **Install ffmpeg** (required for audio analysis)

   Using winget:
   ```powershell
   winget install ffmpeg
   ```

2. **Install Rust** (required to build from source)

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
sudo apt install ffmpeg

# Fedora
sudo dnf install ffmpeg

# Arch
sudo pacman -S ffmpeg
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
| track04.mp3 | MP3 | 320 | -14.0 | -5.5 | -2.0 | +3.5 | native | +3.0 |
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
- MP3 native lossless requires at least **1.5dB headroom to -2.0 dBTP** to be processed
- MP3 re-encoding is **opt-in** and requires explicit confirmation
- macOS resource fork files (`._*`) are automatically ignored

## Technical Details

### Why 1.5dB Steps for Native MP3 Gain?

The MP3 format stores a "global_gain" value as an 8-bit integer (0-255). When decoding, samples are multiplied by `2^(gain/4)`:

- +1 to global_gain = `2^(1/4)` = **+1.5 dB**
- -1 to global_gain = `2^(-1/4)` = **-1.5 dB**

This is a fundamental limitation of the MP3 format, not a tool limitation. headroom uses the [mp3rgain](https://github.com/M-Igashi/mp3rgain) CLI tool to directly manipulate this field in each MP3 frame's side information.

### Why Bitrate-Aware Ceiling for Native MP3?

With 1.5dB step limitation, the ceiling is calculated based on bitrate to match re-encode targets:
- **≥256kbps**: Target -0.5 dBTP, so native lossless requires TP ≤ -2.0 dBTP (allowing at least 1 step)
- **<256kbps**: Target -1.0 dBTP, so native lossless requires TP ≤ -2.5 dBTP (more conservative)
- Example: 320kbps file at -3.5 dBTP gets 2 steps (+3.0dB) → -0.5 dBTP (optimal)
- Example: 128kbps file at -3.5 dBTP gets 1 step (+1.5dB) → -2.0 dBTP (within -1.0 ceiling)

### MP3 Re-encode Quality

When re-encoding is chosen:
- Uses `libmp3lame` encoder (highest quality)
- Preserves original bitrate
- Uses `-q:a 0` (best VBR quality)
- Only applies volume filter (no other processing)

At 320kbps, the re-encode introduces quantization noise below -90dB—far below audible threshold.

### Processing Method Comparison

| Method | Precision | Quality Loss | External Deps | Use Case |
|--------|-----------|--------------|---------------|----------|
| ffmpeg (lossless) | Arbitrary | None | ffmpeg | FLAC, AIFF, WAV |
| native (MP3) | 1.5dB steps | **None** | None | MP3 with ≥1.5dB to bitrate ceiling |
| ffmpeg re-encode | Arbitrary | Inaudible at ≥256kbps | ffmpeg | MP3 needing precise gain |

## Contributing

Found a bug or have a feature request? Please [open an issue](https://github.com/M-Igashi/headroom/issues)!

If headroom has been useful for your DJ sets, consider ⭐ starring the repo — it really helps!

## License

MIT
