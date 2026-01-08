# headroom

Audio loudness analyzer and gain adjustment tool for mastering workflows.

## Features

- Scan audio files (.flac, .aiff, .aif, .wav)
- Measure LUFS (Integrated Loudness) and True Peak
- Calculate available headroom to -1 dBTP ceiling
- Generate CSV reports
- Optional: Auto-process with backup, gain adjustment, and renaming

## Installation

```bash
# Coming soon
```

## Usage

```bash
# Analyze files in a directory
headroom ./path/to/audio/

# Generate report only
headroom ./path/to/audio/ --report-only

# Auto-process files
headroom ./path/to/audio/ --apply
```

## Output

| Filename | LUFS | True Peak | Headroom |
|----------|------|-----------|----------|
| track01.wav | -14.2 | -0.3 dBTP | 0.7 dB |
| track02.flac | -12.8 | -1.5 dBTP | -0.5 dB |

## License

MIT
