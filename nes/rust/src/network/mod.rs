//! This is the module for network play related code.

mod emulator;

use std::collections::HashSet;

use futures::FutureExt;
use libp2p::core::transport::ListenerId;
use libp2p::{futures::StreamExt, Multiaddr, Swarm};

pub enum NodeRole {
    DedicatedHost,
    PlayerHost,
    Player,
    Observer,
}

#[derive(Debug)]
pub enum MessageToNetworkThread {
    ControllerData(u8, crate::controller::ButtonCombination),
    StartServer,
    StopServer,
    Connect(String),
    Test,
}

#[derive(Debug)]
pub enum MessageFromNetworkThread {
    EmulatorVideoStream(Vec<u8>),
    NewAddress(Multiaddr),
    ExpiredAddress(Multiaddr),
    ServerStatus(bool),
    Test,
}

/// The main networking struct for the emulator
pub struct Network {
    tokio: tokio::runtime::Runtime,
    thread: tokio::task::JoinHandle<()>,
    sender: async_channel::Sender<MessageToNetworkThread>,
    recvr: async_channel::Receiver<MessageFromNetworkThread>,
    addresses: HashSet<Multiaddr>,
    server_running: bool,
}

// This describes the behaviour of a libp2p swarm
#[derive(libp2p::swarm::NetworkBehaviour)]
struct SwarmBehavior {
    upnp: libp2p::upnp::tokio::Behaviour,
    emulator: emulator::Behavior,
}

/// The object used internally to this module to manage network state
struct InternalNetwork {
    swarm: Swarm<SwarmBehavior>,
    sender: async_channel::Sender<MessageFromNetworkThread>,
    recvr: async_channel::Receiver<MessageToNetworkThread>,
    addresses: HashSet<Multiaddr>,
    proxy: egui_multiwin::winit::event_loop::EventLoopProxy<crate::event::Event>,
    listener: Option<ListenerId>,
}

impl InternalNetwork {
    async fn do_the_thing(&mut self) -> Option<()> {
        loop {
            let f1 = self.swarm.select_next_some().fuse();
            futures::pin_mut!(f1);
            let f2 = self.recvr.recv().fuse();
            futures::pin_mut!(f2);
            futures::select! {
                            r = f2 => {
                                if let Ok(m) = r {
                                    match m {
                                        MessageToNetworkThread::Test => {
                                            let behavior = self.swarm.behaviour_mut();
                                            behavior.emulator.send_message();
                                        }
                                        MessageToNetworkThread::Connect(cs) => {
                                            match cs.parse::<Multiaddr>() {
                                                Ok(addr) => {
                                                    println!("Attempt to connect to {} {:?}", cs, self.swarm.dial(addr));
                                                }
                                                Err(e) => {
                                                    println!("Error parsing multiaddr {:?}", e);
                                                }
                                            }
                                        }
                                        MessageToNetworkThread::ControllerData(i, buttons) => {
                                            let behavior = self.swarm.behaviour_mut();
                                            behavior.emulator.send_message();
                                        }
                                        MessageToNetworkThread::StopServer => {
                                            if let Some(list) = &mut self.listener {
                                                self.swarm.remove_listener(*list);
                                            }
                                            self.listener = None;
                                            self.addresses.clear();
                                            self.sender.send(MessageFromNetworkThread::ServerStatus(false)).await;
                                            self.proxy.send_event(crate::event::Event::new_general(
                                                crate::event::EventType::CheckNetwork,
                                            ));
                                        }
                                        MessageToNetworkThread::StartServer => {
                                            if self.listener.is_none() {
                                                let listenres = self.swarm
                                                .listen_on("/ip4/0.0.0.0/tcp/0".parse().ok()?);
                                                if let Ok(lis) = listenres {
                                                    self.listener = Some(lis);
                                                }
                                                println!("Server start result is {:?}", listenres);
                                                let s = self.listener.is_some();
                                                self.sender.send(MessageFromNetworkThread::ServerStatus(s)).await;
                                                self.proxy.send_event(crate::event::Event::new_general(
                                                    crate::event::EventType::CheckNetwork,
                                                ));
                                            }
                                        }
            }
                                }
                            },
                            ev = f1 => {
                                match ev {
                                    libp2p::swarm::SwarmEvent::NewListenAddr { address, .. } => {
                                        println!("Listening on {address:?}");
                                        self.sender
                                            .send(MessageFromNetworkThread::NewAddress(address.clone()))
                                            .await;
                                        self.proxy.send_event(crate::event::Event::new_general(
                                            crate::event::EventType::CheckNetwork,
                                        ));
                                        self.addresses.insert(address);
                                    }
                                    libp2p::swarm::SwarmEvent::Behaviour(SwarmBehaviorEvent::Upnp(
                                        libp2p::upnp::Event::NewExternalAddr(addr),
                                    )) => {
                                        println!("New external address: {addr}");
                                        self.sender
                                            .send(MessageFromNetworkThread::NewAddress(addr.clone()))
                                            .await;
                                        self.proxy.send_event(crate::event::Event::new_general(
                                            crate::event::EventType::CheckNetwork,
                                        ));
                                        self.addresses.insert(addr);
                                    }
                                    libp2p::swarm::SwarmEvent::Behaviour(SwarmBehaviorEvent::Upnp(
                                        libp2p::upnp::Event::GatewayNotFound,
                                    )) => {
                                        println!("Gateway does not support UPnP");
                                    }
                                    libp2p::swarm::SwarmEvent::Behaviour(SwarmBehaviorEvent::Upnp(
                                        libp2p::upnp::Event::NonRoutableGateway,
                                    )) => {
                                        println!("Gateway is not exposed directly to the public Internet, i.e. it itself has a private IP address.");
                                    }
                                    libp2p::swarm::SwarmEvent::Behaviour(SwarmBehaviorEvent::Upnp(
                                        libp2p::upnp::Event::ExpiredExternalAddr(addr),
                                    )) => {
                                        println!("Expired address: {}", addr);
                                        self.sender
                                            .send(MessageFromNetworkThread::ExpiredAddress(addr.clone()))
                                            .await;
                                        self.proxy.send_event(crate::event::Event::new_general(
                                            crate::event::EventType::CheckNetwork,
                                        ));
                                        self.addresses.remove(&addr);
                                    }
                                    libp2p::swarm::SwarmEvent::Behaviour(SwarmBehaviorEvent::Emulator(e)) => {
                                        match e {
                                            emulator::Event::Message(_, _, _) => todo!(),
                                            emulator::Event::TestEvent => {
                                                self.sender.send(MessageFromNetworkThread::Test).await;
                                            }
                                            emulator::Event::UnsupportedPeer(_) => todo!(),
                                        }
                                    }
                                    _ => {}
                                }
                            },
                        }
        }
        Some(())
    }

    fn start(
        runtime: &mut tokio::runtime::Runtime,
        s: async_channel::Sender<MessageFromNetworkThread>,
        r: async_channel::Receiver<MessageToNetworkThread>,
        proxy: egui_multiwin::winit::event_loop::EventLoopProxy<crate::event::Event>,
    ) -> tokio::task::JoinHandle<()> {
        runtime.spawn(async {
            println!("Started async code");
            if let Some(mut i) = Self::try_new(s, r, proxy) {
                i.do_the_thing().await;
            }
        })
    }

    /// Create a new object
    fn try_new(
        s: async_channel::Sender<MessageFromNetworkThread>,
        r: async_channel::Receiver<MessageToNetworkThread>,
        proxy: egui_multiwin::winit::event_loop::EventLoopProxy<crate::event::Event>,
    ) -> Option<Self> {
        let swarm = libp2p::SwarmBuilder::with_new_identity()
            .with_tokio()
            .with_tcp(
                Default::default(),
                libp2p::noise::Config::new,
                libp2p::yamux::Config::default,
            )
            .ok()?
            .with_behaviour(|_key| {
                let beh = SwarmBehavior {
                    upnp: libp2p::upnp::tokio::Behaviour::default(),
                    emulator: emulator::Behavior::new(),
                };
                beh
            })
            .ok()?
            .build();
        Some(Self {
            swarm,
            recvr: r,
            sender: s,
            addresses: HashSet::new(),
            proxy,
            listener: None,
        })
    }
}

impl Network {
    ///Create a new instance of network with the given role
    pub fn new(
        proxy: egui_multiwin::winit::event_loop::EventLoopProxy<crate::event::Event>,
    ) -> Self {
        let (s1, r1) = async_channel::bounded(10);
        let (s2, r2) = async_channel::bounded(10);
        let mut t = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        let t2 = InternalNetwork::start(&mut t, s2, r1, proxy);
        Self {
            tokio: t,
            thread: t2,
            sender: s1,
            recvr: r2,
            addresses: HashSet::new(),
            server_running: false,
        }
    }

    pub fn get_addresses(&self) -> &HashSet<Multiaddr> {
        &self.addresses
    }

    pub fn process_messages(&mut self) {
        while let Ok(m) = self.recvr.try_recv() {
            match m {
                MessageFromNetworkThread::Test => {
                    println!("Received test message from network");
                }
                MessageFromNetworkThread::EmulatorVideoStream(_) => {}
                MessageFromNetworkThread::NewAddress(a) => {
                    self.addresses.insert(a);
                }
                MessageFromNetworkThread::ExpiredAddress(a) => {
                    self.addresses.remove(&a);
                }
                MessageFromNetworkThread::ServerStatus(s) => {
                    self.server_running = s;
                    if !s {
                        self.addresses.clear();
                    }
                }
            }
        }
    }

    pub fn is_server_running(&self) -> bool {
        self.server_running
    }

    pub fn start_server(&mut self) {
        self.sender
            .send_blocking(MessageToNetworkThread::StartServer);
    }

    pub fn stop_server(&mut self) {
        self.sender
            .send_blocking(MessageToNetworkThread::StopServer);
    }

    pub fn try_connect(&mut self, cs: &String) {
        self.sender
            .send_blocking(MessageToNetworkThread::Connect(cs.to_owned()));
    }

    pub fn send_controller_data(&mut self, i: u8, data: crate::controller::ButtonCombination) {
        self.sender
            .send_blocking(MessageToNetworkThread::ControllerData(i, data));
    }
}
