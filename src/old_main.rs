extern crate rodio;

use rodio::{buffer::SamplesBuffer, OutputStream, Sink, Source};
use std::fs::File;
use std::io::Read;
use std::thread::sleep;
use std::time::Duration;

fn main() {
    // Open and read the .tap file
    let mut file = File::open("./Android1.tap").expect("Failed to open file");
    let mut tap_data = Vec::new();
    file.read_to_end(&mut tap_data)
        .expect("Failed to read file");

    // Parse the .tap file and extract the audio data
    let audio_data = extract_audio_data(&tap_data).unwrap();

    // Initialize audio output
    let (_stream, stream_handle) =
        OutputStream::try_default().expect("Failed to create audio stream");
    let sink = Sink::try_new(&stream_handle).expect("Failed to create sink");

    // Play the audio
    sink.append(audio_data);
    sink.play();

    // Sleep for a while to allow the audio to play
    sleep(Duration::from_secs(5));

    // Stop playback
    sink.stop();
}

fn extract_audio_data(tap_data: &[u8]) -> Option<SamplesBuffer<i16>> {
    let mut audio_data = Vec::new();
    let mut i = 0;

    while i < tap_data.len() {
        println!("tap_data.len(): {}", tap_data.len());
        println!("i: {}", i);
        // Read the block length
        let block_length = tap_data[i] as usize + ((tap_data[i + 1] as usize) << 8);
        println!("block_length: {}", block_length);

        // Check the block type
        let block_type = tap_data[i + 2];
        println!("block_type: {}", block_type);
        i += 3;

        match block_type {
            0x10 => {
                // This is a pure tone block
                // Read the duration in milliseconds
                let duration_ms = tap_data[i] as u64 + ((tap_data[i + 1] as u64) << 8);
                i += 2;

                // Read the pulse length in cycles
                let pulse_length = tap_data[i] as u16 + ((tap_data[i + 1] as u16) << 8);
                i += 2;

                // Generate the audio waveform for the pure tone
                let audio_samples = generate_pure_tone(duration_ms, pulse_length);

                // Append the audio samples to the audio_data vector
                audio_data.extend(audio_samples);
            }
            0x11 => {
                // Sequence of Pulses of Different Lengths
                let end_of_block = i + block_length;

                while i < end_of_block {
                    // Read the duration of each pulse
                    let pulse_duration_ms = tap_data[i] as u64 + ((tap_data[i + 1] as u64) << 8);
                    i += 2;

                    // Generate the audio waveform for the pulse
                    let pulse_samples = generate_pulse(pulse_duration_ms);
                    audio_data.extend(pulse_samples);

                    // Optionally, add a brief period of silence between pulses
                    let silence_samples = generate_silence(5); // 5 ms of silence
                    audio_data.extend(silence_samples);
                }
            }
            0x12 => {
                // Standard Data Block
                let end_of_block = i + block_length;

                while i < end_of_block {
                    // Read a byte of data
                    let byte = tap_data[i];
                    i += 1;

                    // Process each bit in the byte
                    for bit in 0..8 {
                        let is_bit_set = byte & (1 << bit) != 0;

                        // Generate audio for the bit
                        let bit_samples = if is_bit_set {
                            generate_pulse_high(30) // Duration for a '1' bit (e.g., 30ms)
                        } else {
                            generate_pulse_low(30) // Duration for a '0' bit (e.g., 30ms)
                        };
                        audio_data.extend(bit_samples);
                    }
                }
            }
            _ => {
                // Skip other block types
                i += block_length;
            }
        }
    }

    if !audio_data.is_empty() {
        let sample_rate = 44100;
        Some(SamplesBuffer::new(1, sample_rate, audio_data))
    } else {
        None
    }
}

fn generate_pure_tone(duration_ms: u64, pulse_length: u16) -> Vec<i16> {
    // Sample rate and frequency settings
    let sample_rate = 44100; // Sample rate in Hz
    let frequency = 1500; // Frequency in Hz (adjust as needed)

    // Calculate the number of samples required for the specified duration
    let num_samples = (duration_ms * sample_rate) / 1000;

    // Calculate the number of samples for each half of the square wave
    let half_pulse_length_samples = (sample_rate / (2 * frequency)) as usize;

    // Generate the square wave audio waveform
    let mut audio_samples = Vec::new();
    for _ in 0..(num_samples / half_pulse_length_samples as u64) {
        // Generate the positive half of the square wave
        audio_samples.extend(vec![255; half_pulse_length_samples]);

        // Generate the negative half of the square wave
        audio_samples.extend(vec![0; half_pulse_length_samples]);
    }

    // Add any remaining samples to complete the specified duration
    let remaining_samples = (num_samples % half_pulse_length_samples as u64) as usize;
    if remaining_samples > 0 {
        audio_samples.extend(vec![255; remaining_samples]);
    }

    audio_samples
        .iter()
        .map(|&sample| {
            // Map 0-255 to -32768 to 32767
            ((sample as i16) - 128) * 256
        })
        .collect()
}

fn generate_pulse(duration_ms: u64) -> Vec<i16> {
    let frequency = 1500; // Frequency of the pulse in Hz
    let sample_rate = 44100; // Sample rate in Hz
    let num_samples = (duration_ms * sample_rate) / 1000; // Total number of samples for the duration

    // Calculate the number of samples for each half of the square wave
    let half_pulse_length_samples = (sample_rate / (2 * frequency)) as usize;

    let mut pulse_samples = Vec::new();
    for _ in 0..(num_samples / half_pulse_length_samples as u64) {
        // Generate the positive half of the square wave
        pulse_samples.extend(vec![32767; half_pulse_length_samples]);

        // Generate the negative half of the square wave
        pulse_samples.extend(vec![-32768; half_pulse_length_samples]);
    }

    // Add any remaining samples to complete the specified duration
    let remaining_samples = (num_samples % half_pulse_length_samples as u64) as usize;
    if remaining_samples > 0 {
        pulse_samples.extend(vec![32767; remaining_samples]);
    }

    pulse_samples
}

fn generate_pulse_high(duration_ms: u64) -> Vec<i16> {
    // Similar to generate_pulse, but with a different frequency
    generate_pulse_generic(duration_ms, 2500) // High frequency for '1' bit
}

fn generate_pulse_low(duration_ms: u64) -> Vec<i16> {
    // Similar to generate_pulse, but with a different frequency or silence
    // For simplicity, let's use silence for '0' bit
    generate_silence(duration_ms)
}

fn generate_pulse_generic(duration_ms: u64, frequency: u16) -> Vec<i16> {
    // Generic pulse generation function
    let sample_rate = 44100;
    let num_samples = (duration_ms * sample_rate) / 1000;

    let half_pulse_length_samples = (sample_rate / (2 * frequency as u64)) as usize;

    let mut pulse_samples = Vec::new();
    for _ in 0..(num_samples / half_pulse_length_samples as u64) {
        pulse_samples.extend(vec![32767; half_pulse_length_samples]);
        pulse_samples.extend(vec![-32768; half_pulse_length_samples]);
    }

    let remaining_samples = (num_samples % half_pulse_length_samples as u64) as usize;
    if remaining_samples > 0 {
        pulse_samples.extend(vec![32767; remaining_samples]);
    }

    pulse_samples
}

fn generate_silence(duration_ms: u64) -> Vec<i16> {
    let sample_rate = 44100; // Sample rate in Hz
    let num_samples = (duration_ms * sample_rate) / 1000; // Total number of samples for the duration

    vec![0; num_samples as usize] // Fill the vector with zeros
}
