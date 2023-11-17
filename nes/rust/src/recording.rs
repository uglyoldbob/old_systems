//! This is the module for recording related code

use std::path::PathBuf;

use gstreamer::prelude::{Cast, ElementExt, ElementExtManual, GstBinExtManual};

use crate::apu::AudioProducerWithRate;

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
        name: PathBuf,
        cpu_frequency: f32,
    ) {
        if self.record_pipeline.is_none() && have_gstreamer.is_ok() {
            let version = gstreamer::version_string().as_str().to_string();
            println!("GStreamer version is {}", version);
            let vinfo = gstreamer_video::VideoInfo::builder(
                gstreamer_video::VideoFormat::Rgb,
                image.width as u32,
                image.height as u32,
            )
            .fps(framerate as i32)
            .build()
            .unwrap();
            let video_caps = vinfo.to_caps().unwrap();
            let app_source = gstreamer_app::AppSrc::builder()
                .name("emulator_video")
                .caps(&video_caps)
                .format(gstreamer::Format::Time)
                .build();
            let ainfo =
                gstreamer_audio::AudioInfo::builder(gstreamer_audio::AudioFormat::F32le, 44100, 2)
                    .build()
                    .unwrap();
            let audio_caps = ainfo.to_caps().unwrap();
            let audio_source = gstreamer_app::AppSrc::builder()
                .name("emulator_audio")
                .caps(&audio_caps)
                .format(gstreamer::Format::Time)
                .build();
            audio_source.set_block(true);
            app_source.set_do_timestamp(true);
            app_source.set_is_live(true);
            audio_source.set_is_live(true);
            app_source.set_block(false);
            audio_source.set_do_timestamp(true);

            let vbitrate = &format!(
                "{}",
                image.width as u32 * image.height as u32 * framerate as u32 / 8
            );
            println!("Video birate is calculated as {}", vbitrate);

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
                .property_from_str("bitrate", vbitrate)
                .build()
                .expect("Could not create source element.");
            let avimux = gstreamer::ElementFactory::make("avimux")
                .name("avi")
                .build()
                .expect("Could not create source element.");

            let aresample = gstreamer::ElementFactory::make("audioresample")
                .name("aresample")
                .build()
                .expect("Could not create source element.");

            let sink = gstreamer::ElementFactory::make("filesink")
                .name("sink")
                .property_from_str("location", &name.into_os_string().into_string().unwrap())
                .build()
                .expect("Could not create sink element");

            let pipeline = gstreamer::Pipeline::with_name("recording-pipeline");
            pipeline
                .add_many([
                    app_source.upcast_ref(),
                    audio_source.upcast_ref(),
                    &aencoder,
                    &vconv,
                    &aconv,
                    &aresample,
                    &vencoder,
                    &avimux,
                    &sink,
                ])
                .unwrap();
            gstreamer::Element::link_many([app_source.upcast_ref(), &vconv, &vencoder]).unwrap();
            gstreamer::Element::link_many([
                audio_source.upcast_ref(),
                &aconv,
                &aresample,
                &aencoder,
            ])
            .unwrap();

            aencoder.link(&avimux).unwrap();
            vencoder.link(&avimux).unwrap();
            avimux.link(&sink).unwrap();

            pipeline
                .set_state(gstreamer::State::Playing)
                .expect("Unable to set the pipeline to the `Playing` state");

            self.record_source = Some(app_source);
            self.record_pipeline = Some(pipeline);

            self.audio = Some(AudioProducerWithRate::new_gstreamer(
                44100,
                cpu_frequency / 44100.0,
                audio_source,
            ));
        }
    }

    /// Send a frame of data to the recording
    pub fn send_frame(&mut self, image: &crate::ppu::PixelImage<egui_multiwin::egui::Color32>) {
        if let Some(_pipeline) = &mut self.record_pipeline {
            if let Some(source) = &mut self.record_source {
                let mut buf =
                    gstreamer::Buffer::with_size(image.width as usize * image.height as usize * 3)
                        .unwrap();
                image.to_gstreamer(&mut buf);
                source.do_timestamp();
                match source.push_buffer(buf) {
                    Ok(_a) => {}
                    Err(e) => {
                        println!("Error pushing video data: {:?}", e);
                    }
                }
            }
        }
    }

    /// Stop recording
    pub fn stop(&mut self) -> Result<(), gstreamer::FlowError> {
        if let Some(pipeline) = &mut self.record_pipeline {
            let _dot =
                gstreamer::debug_bin_to_dot_data(pipeline, gstreamer::DebugGraphDetails::all());
            //std::fs::write("./pipeline.dot", dot).expect("Unable to write pipeline file");

            if let Some(source) = &mut self.record_source {
                source.end_of_stream()?;
            }
            pipeline
                .set_state(gstreamer::State::Null)
                .expect("Unable to set the recording pipeline to the `Null` state");
        }
        self.record_pipeline = None;
        self.record_source = None;
        self.audio = None;
        Ok(())
    }
}
