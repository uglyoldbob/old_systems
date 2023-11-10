//! This is the module for network play related code.

mod emulator;
mod streaming;

use std::collections::HashSet;

use futures::FutureExt;
use libp2p::core::transport::ListenerId;
use libp2p::{futures::StreamExt, Multiaddr, Swarm};

use crate::controller::ButtonCombination;

#[derive(PartialEq, Copy, Clone, Debug, serde::Deserialize, serde::Serialize)]
/// The roles that a node on the network can have
pub enum NodeRole {
    /// The node is only an emulator. Other players and observers will connect to this node.
    DedicatedHost,
    /// The node is a player and a host. The emulator is hosted outside of the network object.
    PlayerHost,
    /// A regular player with a controller "in hand".
    Player,
    /// Just watching the game. No controller inputs are sent to the host.
    Observer,
    /// A newly joined unknown role.
    Unknown,
}

/// Messages sent to the network thread from the main gui.
#[derive(Debug)]
pub enum MessageToNetworkThread {
    /// Controller data from the player, with which controller index and the button data.
    ControllerData(u8, crate::controller::ButtonCombination),
    /// Signal to start PlayerHost mode
    StartServer,
    /// Signal to stop PlayerHost mode
    StopServer,
    /// This is a signal to connect to a host, starting unknown mode.
    Connect(String),
    /// Request to be an observer of the game in progress.
    RequestObserverStatus,
    /// Used by hosts to set the role of a user.
    SetUserRole(libp2p::PeerId, NodeRole),
    /// Used by observers and players to request a different controller than what they are currently holding.
    RequestController(Option<u8>),
    /// Used by hosts to set the held controller of a player or observer.
    SetController(libp2p::PeerId, Option<u8>),
}

/// Represents a message sent from the network thread back to the main thread.
#[derive(Debug)]
pub enum MessageFromNetworkThread {
    /// Emulator video and audio data.
    EmulatorVideoStream(Vec<u8>),
    /// Controller data from one of the players.
    ControllerData(u8, ButtonCombination),
    /// Indicates that the host has a new address
    NewAddress(Multiaddr),
    /// Indicates that the host no longer has the specified address
    ExpiredAddress(Multiaddr),
    /// Indicates the status of a game host.
    ServerStatus(bool),
    /// Indicates that the node is connected to a game host
    ConnectedToHost,
    /// Indicates the new role for the node.
    NewRole(NodeRole),
    /// A request for a different role on the network.
    RequestRole(libp2p::PeerId, NodeRole),
    /// A request to have the specified controller. None means the player becomes an observer.
    RequestController(libp2p::PeerId, Option<u8>),
    /// The held controller for the node is to be changed to what is indicated. None means the player becomes an observer.
    SetController(Option<u8>),
}

/// This describes the behaviour of a libp2p swarm
#[derive(libp2p::swarm::NetworkBehaviour)]
struct SwarmBehavior {
    /// This is the behaviour that operates upnp for servers.
    upnp: libp2p::upnp::tokio::Behaviour,
    /// The main behavior that specifies how to send and receive all of the emulator traffic.
    emulator: emulator::Behavior,
}

/// The object used internally to this module to manage network state
struct InternalNetwork {
    /// The libp2p swarm
    swarm: Swarm<SwarmBehavior>,
    /// The channel used to send messages back to the main ui thread.
    sender: async_channel::Sender<MessageFromNetworkThread>,
    /// The channel used to receive messages from the main ui thread.
    recvr: async_channel::Receiver<MessageToNetworkThread>,
    /// The list of addresses that a server is listening on.
    addresses: HashSet<Multiaddr>,
    /// The proxy object used to indicate that there are new messages on the `sender` channel.
    proxy: egui_multiwin::winit::event_loop::EventLoopProxy<crate::event::Event>,
    /// The id of the listener for a server.
    listener: Option<ListenerId>,
}

impl InternalNetwork {
    /// The main function of the struct. Operates a big loop, driving all network activity.
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

    /// Start the network thread with the tokio runtime.
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
    /// The tokio runtime for the network thread.
    tokio: tokio::runtime::Runtime,
    /// The thread that the tokio runtime runs on
    thread: tokio::task::JoinHandle<()>,
    /// The channel for sending messages to the network thread.
    sender: async_channel::Sender<MessageToNetworkThread>,
    /// The channel for receiving messages from the network thread.
    recvr: async_channel::Receiver<MessageFromNetworkThread>,
    /// The list of addresses that a server is listening on.
    addresses: HashSet<Multiaddr>,
    /// True indicates that the server is running.
    server_running: bool,
    /// The role for this node in the network.
    role: NodeRole,
    /// The cached button inputs for all possible players in the game.
    buttons: [Option<ButtonCombination>; 4],
    /// The controller that this node is holding. None indicates that no controller is held.
    my_controller: Option<u8>,
    /// Indicates who is holding each controller, if there is somebody holding that controller.
    controller_holder: [Option<libp2p::PeerId>; 4],
    /// Indicates that this node is connected to another server.
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

    /// Return the list of addresses that a node is lestening on.
    pub fn get_addresses(&self) -> &HashSet<Multiaddr> {
        &self.addresses
    }

    /// Retrieve the specified index of button data, cloned.
    pub fn get_button_data(&mut self, i: u8) -> Option<ButtonCombination> {
        self.buttons[i as usize].clone()
    }

    /// Process all messages received from the network thread.
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
                        for i in 0..4 {
                            if let Some(peer) = self.controller_holder[i] {
                                if peer == p {
                                    self.controller_holder[i] = None;
                                    break;
                                }
                            }
                        }
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

    /// What is my role in the network?
    pub fn role(&self) -> NodeRole {
        self.role
    }

    /// Returns the index for what controller the current node is handling.
    pub fn get_controller_id(&self) -> Option<u8> {
        self.my_controller
    }

    /// True when the emulator is a running host.
    pub fn is_server_running(&self) -> bool {
        self.server_running
    }

    /// Used by players to request control of a specific controller.
    pub fn request_controller(&mut self, i: u8) {
        self.sender
            .send_blocking(MessageToNetworkThread::RequestController(Some(i)));
    }

    /// Used by players to release control of any controller they may be holding.
    pub fn release_controller(&mut self) {
        self.sender
            .send_blocking(MessageToNetworkThread::RequestController(None));
    }

    /// Starts an emulator host.
    pub fn start_server(&mut self) {
        self.sender
            .send_blocking(MessageToNetworkThread::StartServer);
    }

    /// Stops an emulator host.
    pub fn stop_server(&mut self) {
        self.sender
            .send_blocking(MessageToNetworkThread::StopServer);
    }

    /// Try to connect to an existing emulator host.
    pub fn try_connect(&mut self, cs: &String) {
        self.sender
            .send_blocking(MessageToNetworkThread::Connect(cs.to_owned()));
    }

    /// Send controller data to the emulator host.
    pub fn send_controller_data(&mut self, i: u8, data: crate::controller::ButtonCombination) {
        if let Some(id) = &self.my_controller {
            if *id == i {
                self.sender
                    .send_blocking(MessageToNetworkThread::ControllerData(i, data));
            }
        }
    }
}
