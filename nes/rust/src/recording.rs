//! This is the module for recording related code

use gstreamer::prelude::{
    Cast, ElementExt, GstBinExtManual
};

/// The main struct for recording related activities
pub struct Recording {
    /// The pipeline for recording to disk
    record_pipeline: Option<gstreamer::Pipeline>,
    /// The source for video data fromm the emulator
    record_source: Option<gstreamer_app::AppSrc>,
}

impl Recording {
    /// Create a recording object
    pub fn new() -> Self {
        Self {
            record_pipeline: None,
            record_source: None,
        }
    }

    /// Start recording by setting up the necessary objects.
    pub fn start(&mut self, have_gstreamer: &Result<(), gstreamer::glib::Error>) {
        if self.record_pipeline.is_none() {
            if have_gstreamer.is_ok() {
                let version = gstreamer::version_string().as_str().to_string();
                println!("GStreamer version is {}", version);
                let vinfo = gstreamer_video::VideoInfo::builder(
                    gstreamer_video::VideoFormat::Rgb,
                    256,
                    240,
                )
                .build()
                .unwrap();
                let video_caps = vinfo.to_caps().unwrap();
                let app_source = gstreamer_app::AppSrc::builder()
                    .name("emulator_video")
                    .caps(&video_caps)
                    .format(gstreamer::Format::Time)
                    .build();
                app_source.set_block(true);
                let vsource = gstreamer::ElementFactory::make("videoparse")
                    .name("vparse")
                    .property_from_str("framerate", "60/1")
                    .property_from_str("width", "256")
                    .property_from_str("height", "240")
                    .property_from_str("format", "rgb")
                    .build()
                    .expect("Could not create source element.");
                let vconv = gstreamer::ElementFactory::make("videoconvert")
                    .name("vconvert")
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

                let sink = gstreamer::ElementFactory::make("filesink")
                    .name("sink")
                    .property_from_str("location", "./test.avi")
                    .build()
                    .expect("Could not create sink element");

                let pipeline = gstreamer::Pipeline::with_name("recording-pipeline");
                pipeline
                    .add_many([
                        app_source.upcast_ref(),
                        &vsource,
                        &vconv,
                        &vencoder,
                        &avimux,
                        &sink,
                    ])
                    .unwrap();
                gstreamer::Element::link_many([
                    app_source.upcast_ref(),
                    &vsource,
                    &vconv,
                    &vencoder,
                    &avimux,
                    &sink,
                ])
                .unwrap();

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
        if let Some(pipeline) = &mut self.record_pipeline {
            if let Some(source) = &mut self.record_source {
                let mut buf = gstreamer::Buffer::with_size(256 * 240 * 3).unwrap();
                image.to_gstreamer(256, 240, &mut buf);
                match source.push_buffer(buf) {
                    Ok(a) => {
                    }
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