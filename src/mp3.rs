//! Pure Rust MP3 global_gain manipulation
//! 
//! This module provides lossless MP3 volume adjustment by modifying
//! the global_gain field in each frame's side information.
//! 
//! Each gain step is 1.5dB (fixed by MP3 specification).
//! Valid global_gain range: 0-255

use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

/// MP3 gain step size in dB (fixed by format specification)
pub const GAIN_STEP_DB: f64 = 1.5;

/// MPEG version
#[derive(Debug, Clone, Copy, PartialEq)]
enum MpegVersion {
    Mpeg1,
    Mpeg2,
    Mpeg25,
}

/// Channel mode
#[derive(Debug, Clone, Copy, PartialEq)]
enum ChannelMode {
    Stereo,
    JointStereo,
    DualChannel,
    Mono,
}

impl ChannelMode {
    fn channel_count(&self) -> usize {
        match self {
            ChannelMode::Mono => 1,
            _ => 2,
        }
    }
}

/// Parsed MP3 frame header
#[derive(Debug, Clone)]
struct FrameHeader {
    version: MpegVersion,
    #[allow(dead_code)]
    #[allow(dead_code)]
    layer: u8,
    has_crc: bool,
    #[allow(dead_code)]
    bitrate_kbps: u32,
    #[allow(dead_code)]
    sample_rate: u32,
    #[allow(dead_code)]
    padding: bool,
    channel_mode: ChannelMode,
    frame_size: usize,
}

impl FrameHeader {
    /// Number of granules per frame
    fn granule_count(&self) -> usize {
        match self.version {
            MpegVersion::Mpeg1 => 2,
            _ => 1,
        }
    }
    
    /// Offset from frame start to side information
    fn side_info_offset(&self) -> usize {
        if self.has_crc { 6 } else { 4 }
    }
}

/// Bitrate table for MPEG1 Layer III
const BITRATE_TABLE_MPEG1_L3: [u32; 15] = [
    0, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320
];

/// Bitrate table for MPEG2/2.5 Layer III
const BITRATE_TABLE_MPEG2_L3: [u32; 15] = [
    0, 8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160
];

/// Sample rate table
const SAMPLE_RATE_TABLE: [[u32; 3]; 3] = [
    [44100, 48000, 32000],  // MPEG1
    [22050, 24000, 16000],  // MPEG2
    [11025, 12000, 8000],   // MPEG2.5
];

/// Parse a 4-byte frame header
fn parse_header(header: &[u8]) -> Option<FrameHeader> {
    if header.len() < 4 {
        return None;
    }
    
    // Check sync word (11 bits: 0xFF + upper 3 bits of second byte)
    if header[0] != 0xFF || (header[1] & 0xE0) != 0xE0 {
        return None;
    }
    
    // MPEG version (bits 4-3 of byte 1)
    let version_bits = (header[1] >> 3) & 0x03;
    let version = match version_bits {
        0b00 => MpegVersion::Mpeg25,
        0b10 => MpegVersion::Mpeg2,
        0b11 => MpegVersion::Mpeg1,
        _ => return None, // Reserved
    };
    
    // Layer (bits 2-1 of byte 1)
    let layer_bits = (header[1] >> 1) & 0x03;
    let layer = match layer_bits {
        0b01 => 3,
        0b10 => 2,
        0b11 => 1,
        _ => return None, // Reserved
    };
    
    // We only support Layer III
    if layer != 3 {
        return None;
    }
    
    // Protection bit (bit 0 of byte 1) - 0 means CRC present
    let has_crc = (header[1] & 0x01) == 0;
    
    // Bitrate index (bits 7-4 of byte 2)
    let bitrate_index = (header[2] >> 4) & 0x0F;
    if bitrate_index == 0 || bitrate_index == 15 {
        return None; // Free/bad
    }
    
    let bitrate_kbps = match version {
        MpegVersion::Mpeg1 => BITRATE_TABLE_MPEG1_L3[bitrate_index as usize],
        _ => BITRATE_TABLE_MPEG2_L3[bitrate_index as usize],
    };
    
    // Sample rate index (bits 3-2 of byte 2)
    let sr_index = ((header[2] >> 2) & 0x03) as usize;
    if sr_index == 3 {
        return None; // Reserved
    }
    
    let version_index = match version {
        MpegVersion::Mpeg1 => 0,
        MpegVersion::Mpeg2 => 1,
        MpegVersion::Mpeg25 => 2,
    };
    let sample_rate = SAMPLE_RATE_TABLE[version_index][sr_index];
    
    // Padding (bit 1 of byte 2)
    let padding = (header[2] & 0x02) != 0;
    
    // Channel mode (bits 7-6 of byte 3)
    let channel_bits = (header[3] >> 6) & 0x03;
    let channel_mode = match channel_bits {
        0b00 => ChannelMode::Stereo,
        0b01 => ChannelMode::JointStereo,
        0b10 => ChannelMode::DualChannel,
        0b11 => ChannelMode::Mono,
        _ => unreachable!(),
    };
    
    // Calculate frame size
    let samples_per_frame = match version {
        MpegVersion::Mpeg1 => 1152,
        _ => 576,
    };
    let padding_size = if padding { 1 } else { 0 };
    let frame_size = (samples_per_frame * bitrate_kbps as usize * 125) / sample_rate as usize + padding_size;
    
    Some(FrameHeader {
        version,
        layer,
        has_crc,
        bitrate_kbps,
        sample_rate,
        padding,
        channel_mode,
        frame_size,
    })
}

/// Location of a global_gain field within the file
#[derive(Debug, Clone)]
struct GainLocation {
    /// Byte offset in file
    byte_offset: usize,
    /// Bit offset within the byte (0-7, MSB first)
    bit_offset: u8,
}

/// Calculate global_gain locations within a frame's side information
fn calculate_gain_locations(
    frame_offset: usize,
    header: &FrameHeader,
) -> Vec<GainLocation> {
    let mut locations = Vec::new();
    let side_info_start = frame_offset + header.side_info_offset();
    
    let num_channels = header.channel_mode.channel_count();
    let num_granules = header.granule_count();
    
    // Bit layout of side information (Layer III):
    // MPEG1 stereo:
    //   main_data_begin: 9 bits
    //   private_bits: 3 bits
    //   scfsi[ch][band]: 4 bits × 2 channels = 8 bits
    //   Total before granules: 20 bits
    //
    // MPEG1 mono:
    //   main_data_begin: 9 bits
    //   private_bits: 5 bits
    //   scfsi[0][band]: 4 bits
    //   Total before granules: 18 bits
    //
    // MPEG2/2.5 stereo:
    //   main_data_begin: 8 bits
    //   private_bits: 2 bits
    //   Total before granules: 10 bits (no scfsi)
    //
    // MPEG2/2.5 mono:
    //   main_data_begin: 8 bits
    //   private_bits: 1 bit
    //   Total before granules: 9 bits (no scfsi)
    
    let bits_before_granules = match (header.version, num_channels) {
        (MpegVersion::Mpeg1, 1) => 18,
        (MpegVersion::Mpeg1, _) => 20,
        (_, 1) => 9,
        (_, _) => 10,
    };
    
    // Granule structure (each channel within granule):
    //   part2_3_length: 12 bits
    //   big_values: 9 bits
    //   global_gain: 8 bits  <-- target
    //   scalefac_compress: 4 bits (MPEG1) or 9 bits (MPEG2)
    //   window_switching_flag: 1 bit
    //   ... (varies based on window_switching_flag)
    //
    // Bits to global_gain within granule: 12 + 9 = 21 bits
    
    // Size of each granule's data in bits
    // MPEG1: 59 bits per channel
    // MPEG2: 63 bits per channel
    let bits_per_granule_channel = match header.version {
        MpegVersion::Mpeg1 => 59,
        _ => 63,
    };
    
    for gr in 0..num_granules {
        for ch in 0..num_channels {
            // Calculate bit offset to this global_gain
            let granule_start_bit = bits_before_granules 
                + (gr * num_channels + ch) * bits_per_granule_channel;
            let global_gain_bit = granule_start_bit + 21; // part2_3_length(12) + big_values(9)
            
            let byte_offset = side_info_start + global_gain_bit / 8;
            let bit_offset = (global_gain_bit % 8) as u8;
            
            locations.push(GainLocation {
                byte_offset,
                bit_offset,
            });
        }
    }
    
    locations
}

/// Read 8-bit value at bit-unaligned position
fn read_gain_at(data: &[u8], loc: &GainLocation) -> u8 {
    let idx = loc.byte_offset;
    if idx >= data.len() {
        return 0;
    }
    
    if loc.bit_offset == 0 {
        data[idx]
    } else if idx + 1 < data.len() {
        // Straddles two bytes
        let shift = loc.bit_offset;
        let high = (data[idx] << shift) as u8;
        let low = data[idx + 1] >> (8 - shift);
        high | low
    } else {
        data[idx] << loc.bit_offset
    }
}

/// Write 8-bit value at bit-unaligned position
fn write_gain_at(data: &mut [u8], loc: &GainLocation, value: u8) {
    let idx = loc.byte_offset;
    if idx >= data.len() {
        return;
    }
    
    if loc.bit_offset == 0 {
        data[idx] = value;
    } else if idx + 1 < data.len() {
        // Straddles two bytes
        let shift = loc.bit_offset;
        let mask_high = 0xFFu8 << (8 - shift);
        let mask_low = 0xFFu8 >> shift;
        
        data[idx] = (data[idx] & mask_high) | (value >> shift);
        data[idx + 1] = (data[idx + 1] & mask_low) | (value << (8 - shift));
    } else {
        // Only partial write possible
        let shift = loc.bit_offset;
        let mask_high = 0xFFu8 << (8 - shift);
        data[idx] = (data[idx] & mask_high) | (value >> shift);
    }
}

/// Skip ID3v2 tag at beginning of data, returns offset to first audio frame
fn skip_id3v2(data: &[u8]) -> usize {
    if data.len() < 10 {
        return 0;
    }
    
    // Check for "ID3" signature
    if &data[0..3] != b"ID3" {
        return 0;
    }
    
    // ID3v2 size is stored as syncsafe integer (7 bits per byte)
    let size = ((data[6] as usize & 0x7F) << 21)
             | ((data[7] as usize & 0x7F) << 14)
             | ((data[8] as usize & 0x7F) << 7)
             | (data[9] as usize & 0x7F);
    
    10 + size
}

/// Apply gain adjustment to MP3 file (lossless)
/// 
/// # Arguments
/// * `file_path` - Path to MP3 file
/// * `gain_steps` - Number of 1.5dB steps to apply (positive = louder)
/// 
/// # Returns
/// * Number of frames modified
pub fn apply_gain(file_path: &Path, gain_steps: i32) -> Result<usize> {
    if gain_steps == 0 {
        return Ok(0);
    }
    
    // Read entire file into memory
    let mut data = fs::read(file_path)
        .with_context(|| format!("Failed to read MP3 file: {}", file_path.display()))?;
    
    let file_size = data.len();
    let mut modified_frames = 0;
    
    // Skip ID3v2 tag
    let mut pos = skip_id3v2(&data);
    
    // Process each frame
    while pos + 4 <= file_size {
        // Try to parse header at current position
        let header = match parse_header(&data[pos..]) {
            Some(h) => h,
            None => {
                // Try to find next frame
                pos += 1;
                continue;
            }
        };
        
        // Validate frame by checking next frame sync
        let next_pos = pos + header.frame_size;
        let valid_frame = if next_pos + 2 <= file_size {
            data[next_pos] == 0xFF && (data[next_pos + 1] & 0xE0) == 0xE0
        } else {
            // Last frame or near end
            next_pos <= file_size
        };
        
        if !valid_frame {
            pos += 1;
            continue;
        }
        
        // Calculate gain locations for this frame
        let locations = calculate_gain_locations(pos, &header);
        
        // Modify each global_gain in the frame
        for loc in &locations {
            let current_gain = read_gain_at(&data, loc);
            
            // Calculate new gain with clamping
            let new_gain = if gain_steps > 0 {
                // Increasing gain
                current_gain.saturating_add(gain_steps.min(255) as u8)
            } else {
                // Decreasing gain - don't wrap, clamp to 0
                let decrease = (-gain_steps).min(255) as u8;
                current_gain.saturating_sub(decrease)
            };
            
            write_gain_at(&mut data, loc, new_gain);
        }
        
        modified_frames += 1;
        
        // Move to next frame
        pos = next_pos;
    }
    
    // Write modified data back
    fs::write(file_path, &data)
        .with_context(|| format!("Failed to write MP3 file: {}", file_path.display()))?;
    
    Ok(modified_frames)
}

/// Convert dB gain to MP3 gain steps
#[allow(dead_code)]
    #[allow(dead_code)]
pub fn db_to_steps(db: f64) -> i32 {
    (db / GAIN_STEP_DB).round() as i32
}

/// Convert MP3 gain steps to dB
#[allow(dead_code)]
    #[allow(dead_code)]
pub fn steps_to_db(steps: i32) -> f64 {
    steps as f64 * GAIN_STEP_DB
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_db_to_steps() {
        assert_eq!(db_to_steps(0.0), 0);
        assert_eq!(db_to_steps(1.5), 1);
        assert_eq!(db_to_steps(3.0), 2);
        assert_eq!(db_to_steps(-1.5), -1);
        assert_eq!(db_to_steps(2.25), 2); // Rounds to nearest
        assert_eq!(db_to_steps(2.26), 2);
    }
    
    #[test]
    fn test_steps_to_db() {
        assert_eq!(steps_to_db(0), 0.0);
        assert_eq!(steps_to_db(1), 1.5);
        assert_eq!(steps_to_db(-2), -3.0);
    }
    
    #[test]
    fn test_parse_valid_header() {
        // Valid MPEG1 Layer 3, 128kbps, 44100Hz, stereo
        let header = [0xFF, 0xFB, 0x90, 0x00];
        let parsed = parse_header(&header);
        assert!(parsed.is_some());
        let h = parsed.unwrap();
        assert_eq!(h.version, MpegVersion::Mpeg1);
        assert_eq!(h.layer, 3);
        assert_eq!(h.bitrate_kbps, 128);
        assert_eq!(h.sample_rate, 44100);
    }
    
    #[test]
    fn test_parse_invalid_header() {
        // Not a valid sync
        assert!(parse_header(&[0x00, 0x00, 0x00, 0x00]).is_none());
        // Valid sync but Layer I
        assert!(parse_header(&[0xFF, 0xFF, 0x90, 0x00]).is_none());
    }
    
    #[test]
    fn test_bit_operations() {
        let mut data = vec![0xAB, 0xCD, 0xEF, 0x12, 0x34];
        
        // Aligned read
        let loc_aligned = GainLocation { byte_offset: 1, bit_offset: 0 };
        assert_eq!(read_gain_at(&data, &loc_aligned), 0xCD);
        
        // Unaligned read (4 bits offset)
        let loc_unaligned = GainLocation { byte_offset: 1, bit_offset: 4 };
        // Should read: lower 4 bits of 0xCD (0x0D) << 4 | upper 4 bits of 0xEF (0x0E)
        // = 0xD0 | 0x0E = 0xDE
        assert_eq!(read_gain_at(&data, &loc_unaligned), 0xDE);
        
        // Aligned write
        write_gain_at(&mut data, &loc_aligned, 0x42);
        assert_eq!(data[1], 0x42);
        
        // Unaligned write
        data = vec![0xAB, 0xCD, 0xEF, 0x12, 0x34];
        write_gain_at(&mut data, &loc_unaligned, 0x99);
        // Should write: 0xC9 to byte 1, 0x9F to byte 2
        assert_eq!(data[1], 0xC9);
        assert_eq!(data[2], 0x9F);
    }
    
    #[test]
    fn test_skip_id3v2() {
        // No ID3v2 tag
        let data_no_tag = vec![0xFF, 0xFB, 0x90, 0x00];
        assert_eq!(skip_id3v2(&data_no_tag), 0);
        
        // ID3v2 tag with size 0
        let data_with_tag = vec![b'I', b'D', b'3', 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF, 0xFB];
        assert_eq!(skip_id3v2(&data_with_tag), 10);
        
        // ID3v2 tag with size 127 (syncsafe: 0x00 0x00 0x00 0x7F)
        let mut data_larger = vec![b'I', b'D', b'3', 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x7F];
        data_larger.extend(vec![0u8; 127]);
        assert_eq!(skip_id3v2(&data_larger), 10 + 127);
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::process::Command;
    
    #[test]
    fn test_apply_gain_real_mp3() {
        // Create test MP3 with ffmpeg
        let test_dir = std::env::temp_dir().join("headroom_test");
        std::fs::create_dir_all(&test_dir).unwrap();
        let test_file = test_dir.join("test_gain.mp3");
        
        // Generate 1 second sine wave MP3
        let output = Command::new("ffmpeg")
            .args([
                "-y", "-f", "lavfi", "-i", "sine=frequency=440:duration=1",
                "-c:a", "libmp3lame", "-b:a", "128k",
                test_file.to_str().unwrap()
            ])
            .output()
            .expect("ffmpeg not found");
        
        assert!(output.status.success(), "Failed to create test MP3");
        
        // Get original file content
        let original = std::fs::read(&test_file).unwrap();
        
        // Apply +2 steps (3dB)
        let frames = apply_gain(&test_file, 2).unwrap();
        assert!(frames > 0, "No frames modified");
        
        // Verify file was modified
        let modified = std::fs::read(&test_file).unwrap();
        assert_eq!(original.len(), modified.len(), "File size should not change");
        assert_ne!(original, modified, "File content should be different");
        
        // Apply -2 steps to restore
        let frames2 = apply_gain(&test_file, -2).unwrap();
        assert_eq!(frames, frames2, "Same number of frames should be modified");
        
        // Content should be back to original
        let restored = std::fs::read(&test_file).unwrap();
        assert_eq!(original, restored, "File should be restored to original");
        
        // Cleanup
        std::fs::remove_file(&test_file).ok();
        std::fs::remove_dir(&test_dir).ok();
    }
}
