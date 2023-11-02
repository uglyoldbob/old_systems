//! This is the module for network play related code.

use std::net::TcpStream;

use libp2p::{futures::StreamExt, Swarm};

/// The types of roles that can occur for the network object
pub enum Role {
    /// The host contains the actual emulator implementation. It sends video streams to all Player and received controller inputs.
    Host,
    /// A player instance receives a video feed from a host and sends controller data.
    Player,
}

/// The main networking struct for the emulator
pub struct Network {
    role: Option<Role>,
    tokio: tokio::runtime::Runtime,
    thread: tokio::task::JoinHandle<()>,
}

// This describes the behaviour of a libp2p swarm
#[derive(libp2p::swarm::NetworkBehaviour)]
struct SwarmBehavior {
    upnp: libp2p::upnp::tokio::Behaviour,
}

/// The object used internally to this module to manage network state
struct InternalNetwork {
    swarm: Swarm<SwarmBehavior>,
}

impl InternalNetwork {
    async fn do_the_thing(&mut self) -> Option<()> {
        self.swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse().ok()?).ok()?;
        loop {
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

    fn start(runtime: &mut tokio::runtime::Runtime) -> tokio::task::JoinHandle<()> {
        runtime.spawn(async {
            println!("Started async code");
            if let Some(mut i) = Self::try_new() {
                i.do_the_thing().await;
            }
        })
    }

    /// Create a new object
    fn try_new() -> Option<Self> {
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
                };
                beh
            })
            .ok()?
            .build();
        Some(Self {
            swarm,
        })
    }
}

impl Network {
    ///Create a new instance of network with the given role
    pub fn new() -> Self {
        let mut t = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        let t2 = InternalNetwork::start(&mut t);
        Self {
            role: None,
            tokio: t,
            thread: t2,
        }
    }
}
