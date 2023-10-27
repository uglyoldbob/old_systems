//! This is the module for recording related code

use gstreamer::prelude::{Cast, ElementExt, ElementExtManual, GstBinExtManual, PadExt, GstObjectExt};

use crate::{apu::AudioProducerWithRate, AudioConsumer};

/// The main struct for recording related activities
pub struct Recording {
    /// The pipeline for recording to disk
    record_pipeline: Option<gstreamer::Pipeline>,
    /// The source for video data from the emulator
    record_source: Option<gstreamer_app::AppSrc>,
    /// The audio source for the recording
    audio: Option<crate::apu::AudioProducerWithRate>,
}

impl Recording {
    /// Create a recording object
    pub fn new() -> Self {
        Self {
            record_pipeline: None,
            record_source: None,
            audio: None,
        }
    }

    /// Returns an optional sound source
    pub fn get_sound(&mut self) -> &mut Option<crate::apu::AudioProducerWithRate> {
        &mut self.audio
    }

    /// Returns true if recording
    pub fn is_recording(&self) -> bool {
        self.record_pipeline.is_some()
    }

    /// Start recording by setting up the necessary objects.
    pub fn start(
        &mut self,
        have_gstreamer: &Result<(), gstreamer::glib::Error>,
        image: &crate::ppu::PixelImage<egui_multiwin::egui::Color32>,
        framerate: u8,
        name: String,
        interval: f32,
    ) {
        if self.record_pipeline.is_none() {
            if have_gstreamer.is_ok() {
                let version = gstreamer::version_string().as_str().to_string();
                println!("GStreamer version is {}", version);
                let app_source = gstreamer_app::AppSrc::builder()
                    .name("emulator_video")
                    .format(gstreamer::Format::Time)
                    .build();
                let ainfo = gstreamer_audio::AudioInfo::builder(
                    gstreamer_audio::AudioFormat::F32le,
                    44100,
                    2,
                )
                .build()
                .unwrap();
                app_source.set_block(false);
                let audio_caps = ainfo.to_caps().unwrap();
                let audio_source = gstreamer_app::AppSrc::builder()
                    .name("emulator_audio")
                    .caps(&audio_caps)
                    .format(gstreamer::Format::Time)
                    .build();
                audio_source.set_block(false);
                let vsource = gstreamer::ElementFactory::make("videoparse")
                    .name("vparse")
                    .property_from_str("framerate", format!("{}/1", framerate).as_str())
                    .property_from_str("width", format!("{}", image.width).as_str())
                    .property_from_str("height", format!("{}", image.height).as_str())
                    .property_from_str("format", "rgb")
                    .build()
                    .expect("Could not create source element.");
                let vconv = gstreamer::ElementFactory::make("videoconvert")
                    .name("vconvert")
                    .build()
                    .expect("Could not create source element.");
                let aconv = gstreamer::ElementFactory::make("audioconvert")
                    .name("aconvert")
                    .build()
                    .expect("Could not create source element.");
                let aencoder = gstreamer::ElementFactory::make("alawenc")
                    .name("aencode")
                    .build()
                    .expect("Could not create source element.");
                let vencoder = gstreamer::ElementFactory::make("openh264enc")
                    .name("vencode")
                    .build()
                    .expect("Could not create source element.");
                let avimux = gstreamer::ElementFactory::make("avimux")
                    .name("avi")
                    .build()
                    .expect("Could not create source element.");

                let aqueue = gstreamer::ElementFactory::make("queue")
                    .name("aqueue")
                    .build()
                    .expect("Could not create source element.");
                let vqueue = gstreamer::ElementFactory::make("queue")
                    .name("vqueue")
                    .build()
                    .expect("Could not create source element.");

                let sink = gstreamer::ElementFactory::make("filesink")
                    .name("sink")
                    .property_from_str("location", name.as_str())
                    .build()
                    .expect("Could not create sink element");

                let pipeline = gstreamer::Pipeline::with_name("recording-pipeline");
                pipeline
                    .add_many([
                        app_source.upcast_ref(),
                        audio_source.upcast_ref(),
                        &aencoder,
                        &vsource,
                        &vconv,
                        &aconv,
                        &aqueue,
                        &vencoder,
                        &vqueue,
                        &avimux,
                        &sink,
                    ])
                    .unwrap();
                gstreamer::Element::link_many([
                    app_source.upcast_ref(),
                    &vsource,
                    &vconv,
                    &vencoder,
                    &vqueue,
                ])
                .unwrap();
                gstreamer::Element::link_many([
                    audio_source.upcast_ref(), 
                    &aconv,
                    &aencoder,
                    &aqueue]).unwrap();

                aqueue.link(&avimux).unwrap();
                vqueue.link(&avimux).unwrap();
                avimux.link(&sink).unwrap();

                audio_source.set_stream_type(gstreamer_app::AppStreamType::Stream);
                self.audio = Some(AudioProducerWithRate::new_gstreamer(4410, interval, audio_source));

                pipeline
                    .set_state(gstreamer::State::Playing)
                    .expect("Unable to set the pipeline to the `Playing` state");

                self.record_source = Some(app_source);
                self.record_pipeline = Some(pipeline);

                
            }
        }
    }

    /// Send a frame of data to the recording
    pub fn send_frame(&mut self, image: &crate::ppu::PixelImage<egui_multiwin::egui::Color32>) {
        if let Some(_pipeline) = &mut self.record_pipeline {
            if let Some(source) = &mut self.record_source {
                let mut buf =
                    gstreamer::Buffer::with_size(image.width as usize * image.height as usize * 3)
                        .unwrap();
                image.to_gstreamer(image.width as usize, image.height as usize, &mut buf);
                match source.push_buffer(buf) {
                    Ok(a) => {}
                    Err(e) => {
                        println!("Error pushing video data: {:?}", e);
                    }
                }
            }
        }
    }

    /// Stop recording
    pub fn stop(&mut self) {
        if let Some(pipeline) = &mut self.record_pipeline {

            let dot = gstreamer::debug_bin_to_dot_data(pipeline, gstreamer::DebugGraphDetails::all());
                std::fs::write("./pipeline.dot", dot).expect("Unable to write pipeline file");

            if let Some(source) = &mut self.record_source {
                source.end_of_stream();
            }
            pipeline
                .set_state(gstreamer::State::Null)
                .expect("Unable to set the recording pipeline to the `Null` state");
        }
        self.record_pipeline = None;
        self.record_source = None;
    }
}
