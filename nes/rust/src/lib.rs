//! This contains the android code for the nes emulator
/// use `x build --arch arm64 --platform android`
/// or `x run --device ______`
///
mod apu;
mod cartridge;
mod controller;
mod cpu;
mod emulator_data;
mod genie;
mod motherboard;
mod ppu;
mod windows;

#[cfg(target_os = "android")]
static INTERNAL_DATA_PATH: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();

#[cfg(target_os = "android")]
pub fn set_application_data_path(p: &std::path::PathBuf) {
    INTERNAL_DATA_PATH.set(p.clone());
}

#[cfg(target_os = "android")]
pub fn get_application_data_path() -> std::path::PathBuf {
    INTERNAL_DATA_PATH.get().unwrap().to_owned()
}

use crate::cartridge::NesCartridge;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use emulator_data::NesEmulatorData;

use eframe::{egui, NativeOptions};

#[cfg(target_os = "android")]
use egui_winit::winit;

#[cfg(not(target_arch = "wasm32"))]
///Run an asynchronous object on a new thread. Maybe not the best way of accomplishing this, but it does work.
pub fn execute<F: std::future::Future<Output = ()> + Send + 'static>(f: F) {
    std::thread::spawn(move || futures::executor::block_on(f));
}
#[cfg(target_arch = "wasm32")]
///Run an asynchronous object on a new thread. Maybe not the best way of accomplishing this, but it does work.
pub fn execute<F: std::future::Future<Output = ()> + 'static>(f: F) {
    wasm_bindgen_futures::spawn_local(f);
}

#[cfg(not(target_os = "android"))]
struct BluetoothReadHalf(std::sync::Arc<tokio::sync::Mutex<bluetooth_rust::BluetoothStream>>);
#[cfg(not(target_os = "android"))]
struct BluetoothWriteHalf(std::sync::Arc<tokio::sync::Mutex<bluetooth_rust::BluetoothStream>>);

#[cfg(not(target_os = "android"))]
impl tokio::io::AsyncRead for BluetoothReadHalf {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let mut guard = self
            .0
            .try_lock()
            .expect("BluetoothReadHalf polled while write is in progress");
        std::pin::Pin::new(&mut *guard).poll_read(cx, buf)
    }
}

#[cfg(not(target_os = "android"))]
impl tokio::io::AsyncWrite for BluetoothWriteHalf {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        let mut guard = self
            .0
            .try_lock()
            .expect("BluetoothWriteHalf polled while read is in progress");
        std::pin::Pin::new(&mut *guard).poll_write(cx, buf)
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let mut guard = self
            .0
            .try_lock()
            .expect("BluetoothWriteHalf polled while read is in progress");
        std::pin::Pin::new(&mut *guard).poll_flush(cx)
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let mut guard = self
            .0
            .try_lock()
            .expect("BluetoothWriteHalf polled while read is in progress");
        std::pin::Pin::new(&mut *guard).poll_shutdown(cx)
    }
}

#[cfg(not(target_os = "android"))]
fn split_bluetooth(
    stream: bluetooth_rust::BluetoothStream,
) -> (BluetoothReadHalf, BluetoothWriteHalf) {
    let shared = std::sync::Arc::new(tokio::sync::Mutex::new(stream));
    (
        BluetoothReadHalf(shared.clone()),
        BluetoothWriteHalf(shared),
    )
}

#[cfg(not(target_os = "android"))]
struct BluetoothPacketReceiver {
    stream: BluetoothReadHalf,
    len0: Option<u8>,
    len1: Option<u8>,
    buf: Vec<u8>,
    buf_pos: usize,
}

#[cfg(not(target_os = "android"))]
impl BluetoothPacketReceiver {
    async fn receive_packet(&mut self) -> Option<Option<::controller::ControllerSend>> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        if self.len0.is_none() {
            self.len0 = Some(self.stream.read_u8().await.ok()?);
        }
        if self.len0.is_some() {
            if self.len1.is_none() {
                self.len1 = Some(self.stream.read_u8().await.ok()?);
            }
        }
        if let Some(a) = self.len0 {
            if let Some(b) = self.len1 {
                let len = u16::from_be_bytes([a, b]) as usize;
                if self.buf.len() < len {
                    self.buf.resize(len, 0);
                }
                while self.buf_pos < len {
                    let n = self
                        .stream
                        .read(&mut self.buf[self.buf_pos..len])
                        .await
                        .ok()?;
                    if n == 0 {
                        return Some(None); // EOF
                    }
                    self.buf_pos += n;
                }
                let packet = bincode::deserialize(&self.buf[..len]).ok()?;
                self.len0 = None;
                self.len1 = None;
                self.buf_pos = 0;
                return Some(Some(packet));
            }
        }
        None
    }
}

#[cfg(not(target_os = "android"))]
struct BluetoothControllerClient {
    streamr: BluetoothPacketReceiver,
    streamw: BluetoothWriteHalf,
    addr: [u8; 6],
    send: tokio::sync::mpsc::Sender<BluetoothControllerInfo>,
    mychan: (
        tokio::sync::mpsc::Sender<BluetoothControllerResponse>,
        tokio::sync::mpsc::Receiver<BluetoothControllerResponse>,
    ),
    my_player_number: Option<u8>,
}

#[cfg(not(target_os = "android"))]
impl BluetoothControllerClient {
    fn new(
        stream: bluetooth_rust::BluetoothStream,
        addr: [u8; 6],
        send: tokio::sync::mpsc::Sender<BluetoothControllerInfo>,
    ) -> Self {
        let stream = split_bluetooth(stream);
        let brecv = BluetoothPacketReceiver {
            stream: stream.0,
            len0: None,
            len1: None,
            buf: Vec::new(),
            buf_pos: 0,
        };
        Self {
            streamr: brecv,
            streamw: stream.1,
            addr,
            send,
            mychan: tokio::sync::mpsc::channel(10),
            my_player_number: None,
        }
    }

    async fn end(&mut self) {
        let _ = self
            .send
            .send(BluetoothControllerInfo {
                address: self.addr,
                response: self.mychan.0.clone(),
                message: BluetoothControllerInfoMessage::Disconnect(self.my_player_number),
            })
            .await;
    }

    async fn send_message(
        &mut self,
        msg: &::controller::ControllerReceive,
    ) -> Result<Result<(), std::io::Error>, std::io::Error> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let d = bincode::serialize(msg).map_err(|e| std::io::Error::other(e))?;
        tokio::time::timeout(
            std::time::Duration::from_secs(1),
            self.streamw.write_u16(d.len() as u16),
        )
        .await??;
        tokio::time::timeout(
            std::time::Duration::from_secs(1),
            self.streamw.write_all(&d),
        )
        .await??;
        tokio::time::timeout(std::time::Duration::from_secs(1), self.streamw.flush()).await??;
        Ok(Ok(()))
    }

    async fn handle_bluetooth_controller_client(&mut self) -> Result<(), std::io::Error> {
        let mut mychan = tokio::sync::mpsc::channel(10);

        self.send
            .send(BluetoothControllerInfo {
                address: self.addr,
                response: mychan.0.clone(),
                message: BluetoothControllerInfoMessage::Initialize,
            })
            .await
            .map_err(|e| std::io::Error::other(e))?;

        loop {
            tokio::select! {
                r = mychan.1.recv() => {
                    match r {
                        None => {
                            break;
                        }
                        Some(a) => {
                            match a {
                                BluetoothControllerResponse::SetPlayerNumber(i) => {
                                    self.my_player_number = Some(i);
                                }
                            }
                        }
                    }
                }
                a = tokio::time::timeout(std::time::Duration::from_secs(5), self.streamr.receive_packet()) => {
                    match a {
                        Err(_timeout) => {
                            break;
                        }
                        Ok(a) => {
                            match a {
                                None => {
                                    // Did not receive a full packet, continue
                                }
                                Some(a) => {
                                    match a {
                                        None => {
                                            break;
                                        }
                                        Some(p) => match p {
                                            ::controller::ControllerSend::GetPlayerNumber => {
                                                self.send_message(&::controller::ControllerReceive::PlayerNumber(self.my_player_number)).await??;
                                            }
                                            ::controller::ControllerSend::ButtonData(data) => {
                                                if let Some(pnum) = self.my_player_number {
                                                    self.send.send(BluetoothControllerInfo {
                                                        address: self.addr,
                                                        response: mychan.0.clone(),
                                                        message: BluetoothControllerInfoMessage::ButtonData(pnum, data),
                                                    })
                                                    .await
                                                    .map_err(|e| std::io::Error::other(e))?;
                                                }
                                                self.send_message(&::controller::ControllerReceive::AcknowledgeButtonData).await??;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

/// The responses to a bluetooth controller communication
pub enum BluetoothControllerResponse {
    /// Set the player number for the controller
    SetPlayerNumber(u8),
}

/// The types of messages that can be delivered from a bluetooth controller
pub enum BluetoothControllerInfoMessage {
    /// Initialize the bluetooth controller
    Initialize,
    /// disconnect the bluetooth controller
    Disconnect(Option<u8>),
    /// A dummy message from the bluetooth controller
    Dummy,
    /// The controller number, then the button data from the controller
    ButtonData(u8, u16),
}

/// Commands that be sent from a bluetooth thread
pub struct BluetoothControllerInfo {
    /// The bluetooth address of the controller
    pub address: [u8; 6],
    /// Response channel
    pub response: tokio::sync::mpsc::Sender<BluetoothControllerResponse>,
    /// The actual message
    pub message: BluetoothControllerInfoMessage,
}

#[cfg(not(target_os = "android"))]
async fn run_bluetooth(
    send: tokio::sync::mpsc::Sender<BluetoothControllerInfo>,
) -> Result<(), String> {
    let mut bab = bluetooth_rust::BluetoothAdapterBuilder::new();
    let s = tokio::sync::mpsc::channel(100);
    bab.with_sender(s.0);
    let ba = bab.build().await;
    match ba {
        Ok(ba) => {
            let settings = bluetooth_rust::BluetoothRfcommProfileSettings {
                uuid: "76ECEF8B-24D4-4F7C-9DE0-706864B6BC14".to_string(),
                name: Some("NES Controller Service".to_string()),
                service_uuid: Some("76ECEF8B-24D4-4F7C-9DE0-706864B6BC14".to_string()),
                channel: Some(23),
                psm: None,
                authenticate: Some(false),
                authorize: Some(false),
                auto_connect: Some(true),
                sdp_record: None,
                sdp_version: None,
                sdp_features: None,
            };
            let mut profile = ba.register_rfcomm_profile(settings).await?;
            log::info!("Registered bluetooth");
            loop {
                let c = profile.connectable().await?;
                match c.accept().await {
                    Ok(a) => {
                        log::info!("Got a bluetooth client: {:x?}", a.1);
                        let chan2 = send.clone();
                        tokio::spawn(async move {
                            let chan3 = chan2.clone();
                            let mut cl = BluetoothControllerClient::new(a.0, a.1, chan3);
                            if let Err(e) = cl.handle_bluetooth_controller_client().await {
                                log::error!("Error communicating with bluetooth client: {:?}", e);
                            }
                            log::info!("Ending bluetooth client {:x?}", a.1);
                            cl.end().await;
                        });
                    }
                    Err(e) => {
                        log::error!("Error accepting connection: {}", e);
                    }
                }
            }
        }
        Err(e) => log::error!("Failed to get bluetooth adapter: {}", e),
    }
    Ok(())
}

#[cfg(target_os = "android")]
fn hide_system_bars(app: &winit::platform::android::activity::AndroidApp) {
    use jni::objects::JObject;
    use jni::JavaVM;

    let vm = unsafe {
        jni::JavaVM::from_raw(app.vm_as_ptr() as *mut *const jni::sys::JNIInvokeInterface_)
    };
    vm.attach_current_thread(|env| {
        let context = unsafe {
            jni::objects::JObject::from_raw(env, app.activity_as_ptr() as *mut jni::sys::_jobject)
        };

        let activity =
            unsafe { JObject::from_raw(env, app.activity_as_ptr() as jni::sys::jobject) };

        // Activity.getWindow()
        let window = env
            .call_method(
                &activity,
                jni::jni_str!("getWindow"),
                jni::jni_sig!("()Landroid/view/Window;"),
                &[],
            )?
            .l()?;

        // Window.setDecorFitsSystemWindows(false)
        env.call_method(
            &window,
            jni::jni_str!("setDecorFitsSystemWindows"),
            jni::jni_sig!("(Z)V"),
            &[false.into()],
        )?;

        // Window.getInsetsController()
        let controller = env
            .call_method(
                &window,
                jni::jni_str!("getInsetsController"),
                jni::jni_sig!("()Landroid/view/WindowInsetsController;"),
                &[],
            )?
            .l()?;

        // WindowInsets.Type.systemBars()
        let system_bars = env
            .call_static_method(
                jni::jni_str!("android/view/WindowInsets$Type"),
                jni::jni_str!("systemBars"),
                jni::jni_sig!("()I"),
                &[],
            )?
            .i()?;

        // WindowInsetsController.hide(systemBars)
        env.call_method(
            &controller,
            jni::jni_str!("hide"),
            jni::jni_sig!("(I)V"),
            &[system_bars.into()],
        )?;

        // BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE = 2
        env.call_method(
            &controller,
            jni::jni_str!("setSystemBarsBehavior"),
            jni::jni_sig!("(I)V"),
            &[2i32.into()],
        )?;

        Ok::<(), jni::errors::Error>(())
    })
    .unwrap();
}

#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: winit::platform::android::activity::AndroidApp) {
    use common_emulator::audio::{AudioProducer, AudioProducerWithRate};
    use eframe::Renderer;

    std::env::set_var("RUST_BACKTRACE", "full");
    android_logger::init_once(
        android_logger::Config::default().with_max_level(log::LevelFilter::Error),
    );

    hide_system_bars(&app);

    let appc = app.clone();

    let internal_data_path = app.internal_data_path().unwrap();
    set_application_data_path(&internal_data_path);

    let options = NativeOptions {
        android_app: Some(app),
        renderer: Renderer::Wgpu,
        ..Default::default()
    };

    run(options)
}

pub fn run(mut options: eframe::NativeOptions) {
    use common_emulator::audio::{AudioProducer, AudioProducerWithRate};

    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        // Only print, don't propagate during cleanup
        log::error!("Panic (possibly during shutdown): {info}");
        default_hook(info);
    }));

    #[cfg(feature = "puffin")]
    puffin::set_scopes_on(true); // Remember to call this, or puffin will be disabled!

    //TODO only disable vsync when required
    options.vsync = false;

    #[cfg(not(target_os = "android"))]
    let chan = {
        let trt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to start async runtime");
        let chan = tokio::sync::mpsc::channel(100);
        trt.spawn(async {
            if let Err(e) = run_bluetooth(chan.0).await {
                log::error!("Error running bluetooth: {:?}", e);
            }
        });
        chan.1
    };

    let mut nes_data = NesEmulatorData::new();
    #[cfg(not(target_os = "android"))]
    nes_data.register_bluetooth(chan);

    let host = cpal::default_host();
    let device = host.default_output_device();
    let mut sound_rate = 0;
    let mut sound_producer = None;
    let sound_stream = if let Some(d) = &device {
        let ranges = d.supported_output_configs();
        if let Ok(r) = ranges {
            let mut configs: Vec<cpal::SupportedStreamConfigRange> = r.collect();
            for c in &configs {
                log::info!(
                    "Audio: {:?} {:?}-{:?}",
                    c.sample_format(),
                    c.min_sample_rate(),
                    c.max_sample_rate()
                );
            }
            configs.retain(|config| {
                config.min_sample_rate() <= 44_100 && 44_100 <= config.max_sample_rate()
            });
            configs.sort_by(|a, b| {
                let format_index = |sf| match sf {
                    cpal::SampleFormat::F32 => 0,
                    cpal::SampleFormat::U32 => 1,
                    cpal::SampleFormat::U16 => 2,
                    cpal::SampleFormat::U8 => 3,
                    cpal::SampleFormat::I8
                    | cpal::SampleFormat::I16
                    | cpal::SampleFormat::I32
                    | cpal::SampleFormat::I64
                    | cpal::SampleFormat::U64
                    | cpal::SampleFormat::F64 => 10,
                    _ => 10,
                };

                format_index(a.sample_format())
                    .cmp(&format_index(b.sample_format()))
                    .then_with(|| b.max_sample_rate().cmp(&a.max_sample_rate()))
            });

            let supportedconfig = configs[0].clone().with_sample_rate(44100);
            let format = supportedconfig.sample_format();
            log::info!("output format is {:?}", format);
            let mut config = supportedconfig.config();

            let mut num_samples = (config.sample_rate as f32 * 0.10) as usize;
            let sbs = supportedconfig.buffer_size();
            let num_samples_buffer = if let cpal::SupportedBufferSize::Range { min, max } = sbs {
                if num_samples > *max as usize {
                    num_samples = *max as usize;
                    cpal::BufferSize::Fixed(*max as cpal::FrameCount)
                } else if num_samples < *min as usize {
                    num_samples = *min as usize;
                    cpal::BufferSize::Fixed(*min as cpal::FrameCount)
                } else {
                    cpal::BufferSize::Fixed(num_samples as cpal::FrameCount)
                }
            } else {
                //TODO maybe do somethind else when buffer size is unknown
                cpal::BufferSize::Fixed(num_samples as cpal::FrameCount)
            };
            config.buffer_size = num_samples_buffer;
            config.channels = 2;
            log::info!("SBS IS {:?}", sbs);

            log::info!("audio config is {:?}", config);

            log::info!(
                "Audio buffer size is {} elements, sample rate is {}",
                num_samples,
                config.sample_rate
            );

            let (mut stream, user_audio) = match format {
                cpal::SampleFormat::U8 => {
                    let rb = ringbuf::HeapRb::new(num_samples * 4);
                    let (producer, mut consumer) = rb.split();

                    let user_audio =
                        AudioProducerWithRate::new(AudioProducer::U8(producer), num_samples * 2);

                    let stream = d
                        .build_output_stream(
                            &config,
                            move |data: &mut [u8], _cb: &cpal::OutputCallbackInfo| {
                                let mut index = 0;
                                while index < data.len() {
                                    let c = consumer.pop_slice(&mut data[index..]);
                                    if c == 0 {
                                        break;
                                    }
                                    index += c;
                                }
                            },
                            move |_err| {},
                            None,
                        )
                        .ok();
                    (stream, user_audio)
                }
                cpal::SampleFormat::U16 => {
                    let rb = ringbuf::HeapRb::new(num_samples * 4);
                    let (producer, mut consumer) = rb.split();

                    let user_audio =
                        AudioProducerWithRate::new(AudioProducer::U16(producer), num_samples * 2);

                    let stream = d
                        .build_output_stream(
                            &config,
                            move |data: &mut [u16], _cb: &cpal::OutputCallbackInfo| {
                                let mut index = 0;
                                while index < data.len() {
                                    let c = consumer.pop_slice(&mut data[index..]);
                                    if c == 0 {
                                        break;
                                    }
                                    index += c;
                                }
                            },
                            move |_err| {},
                            None,
                        )
                        .ok();
                    (stream, user_audio)
                }
                cpal::SampleFormat::U32 => {
                    let rb = ringbuf::HeapRb::new(num_samples * 4);
                    let (producer, mut consumer) = rb.split();

                    let user_audio =
                        AudioProducerWithRate::new(AudioProducer::U32(producer), num_samples * 2);

                    let stream = d
                        .build_output_stream(
                            &config,
                            move |data: &mut [u32], _cb: &cpal::OutputCallbackInfo| {
                                let mut index = 0;
                                while index < data.len() {
                                    let c = consumer.pop_slice(&mut data[index..]);
                                    if c == 0 {
                                        break;
                                    }
                                    index += c;
                                }
                            },
                            move |_err| {},
                            None,
                        )
                        .ok();
                    (stream, user_audio)
                }
                cpal::SampleFormat::F32 => {
                    let rb = ringbuf::HeapRb::new(num_samples * 4);
                    let (producer, mut consumer) = rb.split();

                    let user_audio =
                        AudioProducerWithRate::new(AudioProducer::F32(producer), num_samples * 2);

                    let stream = d
                        .build_output_stream(
                            &config,
                            move |data: &mut [f32], _cb: &cpal::OutputCallbackInfo| {
                                let mut index = 0;
                                while index < data.len() {
                                    let c = consumer.pop_slice(&mut data[index..]);
                                    if c == 0 {
                                        break;
                                    }
                                    index += c;
                                }
                            },
                            move |_err| {},
                            None,
                        )
                        .ok();
                    (stream, user_audio)
                }
                _ => todo!(),
            };

            if let Some(s) = &mut stream {
                s.play().unwrap();
                sound_rate = config.sample_rate;
                nes_data.local.set_sound_rate(config.sample_rate);
                sound_producer = Some(user_audio);
            }
            stream
        } else {
            None
        }
    } else {
        None
    };

    let wdir = std::env::current_dir().unwrap();
    log::info!("Current dir is {}", wdir.display());
    nes_data.mb.set_controller(
        0,
        nes_data.local.configuration.controller_type[0].make_controller(),
    );
    nes_data.mb.set_controller(
        1,
        nes_data.local.configuration.controller_type[1].make_controller(),
    );
    nes_data.mb.set_controller(
        2,
        nes_data.local.configuration.controller_type[2].make_controller(),
    );
    nes_data.mb.set_controller(
        3,
        nes_data.local.configuration.controller_type[3].make_controller(),
    );

    if nes_data.local.configuration.sticky_rom {
        if let Some(c) = nes_data.local.configuration.start_rom() {
            if let Ok(nc) = NesCartridge::load_cartridge(c.to_string(), &nes_data.local.save_path())
            {
                log::info!("Loaded sticky rom {c}");
                nes_data.insert_cartridge(nc.0);
                if let Some(save) = nc.1 {
                    nes_data.deserialize(save);
                }
            }
        }
    }
            }
        }
    }

    eframe::run_native(
        "Zesty NES Emulator",
        options,
        Box::new(move |_cc| {
            Ok(Box::new(crate::windows::main::MainNesWindow::new(
                nes_data,
                sound_rate,
                sound_producer,
                sound_stream,
            )))
        }),
    )
    .expect("Failed to run application");
}
