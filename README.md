# headroom

Audio loudness analyzer and gain adjustment tool for mastering and DJ workflows.

## What is this?

**headroom** simulates the behavior of Rekordbox's Auto Gain feature, but with a key difference: it identifies files with available headroom (True Peak below the target ceiling) and applies gain adjustment **without using a limiter**.

This tool is designed for DJs and producers who want to maximize loudness while preserving dynamics, ensuring tracks hit the optimal True Peak ceiling without clipping.

## Key Features

- **Single binary**: mp3rgain is built-in as a library — only ffmpeg required as external dependency
- **Smart True Peak ceiling**: Based on AES TD1008, uses -0.5 dBTP for high-quality files, -1.0 dBTP for low-bitrate
- **Multiple processing methods**: ffmpeg for lossless formats, built-in mp3rgain for lossless MP3, ffmpeg re-encode for precise MP3/AAC gain
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
| MP3 | .mp3 | mp3rgain (built-in) | 1.5dB steps | Truly lossless (global_gain modification) |
| MP3 | .mp3 | ffmpeg re-encode | Arbitrary | For files needing precise gain |
| AAC/M4A | .m4a, .aac, .mp4 | ffmpeg re-encode | Arbitrary | Always requires re-encode |

## MP3 Processing: Three-Tier Approach

headroom intelligently chooses the best method for each MP3 file:

### 1. Native Lossless (Built-in mp3rgain, bitrate-aware ceiling)
For MP3 files with ≥1.5 dB headroom to bitrate-aware ceiling:
- Truly lossless global_gain header modification
- 1.5 dB step increments (MP3 format specification)
- Uses built-in [mp3rgain](https://github.com/M-Igashi/mp3rgain) library
- ≥256kbps: -0.5 dBTP ceiling (requires TP ≤ -2.0 dBTP)
- <256kbps: -1.0 dBTP ceiling (requires TP ≤ -2.5 dBTP)

### 2. Re-encode (Precise, bitrate-aware ceiling)
For MP3 files with headroom but <1.5 dB to ceiling:
- Uses ffmpeg for arbitrary precision gain
- Preserves original bitrate
- Requires explicit user confirmation

### 3. Skip (No headroom)
Files already at or above the target ceiling are not processed.

## AAC/M4A Processing

AAC/M4A files are supported with the following characteristics:

- **Always requires re-encoding**: Unlike MP3, AAC has no lossless gain adjustment mechanism
- **Bitrate-aware ceiling**: Same strategy as MP3 re-encode (-0.5 dBTP for ≥256kbps, -1.0 dBTP for lower)
- **High-quality encoder**: Prefers libfdk_aac when available, falls back to built-in aac encoder
- **Preserves original bitrate**: Maintains audio quality matching the source file
- **Grouped with MP3 re-encode**: AAC files are processed together with MP3 re-encode files in the second confirmation stage

### Why Re-encode AAC is Safe at High Bitrates

Same principles apply as MP3 re-encoding:
- At ≥256kbps, quantization noise stays below -90dB (inaudible)
- Only gain is applied (no EQ, compression, or dynamics processing)
- Original bitrate is preserved
- Requires explicit user opt-in

## True Peak Ceiling Strategy

Based on [AES TD1008](https://www.aes.org/technical/documentDownloads.cfm?docID=731) recommendations:

| Format | Method | Ceiling | Rationale |
|--------|--------|---------|-----------|
| Lossless (FLAC, AIFF, WAV) | ffmpeg | **-0.5 dBTP** | Will be distributed via high-bitrate streaming |
| MP3 ≥256kbps (lossless) | mp3rgain | **-0.5 dBTP** | Requires TP ≤ -2.0 dBTP for 1.5dB steps |
| MP3 <256kbps (lossless) | mp3rgain | **-1.0 dBTP** | Requires TP ≤ -2.5 dBTP for 1.5dB steps |
| MP3 ≥256kbps (re-encode) | ffmpeg | **-0.5 dBTP** | High-bitrate codecs have minimal overshoot |
| MP3 <256kbps (re-encode) | ffmpeg | **-1.0 dBTP** | Lower bitrates cause more codec overshoot |
| AAC ≥256kbps | ffmpeg | **-0.5 dBTP** | High-bitrate AAC has minimal overshoot |
| AAC <256kbps | ffmpeg | **-1.0 dBTP** | Lower bitrates cause more codec overshoot |

## How It Works

1. Scans the current directory for audio files (FLAC, AIFF, WAV, MP3, AAC/M4A)
2. Measures LUFS (Integrated Loudness) and True Peak using ffmpeg
3. Categorizes files by processing method:
   - **Green**: Lossless files (ffmpeg)
   - **Yellow**: MP3 files with enough headroom for native lossless gain
   - **Magenta**: MP3/AAC files requiring re-encode
4. Displays categorized report
5. Two-stage confirmation:
   - First: "Apply lossless gain adjustment?" (lossless + native MP3)
   - Second: "Also process files with re-encoding?" (MP3/AAC requiring re-encode)
6. Creates backups and processes files

### Example

```
$ cd ~/Music/DJ-Tracks
$ headroom

╭─────────────────────────────────────╮
│          headroom v1.3.0            │
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

● 2 MP3 files (re-encode required for precise gain)
  Filename        LUFS    True Peak    Target        Gain
  track06.mp3    -12.0    -1.5 dBTP   -0.5 dBTP   +1.0 dB
  track07.mp3    -11.5    -1.2 dBTP   -0.5 dBTP   +0.7 dB

● 2 AAC/M4A files (re-encode required)
  Filename        LUFS    True Peak    Target        Gain
  track08.m4a    -13.0    -2.5 dBTP   -0.5 dBTP   +2.0 dB
  track09.m4a    -12.5    -1.8 dBTP   -0.5 dBTP   +1.3 dB

✓ Report saved: ./headroom_report_20250109_123456.csv

? Apply lossless gain adjustment to 3 lossless + 2 MP3 (lossless gain) files? [y/N] y

ℹ 2 MP3 + 2 AAC/M4A files have headroom but require re-encoding for precise gain.
  • Re-encoding causes minor quality loss (inaudible at 256kbps+)
  • Original bitrate will be preserved
? Also process these files with re-encoding? [y/N] y

? Create backup before processing? [Y/n] y
✓ Backup directory: ./backup

✓ Done! 9 files processed.
  • 3 lossless files (ffmpeg)
  • 2 MP3 files (native, lossless)
  • 2 MP3 files (re-encoded)
  • 2 AAC/M4A files (re-encoded)
```

## Installation

### Quick Install

| Platform | Command |
|----------|---------|
| **macOS** | `brew install M-Igashi/tap/headroom` |
| **Windows (Scoop)** | `scoop bucket add headroom https://github.com/M-Igashi/scoop-bucket && scoop install headroom` |
| **Windows (winget)** | `winget install M-Igashi.headroom` |
| **All platforms** | `cargo install headroom` + install ffmpeg |

### Prerequisites

headroom requires one external tool:
- **ffmpeg**: For audio analysis and lossless format processing

> **Note:** As of v1.3.0, mp3rgain is built-in as a library dependency. No separate installation required.

---

### macOS (Homebrew) — Recommended

```bash
brew install M-Igashi/tap/headroom
```

ffmpeg is installed automatically as a dependency.

---

### Windows (Scoop)

```powershell
scoop bucket add headroom https://github.com/M-Igashi/scoop-bucket
scoop install headroom
```

ffmpeg is installed automatically as a dependency.

---

### Windows (winget)

```powershell
winget install M-Igashi.headroom
```

Then install ffmpeg:

```powershell
winget install ffmpeg
```

---

### Cargo (All Platforms)

If you have Rust installed, you can install headroom via cargo:

```bash
cargo install headroom
```

Then install ffmpeg for your platform:

```bash
# macOS
brew install ffmpeg

# Ubuntu/Debian
sudo apt install ffmpeg

# Fedora
sudo dnf install ffmpeg

# Arch
sudo pacman -S ffmpeg

# Windows (winget)
winget install ffmpeg

# Windows (choco)
choco install ffmpeg
```

---

### Pre-built Binaries

Download pre-built binaries from the [Releases](https://github.com/M-Igashi/headroom/releases) page:

| Platform | File |
|----------|------|
| macOS (Universal) | `headroom-vX.X.X-macos-universal.tar.gz` |
| Linux x86_64 | `headroom-vX.X.X-linux-x86_64.tar.gz` |
| Linux ARM64 | `headroom-vX.X.X-linux-aarch64.tar.gz` |
| Windows x86_64 | `headroom-vX.X.X-windows-x86_64.zip` |

**Note:** You must install ffmpeg separately (see platform-specific commands above).

---

### Build from Source

```bash
git clone https://github.com/M-Igashi/headroom.git
cd headroom
cargo build --release

# Binary location:
# - Unix: target/release/headroom
# - Windows: target\release\headroom.exe
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
| track08.m4a | AAC | 256 | -13.0 | -2.5 | -0.5 | +2.0 | re-encode | +2.0 |

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
- MP3 native lossless requires at least **1.5dB headroom to -2.0 dBTP** to be processed
- MP3/AAC re-encoding is **opt-in** and requires explicit confirmation
- macOS resource fork files (`._*`) are automatically ignored

## Technical Details

### Why 1.5dB Steps for Native MP3 Gain?

The MP3 format stores a "global_gain" value as an 8-bit integer (0-255). When decoding, samples are multiplied by `2^(gain/4)`:

- +1 to global_gain = `2^(1/4)` = **+1.5 dB**
- -1 to global_gain = `2^(-1/4)` = **-1.5 dB**

This is a fundamental limitation of the MP3 format, not a tool limitation. headroom uses the built-in [mp3rgain](https://github.com/M-Igashi/mp3rgain) library to directly manipulate this field in each MP3 frame's side information.

### Why Bitrate-Aware Ceiling for Native MP3?

With 1.5dB step limitation, the ceiling is calculated based on bitrate to match re-encode targets:
- **≥256kbps**: Target -0.5 dBTP, so native lossless requires TP ≤ -2.0 dBTP (allowing at least 1 step)
- **<256kbps**: Target -1.0 dBTP, so native lossless requires TP ≤ -2.5 dBTP (more conservative)
- Example: 320kbps file at -3.5 dBTP gets 2 steps (+3.0dB) → -0.5 dBTP (optimal)
- Example: 128kbps file at -3.5 dBTP gets 1 step (+1.5dB) → -2.0 dBTP (within -1.0 ceiling)

### Why AAC Always Requires Re-encoding

Unlike MP3, AAC doesn't have a lossless gain adjustment mechanism like the global_gain field. The only way to apply gain to AAC files is through re-encoding. However, at high bitrates (≥256kbps), the quality loss is imperceptible.

### MP3/AAC Re-encode Quality

When re-encoding is chosen:
- **MP3**: Uses `libmp3lame` encoder with `-q:a 0` (best VBR quality)
- **AAC**: Prefers `libfdk_aac` (highest quality), falls back to built-in `aac` encoder
- Preserves original bitrate
- Only applies volume filter (no other processing)

At 320kbps, the re-encode introduces quantization noise below -90dB—far below audible threshold.

### Processing Method Comparison

| Method | Format | Precision | Quality Loss | External Deps | Use Case |
|--------|--------|-----------|--------------|---------------|----------|
| ffmpeg (lossless) | FLAC, AIFF, WAV | Arbitrary | None | ffmpeg | Lossless files |
| mp3rgain (built-in) | MP3 | 1.5dB steps | **None** | None | MP3 with ≥1.5dB to bitrate ceiling |
| ffmpeg re-encode | MP3 | Arbitrary | Inaudible at ≥256kbps | ffmpeg | MP3 needing precise gain |
| ffmpeg re-encode | AAC/M4A | Arbitrary | Inaudible at ≥256kbps | ffmpeg | AAC files (always) |

## License

MIT
