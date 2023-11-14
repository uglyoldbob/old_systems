//! The streaming module contains gstreamer code that allows an emulator session to be streamed to other participants

use gstreamer::prelude::{
    Cast, ElementExt, ElementExtManual, GstBinExtManual, GstObjectExt, PadExt,
};

use crate::apu::AudioProducerWithRate;

/// The main struct for sending a video stream over an arbitrary stream
pub struct StreamingOut {
    /// The pipeline
    record_pipeline: Option<gstreamer::Pipeline>,
    /// The source for video data from the emulator
    record_source: Option<gstreamer_app::AppSrc>,
    /// The sink that goes out to a network device
    sink: Option<gstreamer_app::AppSink>,
    /// The audio source for the recording
    audio: Option<crate::apu::AudioProducerWithRate>,
}

impl StreamingOut {
    /// Create a recording object
    pub fn new() -> Self {
        Self {
            record_pipeline: None,
            record_source: None,
            sink: None,
            audio: None,
        }
    }

    /// Take an existing sink, if there is one
    pub fn take_sink(&mut self) -> Option<gstreamer_app::AppSink> {
        self.sink.take()
    }

    /// Takes the optional sound source
    pub fn get_sound(&mut self) -> Option<crate::apu::AudioProducerWithRate> {
        self.audio.take()
    }

    /// Returns true if recording
    pub fn is_recording(&self) -> bool {
        self.record_pipeline.is_some()
    }

    /// Start recording by setting up the necessary objects.
    pub fn start(&mut self, width: u16, height: u16, framerate: u8, cpu_frequency: f32) {
        if self.record_pipeline.is_none() {
            let version = gstreamer::version_string().as_str().to_string();
            println!("GStreamer version is {}", version);
            let vinfo = gstreamer_video::VideoInfo::builder(
                gstreamer_video::VideoFormat::Rgb,
                width as u32,
                height as u32,
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

            let sink = gstreamer_app::AppSink::builder()
                .name("network_sink")
                .build();

            audio_source.set_block(false);
            app_source.set_do_timestamp(true);
            app_source.set_is_live(true);
            audio_source.set_is_live(true);
            app_source.set_block(false);
            audio_source.set_do_timestamp(true);
            let vconv = gstreamer::ElementFactory::make("videoconvert")
                .name("vconvert")
                .build()
                .expect("Could not create source element.");
            let aencoder = gstreamer::ElementFactory::make("avenc_ac3")
                .name("aencode")
                .build()
                .expect("Could not create source element.");
            let vencoder = gstreamer::ElementFactory::make("openh264enc")
                .name("vencode")
                .build()
                .expect("Could not create source element.");
            let mux = gstreamer::ElementFactory::make("mpegtsmux")
                .name("mepgmux")
                .build()
                .expect("Could not create source element.");

            let aresample = gstreamer::ElementFactory::make("audioresample")
                .name("aresample")
                .build()
                .expect("Could not create source element.");

            let pipeline = gstreamer::Pipeline::with_name("streaming-pipeline");
            pipeline
                .add_many([
                    app_source.upcast_ref(),
                    audio_source.upcast_ref(),
                    &aencoder,
                    &vconv,
                    &aresample,
                    &vencoder,
                    &mux,
                    sink.upcast_ref(),
                ])
                .unwrap();
            gstreamer::Element::link_many([app_source.upcast_ref(), &vconv, &vencoder]).unwrap();
            gstreamer::Element::link_many([audio_source.upcast_ref(), &aresample, &aencoder])
                .unwrap();

            aencoder.link(&mux).unwrap();
            vencoder.link(&mux).unwrap();
            mux.link(&sink).unwrap();

            pipeline
                .set_state(gstreamer::State::Playing)
                .expect("Unable to set the pipeline to the `Playing` state");

            self.record_source = Some(app_source);
            self.record_pipeline = Some(pipeline);
            self.sink = Some(sink);

            self.audio = Some(AudioProducerWithRate::new_gstreamer(
                44100,
                cpu_frequency / 44100.0,
                audio_source,
            ));
        }
    }

    /// Send a chunk of video data to the pipeline
    pub fn send_video_buffer(&mut self, buffer: Vec<u8>) {
        if let Some(pipeline) = &mut self.record_pipeline {
            if let Some(source) = &mut self.record_source {
                let _dot =
                    gstreamer::debug_bin_to_dot_data(pipeline, gstreamer::DebugGraphDetails::all());
                //std::fs::write("./pipeline_stream_out.dot", dot).expect("Unable to write pipeline file");

                let mut buf = gstreamer::Buffer::with_size(buffer.len()).unwrap();
                let mut p = buf.make_mut().map_writable().unwrap();
                for (a, b) in buffer.iter().zip(p.iter_mut()) {
                    *b = *a;
                }
                drop(p);
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

    /// Send a chunk of audio data to the pipeline
    pub fn send_audio_buffer(&mut self, buffer: Vec<u8>) {
        if let Some(_pipeline) = &mut self.record_pipeline {
            todo!();
        }
    }

    /// Stop recording
    pub fn stop(&mut self) {
        if let Some(pipeline) = &mut self.record_pipeline {
            let _dot =
                gstreamer::debug_bin_to_dot_data(pipeline, gstreamer::DebugGraphDetails::all());
            //std::fs::write("./pipeline.dot", dot).expect("Unable to write pipeline file");

            if let Some(source) = &mut self.record_source {
                source.end_of_stream();
            }
            pipeline
                .set_state(gstreamer::State::Null)
                .expect("Unable to set the recording pipeline to the `Null` state");
        }
        self.record_pipeline = None;
        self.record_source = None;
        self.audio = None;
    }
}

/// The main struct for receiving a stream from StreamingOut
pub struct StreamingIn {
    /// The pipeline
    pipeline: Option<gstreamer::Pipeline>,
    /// The source for video data from the emulator
    stream_source: Option<gstreamer_app::AppSrc>,
    /// The audio sink for the pipeline
    audio: Option<gstreamer_app::AppSink>,
    /// The video sink for the pipeline
    video: Option<gstreamer_app::AppSink>,
}

impl StreamingIn {
    /// Create a recording object
    pub fn new() -> Self {
        Self {
            pipeline: None,
            stream_source: None,
            audio: None,
            video: None,
        }
    }

    /// Return a mutable reference to the source of video
    pub fn video_source(&mut self) -> &mut Option<gstreamer_app::AppSink> {
        &mut self.video
    }

    /// Returns true if streaming
    pub fn is_streaming(&self) -> bool {
        self.pipeline.is_some()
    }

    /// Start stream receiving by setting up the necessary objects.
    pub fn start(&mut self) {
        if self.pipeline.is_none() {
            let version = gstreamer::version_string().as_str().to_string();
            println!("GStreamer version is {}", version);

            let source = gstreamer_app::AppSrc::builder()
                .name("emulator_av_mpeg")
                .build();

            source.set_block(false);
            source.set_do_timestamp(true);
            source.set_is_live(true);

            let queue = gstreamer::ElementFactory::make("queue")
                .name("queue")
                .build()
                .expect("Could not create element.");

            let sconv = gstreamer::ElementFactory::make("tsparse")
                .name("tsparse")
                .build()
                .expect("Could not create element.");

            let demux = gstreamer::ElementFactory::make("tsdemux")
                .name("tsdemux")
                .build()
                .expect("Could not create element.");

            let vdecoder = gstreamer::ElementFactory::make("openh264dec")
                .name("vdecode")
                .build()
                .expect("Could not create source element.");

            let adecoder = gstreamer::ElementFactory::make("avdec_ac3")
                .name("adecode")
                .build()
                .expect("Could not create source element.");

            let asink = gstreamer_app::AppSink::builder().name("audio_sink").build();
            let vsink = gstreamer_app::AppSink::builder().name("video_sink").build();

            let pipeline = gstreamer::Pipeline::with_name("receiving-pipeline");
            pipeline
                .add_many([
                    &sconv,
                    &queue,
                    source.upcast_ref(),
                    &demux,
                    asink.upcast_ref(),
                    vsink.upcast_ref(),
                    &vdecoder,
                    &adecoder,
                ])
                .unwrap();
            gstreamer::Element::link_many([source.upcast_ref(), &queue, &sconv, &demux]).unwrap();
            adecoder.link(&asink).expect("Failed to link to asink");
            vdecoder.link(&vsink).expect("Failed to link to vsink");

            let video_sink_pad = vdecoder
                .static_pad("sink")
                .expect("could not get sink pad from vdecoder");
            let audio_sink_pad = adecoder
                .static_pad("sink")
                .expect("could not get sink pad from adecoder");
            demux.connect_pad_added(move |_src, src_pad| {
                let is_video = if src_pad.name().starts_with("video") {
                    true
                } else {
                    false
                };

                let is_audio = if src_pad.name().starts_with("audio") {
                    true
                } else {
                    false
                };

                let connect_demux = || -> Result<(), u8> {
                    src_pad
                        .link(&video_sink_pad)
                        .expect("failed to link tsdemux.video->h264parse.sink");
                    println!("linked tsdemux->h264parse");
                    Ok(())
                };

                let connect_demux2 = || -> Result<(), u8> {
                    src_pad
                        .link(&audio_sink_pad)
                        .expect("failed to link audio to audio decoder");
                    println!("linked tsdemux->h264parse");
                    Ok(())
                };

                if is_video {
                    match connect_demux() {
                        Ok(_) => println!("video connected"),
                        Err(e) => println!("could not connect video e:{}", e),
                    }
                }
                if is_audio {
                    match connect_demux2() {
                        Ok(_) => println!("audio connected"),
                        Err(e) => println!("could not connect audio e:{}", e),
                    }
                }
            });

            pipeline
                .set_state(gstreamer::State::Playing)
                .expect("Unable to set the pipeline to the `Playing` state");

            self.pipeline = Some(pipeline);
            self.video = Some(vsink);
            self.audio = Some(asink);
        }
    }

    /// Send data to the receiving end of the pipeline
    pub fn send_data(&mut self, buffer: Vec<u8>) {
        if let Some(pipeline) = &mut self.pipeline {
            if let Some(source) = &mut self.stream_source {
                println!("Received {} bytes of data from host", buffer.len());
                let mut buf = gstreamer::Buffer::with_size(buffer.len()).unwrap();
                let mut p = buf.make_mut().map_writable().unwrap();
                for (a, b) in buffer.iter().zip(p.iter_mut()) {
                    *b = *a;
                }
                drop(p);
                source.do_timestamp();
                match source.push_buffer(buf) {
                    Ok(_a) => {}
                    Err(e) => {
                        println!("Error pushing video data: {:?}", e);
                    }
                }
            }
            let dot =
                gstreamer::debug_bin_to_dot_data(pipeline, gstreamer::DebugGraphDetails::all());
            std::fs::write("./pipeline_stream_in.dot", dot).expect("Unable to write pipeline file");
        }
    }

    /// Stop recording
    pub fn stop(&mut self) {
        if let Some(pipeline) = &mut self.pipeline {
            let _dot =
                gstreamer::debug_bin_to_dot_data(pipeline, gstreamer::DebugGraphDetails::all());
            //std::fs::write("./pipeline.dot", dot).expect("Unable to write pipeline file");

            if let Some(source) = &mut self.stream_source {
                source.end_of_stream();
            }
            pipeline
                .set_state(gstreamer::State::Null)
                .expect("Unable to set the recording pipeline to the `Null` state");
        }
        self.pipeline = None;
    }
}
