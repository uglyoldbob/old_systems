//! This is the module for network play related code.

mod emulator;

use libp2p::{futures::StreamExt, Swarm};

#[derive(Debug)]
pub enum MessageToNetwork {
    ControllerData(u8, crate::controller::ButtonCombination),
}

#[derive(Debug)]
pub enum MessageFromNetwork {
    EmulatorVideoStream(Vec<u8>),
}

/// The main networking struct for the emulator
pub struct Network {
    tokio: tokio::runtime::Runtime,
    thread: tokio::task::JoinHandle<()>,
    sender: std::sync::mpsc::Sender<MessageToNetwork>,
    recvr: std::sync::mpsc::Receiver<MessageFromNetwork>,
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
    sender: std::sync::mpsc::Sender<MessageFromNetwork>,
    recvr: std::sync::mpsc::Receiver<MessageToNetwork>,
}

impl InternalNetwork {
    async fn do_the_thing(&mut self) -> Option<()> {
        self.swarm
            .listen_on("/ip4/0.0.0.0/tcp/0".parse().ok()?)
            .ok()?;
        loop {
            while let Ok(a) = self.recvr.try_recv() {
                match a {
                    MessageToNetwork::ControllerData(i, buttons) => {}
                }
            }
            match self.swarm.select_next_some().await {
                libp2p::swarm::SwarmEvent::NewListenAddr { address, .. } => {
                    println!("Listening on {address:?}")
                }
                libp2p::swarm::SwarmEvent::Behaviour(SwarmBehaviorEvent::Upnp(
                    libp2p::upnp::Event::NewExternalAddr(addr),
                )) => {
                    println!("New external address: {addr}");
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
                }
                _ => {}
            }
        }
        Some(())
    }

    fn start(
        runtime: &mut tokio::runtime::Runtime,
        s: std::sync::mpsc::Sender<MessageFromNetwork>,
        r: std::sync::mpsc::Receiver<MessageToNetwork>,
    ) -> tokio::task::JoinHandle<()> {
        runtime.spawn(async {
            println!("Started async code");
            if let Some(mut i) = Self::try_new(s, r) {
                i.do_the_thing().await;
            }
        })
    }

    /// Create a new object
    fn try_new(
        s: std::sync::mpsc::Sender<MessageFromNetwork>,
        r: std::sync::mpsc::Receiver<MessageToNetwork>,
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
        })
    }
}

impl Network {
    ///Create a new instance of network with the given role
    pub fn new() -> Self {
        let (s1, r1) = std::sync::mpsc::channel();
        let (s2, r2) = std::sync::mpsc::channel();
        let mut t = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        let t2 = InternalNetwork::start(&mut t, s2, r1);
        Self {
            tokio: t,
            thread: t2,
            sender: s1,
            recvr: r2,
        }
    }

    pub fn send_controller_data(&mut self, i: u8, data: crate::controller::ButtonCombination) {
        self.sender.send(MessageToNetwork::ControllerData(i, data));
    }
}
