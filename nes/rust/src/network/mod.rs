//! This is the module for network play related code.

mod emulator;

use std::collections::HashSet;

use futures::FutureExt;
use libp2p::{futures::StreamExt, Multiaddr, Swarm};

#[derive(Debug)]
pub enum MessageToNetwork {
    ControllerData(u8, crate::controller::ButtonCombination),
    StartServer,
}

#[derive(Debug)]
pub enum MessageFromNetwork {
    EmulatorVideoStream(Vec<u8>),
    NewAddress(Multiaddr),
    ExpiredAddress(Multiaddr),
}

/// The main networking struct for the emulator
pub struct Network {
    tokio: tokio::runtime::Runtime,
    thread: tokio::task::JoinHandle<()>,
    sender: async_channel::Sender<MessageToNetwork>,
    recvr: async_channel::Receiver<MessageFromNetwork>,
    addresses: HashSet<Multiaddr>,
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
    sender: async_channel::Sender<MessageFromNetwork>,
    recvr: async_channel::Receiver<MessageToNetwork>,
    addresses: HashSet<Multiaddr>,
    proxy: egui_multiwin::winit::event_loop::EventLoopProxy<crate::event::Event>,
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
                                        MessageToNetwork::ControllerData(i, buttons) => {}
                                        MessageToNetwork::StartServer => {
                                            self.swarm
                                                .listen_on("/ip4/0.0.0.0/tcp/0".parse().ok()?)
                                                .ok()?;
                                        }
            }
                                }
                            },
                            ev = f1 => {
                                match ev {
                                    libp2p::swarm::SwarmEvent::NewListenAddr { address, .. } => {
                                        println!("Listening on {address:?}");
                                        self.sender
                                            .send(MessageFromNetwork::NewAddress(address.clone()))
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
                                            .send(MessageFromNetwork::NewAddress(addr.clone()))
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
                                        break;
                                    }
                                    libp2p::swarm::SwarmEvent::Behaviour(SwarmBehaviorEvent::Upnp(
                                        libp2p::upnp::Event::NonRoutableGateway,
                                    )) => {
                                        println!("Gateway is not exposed directly to the public Internet, i.e. it itself has a private IP address.");
                                        break;
                                    }
                                    libp2p::swarm::SwarmEvent::Behaviour(SwarmBehaviorEvent::Upnp(
                                        libp2p::upnp::Event::ExpiredExternalAddr(addr),
                                    )) => {
                                        println!("Expired address: {}", addr);
                                        self.sender
                                            .send(MessageFromNetwork::ExpiredAddress(addr.clone()))
                                            .await;
                                        self.proxy.send_event(crate::event::Event::new_general(
                                            crate::event::EventType::CheckNetwork,
                                        ));
                                        self.addresses.remove(&addr);
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
        s: async_channel::Sender<MessageFromNetwork>,
        r: async_channel::Receiver<MessageToNetwork>,
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
        s: async_channel::Sender<MessageFromNetwork>,
        r: async_channel::Receiver<MessageToNetwork>,
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
        }
    }

    pub fn get_addresses(&self) -> &HashSet<Multiaddr> {
        &self.addresses
    }

    pub fn process_messages(&mut self) {
        while let Ok(m) = self.recvr.try_recv() {
            match m {
                MessageFromNetwork::EmulatorVideoStream(_) => {}
                MessageFromNetwork::NewAddress(a) => {
                    self.addresses.insert(a);
                }
                MessageFromNetwork::ExpiredAddress(a) => {
                    self.addresses.remove(&a);
                }
            }
        }
    }

    pub fn start_server(&mut self) {
        println!(
            "Start server: {:?}",
            self.sender.send_blocking(MessageToNetwork::StartServer)
        );
    }

    pub fn send_controller_data(&mut self, i: u8, data: crate::controller::ButtonCombination) {
        self.sender
            .send_blocking(MessageToNetwork::ControllerData(i, data));
    }
}
