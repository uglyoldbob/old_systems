//! This is the module for network play related code.

mod emulator;

use std::collections::HashSet;

use futures::FutureExt;
use libp2p::core::transport::ListenerId;
use libp2p::{futures::StreamExt, Multiaddr, Swarm};

use crate::controller::ButtonCombination;

#[derive(PartialEq, Copy, Clone, Debug, serde::Deserialize, serde::Serialize)]
pub enum NodeRole {
    DedicatedHost,
    PlayerHost,
    Player,
    Observer,
    Unknown,
}

#[derive(Debug)]
pub enum MessageToNetworkThread {
    ControllerData(u8, crate::controller::ButtonCombination),
    StartServer,
    StopServer,
    Connect(String),
    RequestObserverStatus,
    SetUserRole(libp2p::PeerId, NodeRole),
    RequestController(Option<u8>),
    SetController(libp2p::PeerId, Option<u8>),
}

#[derive(Debug)]
pub enum MessageFromNetworkThread {
    EmulatorVideoStream(Vec<u8>),
    ControllerData(u8, ButtonCombination),
    NewAddress(Multiaddr),
    ExpiredAddress(Multiaddr),
    ServerStatus(bool),
    ConnectedToHost,
    NewRole(NodeRole),
    RequestRole(libp2p::PeerId, NodeRole),
    RequestController(libp2p::PeerId, Option<u8>),
    SetController(Option<u8>),
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
                                        MessageToNetworkThread::SetController(p, c) => {
                                            let behavior = self.swarm.behaviour_mut();
                                            behavior.emulator.set_controller(p, c);
                                        }
                                        MessageToNetworkThread::RequestController(c) => {
                                            let myid = *self.swarm.local_peer_id();
                                            let behavior = self.swarm.behaviour_mut();
                                            behavior.emulator.request_controller(myid, c);
                                        }
                                        MessageToNetworkThread::SetUserRole(p, r) => {
                                            let behavior = self.swarm.behaviour_mut();
                                            behavior.emulator.set_user_role(p, r);
                                        }
                                        MessageToNetworkThread::RequestObserverStatus => {
                                            let myid = *self.swarm.local_peer_id();
                                            let behavior = self.swarm.behaviour_mut();
                                            behavior.emulator.request_observer_status(myid);
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
                                            behavior.emulator.send_controller_data(i, buttons);
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
                                            emulator::MessageToSwarm::ConnectedToHost => {
                                                self.sender
                                                    .send(MessageFromNetworkThread::ConnectedToHost)
                                                    .await;
                                                self.proxy.send_event(crate::event::Event::new_general(
                                                    crate::event::EventType::CheckNetwork,
                                                ));
                                            }
                                            emulator::MessageToSwarm::RequestController(i, c) => {
                                                self.sender
                                                    .send(MessageFromNetworkThread::RequestController(i, c))
                                                    .await;
                                                self.proxy.send_event(crate::event::Event::new_general(
                                                    crate::event::EventType::CheckNetwork,
                                                ));
                                            }
                                            emulator::MessageToSwarm::SetController(c) => {
                                                self.sender
                                                    .send(MessageFromNetworkThread::SetController(c))
                                                    .await;
                                                self.proxy.send_event(crate::event::Event::new_general(
                                                    crate::event::EventType::CheckNetwork,
                                                ));
                                            }
                                            emulator::MessageToSwarm::SetRole(r) => {
                                                self.sender
                                                    .send(MessageFromNetworkThread::NewRole(r))
                                                    .await;
                                                self.proxy.send_event(crate::event::Event::new_general(
                                                    crate::event::EventType::CheckNetwork,
                                                ));
                                            }
                                            emulator::MessageToSwarm::RequestRole(p, r) => {
                                                self.sender
                                                    .send(MessageFromNetworkThread::RequestRole(p, r))
                                                    .await;
                                                self.proxy.send_event(crate::event::Event::new_general(
                                                    crate::event::EventType::CheckNetwork,
                                                ));
                                            }
                                            emulator::MessageToSwarm::ControllerData(i, d) => {
                                                self.sender
                                                    .send(MessageFromNetworkThread::ControllerData(i, d))
                                                    .await;
                                                self.proxy.send_event(crate::event::Event::new_general(
                                                    crate::event::EventType::CheckNetwork,
                                                ));
                                            }
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

/// The main networking struct for the emulator
pub struct Network {
    tokio: tokio::runtime::Runtime,
    thread: tokio::task::JoinHandle<()>,
    sender: async_channel::Sender<MessageToNetworkThread>,
    recvr: async_channel::Receiver<MessageFromNetworkThread>,
    addresses: HashSet<Multiaddr>,
    server_running: bool,
    role: NodeRole,
    buttons: [Option<ButtonCombination>; 4],
    my_controller: Option<u8>,
    controller_holder: [Option<libp2p::PeerId>; 4],
    connected: bool,
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
            role: NodeRole::Unknown,
            buttons: [None; 4],
            my_controller: None,
            controller_holder: [None; 4],
            connected: false,
        }
    }

    pub fn get_addresses(&self) -> &HashSet<Multiaddr> {
        &self.addresses
    }

    pub fn get_button_data_ref(&mut self, i: u8) -> Option<ButtonCombination> {
        self.buttons[i as usize].clone()
    }

    pub fn process_messages(&mut self) {
        while let Ok(m) = self.recvr.try_recv() {
            match m {
                MessageFromNetworkThread::ConnectedToHost => {
                    self.connected = true;
                    let s = self
                        .sender
                        .send_blocking(MessageToNetworkThread::RequestObserverStatus);
                    println!("Request observer {:?}", s);
                }
                MessageFromNetworkThread::RequestController(p, c) => {
                    if let Some(c) = c {
                        if self.controller_holder[c as usize].is_none() {
                            self.controller_holder[c as usize] = Some(p);
                            self.sender
                                .send_blocking(MessageToNetworkThread::SetController(p, Some(c)));
                        }
                    } else {
                        for i in 0..4 {
                            if let Some(peer) = self.controller_holder[i] {
                                if peer == p {
                                    self.controller_holder[i] = None;
                                    self.sender.send_blocking(
                                        MessageToNetworkThread::SetController(p, None),
                                    );
                                    break;
                                }
                            }
                        }
                    }
                }
                MessageFromNetworkThread::SetController(c) => {
                    if c.is_some() && self.role == NodeRole::Observer {
                        self.role = NodeRole::Player;
                    } else if c.is_none() && self.role == NodeRole::Player {
                        self.role = NodeRole::Observer;
                    }
                    self.my_controller = c;
                }
                MessageFromNetworkThread::RequestRole(p, r) => match self.role {
                    NodeRole::DedicatedHost | NodeRole::PlayerHost => match r {
                        NodeRole::Player | NodeRole::Observer => {
                            let s = self
                                .sender
                                .send_blocking(MessageToNetworkThread::SetUserRole(p, r));
                            println!("Setting user role message {:?}", s);
                        }
                        _ => {}
                    },
                    _ => {}
                },
                MessageFromNetworkThread::NewRole(r) => {
                    println!("Role is now {:?}", r);
                    self.role = r;
                }
                MessageFromNetworkThread::ControllerData(i, d) => {
                    self.buttons[i as usize] = Some(d);
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
                    if s {
                        self.role = NodeRole::PlayerHost;
                    } else {
                        self.role = NodeRole::Unknown;
                    }
                    if !s {
                        self.addresses.clear();
                    }
                }
            }
        }
    }

    pub fn role(&self) -> NodeRole {
        self.role
    }

    pub fn get_controller_id(&self) -> Option<u8> {
        self.my_controller
    }

    pub fn is_server_running(&self) -> bool {
        self.server_running
    }

    pub fn request_controller(&mut self, i: u8) {
        self.sender
            .send_blocking(MessageToNetworkThread::RequestController(Some(i)));
    }

    pub fn release_controller(&mut self) {
        self.sender
            .send_blocking(MessageToNetworkThread::RequestController(None));
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
        if let Some(id) = &self.my_controller {
            if *id == i {
                self.sender
                    .send_blocking(MessageToNetworkThread::ControllerData(i, data));
            }
        }
    }
}
