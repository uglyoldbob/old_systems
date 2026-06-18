use common_emulator::video::RgbImage;
use std::time::Duration;

fn main() {
    gstreamer::init().expect("GStreamer init failed");

    let mut so = common_emulator::streaming::StreamingOut::new();
    so.start(256, 240, 60, 1_000_000.0);
    std::thread::sleep(Duration::from_millis(500));

    let samples_per_frame = 44100 / 50;
    let mut appsink = so.take_sink().expect("Failed to take sink");
    let mut aud = so.get_sound().expect("Failed to get audio");

    let mut si = common_emulator::streaming::StreamingIn::new();
    si.start(44100);
    std::thread::sleep(Duration::from_millis(500));

    let mut total_bytes_sent = 0usize;
    let mut encoded_frames = 0;
    let mut decoded_frames = 0;

    for _ in 0..(44100-735) {
        aud.direct_fill_audio_buffer(common_emulator::audio::AudioSample::F32(0.0));
    }

    for i in 0..1600 {
        let fill = (i % 256) as u8;
        let mut image = RgbImage::new(256, 240);
        so.send_video_buffer(image.to_pixels_egui().to_gstreamer_vec());

        let audio: Vec<f32> = (0..samples_per_frame)
            .map(|s| ((s as f32 / samples_per_frame as f32) * std::f32::consts::TAU).sin() * 0.5)
            .collect();
        for sample in &audio {
            aud.direct_fill_audio_buffer(common_emulator::audio::AudioSample::F32(*sample));
        }

        loop {
            match appsink.try_pull_sample(gstreamer::ClockTime::from_mseconds(0)) {
                Some(sample) => {
                    let buffer = sample.buffer().unwrap();
                    let map = buffer.map_readable().unwrap();
                    let data = map.as_slice().to_vec();
                    total_bytes_sent += data.len();
                    encoded_frames += 1;
                    println!("[frame {i}] encoded {} bytes (total: {})", data.len(), total_bytes_sent);
                    si.send_data(data);
                }
                None => break,
            }
        }

        let pull_timeout = if i < 60 {
            gstreamer::ClockTime::from_mseconds(50)
        } else {
            gstreamer::ClockTime::from_mseconds(500)
        };

        // video_source() returns &mut Option<AppSink> — match directly
        if let Some(vsrc) = si.video_source() {
            match vsrc.try_pull_sample(pull_timeout) {
                Some(vsam) => {
                    let vmap = vsam.buffer().unwrap().map_readable().unwrap();
                    println!("[frame {i}] DECODED {} bytes ✓", vmap.len());
                    decoded_frames += 1;
                }
                None if i >= 60 => {
                    eprintln!("[frame {i}] no decoded output yet ({total_bytes_sent} bytes sent)");
                }
                None => {}
            }
        }
    }

    println!("\n=== Results ===");
    println!("Encoded frames: {encoded_frames}");
    println!("Bytes forwarded: {total_bytes_sent}");
    println!("Decoded frames: {decoded_frames}");
    assert!(encoded_frames > 0, "FAIL: StreamingOut produced nothing");
    assert!(decoded_frames > 0, "FAIL: StreamingIn decoded nothing");
    println!("PASS");
}