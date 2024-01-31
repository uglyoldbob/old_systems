//! Responsible for emulating the details of the audio processing (apu) of the nes console.

use biquad::Biquad;

use common_emulator::audio::{AudioProducerWithRate, AudioSample};

///The modes that the sweep can operate in
#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub enum ApuSweepAddition {
    /// The math uses ones complement numbers
    OnesComplement,
    /// The math uses twos complement numbers
    TwosComplement,
}

/// The nes apu
#[non_exhaustive]
#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct SnesApu {
    /// Used to divide the input clock by 2
    clock: bool,
    /// Status register
    status: u8,
    /// Frame clock
    fclock: u8,
    /// The counter for the frame sequencer
    frame_sequencer_clock: u32,
    /// Variable used for the operation of the frame sequencer
    frame_sequencer_reset: u8,
    /// This flag disables sound on startup
    sound_disabled: bool,
    /// The timer for disabling sound on startup
    sound_disabled_clock: u16,
    /// The timing clock used by the dmc channel
    timing_clock: u32,
    /// The index for generating audio samples
    output_index: f32,
    /// Inhibits clocking the length counters when set
    inhibit_length_clock: bool,
    /// A clock that always runs
    always_clock: usize,
    /// Halt holders for the 4 channels
    pend_halt: [Option<bool>; 4],
}

impl SnesApu {
    /// Build a new apu
    pub fn new() -> Self {
        Self {
            clock: false,
            status: 0,
            fclock: 0,
            frame_sequencer_clock: 0,
            frame_sequencer_reset: 0,
            sound_disabled: true,
            sound_disabled_clock: 0,
            timing_clock: 0,
            output_index: 0.0,
            inhibit_length_clock: false,
            always_clock: 0,
            pend_halt: [None; 4],
        }
    }

    /// Reset the apu
    pub fn reset(&mut self) {
        self.status = 0;
        self.sound_disabled = true;
        self.sound_disabled_clock = 0;
        self.frame_sequencer_reset = 2;
        self.always_clock = 0;
    }

    /// Get the irq line for the apu
    pub fn irq(&self) -> bool {
        false
    }

    /// Get the clock phase
    pub fn get_clock(&self) -> bool {
        self.clock
    }

    /// Set the interrupt flag from the frame sequencer
    fn set_interrupt_flag(&mut self) {
        if (self.fclock & 0x40) == 0 {
            self.status |= 0x40;
        }
    }

    /// Build an audio sample and run the audio filter
    fn build_audio_sample(
        &mut self,
        filter: &mut Option<biquad::DirectForm1<f32>>,
    ) -> Option<AudioSample> {
        let audio = 0.0;
        if let Some(filter) = filter {
            let e = filter.run(audio / 5.0);
            self.output_index += 1.0;
            Some(AudioSample::F32(e.min(1.0).max(0.0)))
        } else {
            None
        }
    }

    /// Clock the apu
    pub fn clock_slow(
        &mut self,
        sound: &mut Vec<&mut AudioProducerWithRate>,
        streams: &mut Vec<std::sync::Weak<std::sync::Mutex<AudioProducerWithRate>>>,
        filter: &mut Option<biquad::DirectForm1<f32>>,
    ) {
        self.always_clock = self.always_clock.wrapping_add(1);

        if self.sound_disabled_clock < 2048 {
            self.sound_disabled_clock += 1;
        } else if self.sound_disabled_clock == 2048 {
            self.sound_disabled = false;
        }
        if let Some(sample) = self.build_audio_sample(filter) {
            for p in sound {
                p.fill_audio_buffer(sample);
                p.fill_audio_buffer(sample);
            }
            for p in streams {
                if let Some(p2) = p.upgrade() {
                    let mut a = p2.lock().unwrap();
                    a.fill_audio_buffer(sample);
                    a.fill_audio_buffer(sample);
                }
            }
        }
    }
}
