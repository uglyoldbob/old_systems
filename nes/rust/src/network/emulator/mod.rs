//! A module that implements a custom protocl for libp2p.

use std::{
    collections::VecDeque,
    pin::Pin,
    sync::{Arc, Mutex},
    task::Waker,
};

use futures::{future, Future, SinkExt, StreamExt};
use libp2p::{
    core::UpgradeInfo,
    swarm::{
        handler::ConnectionEvent, ConnectionHandler, ConnectionHandlerEvent, NetworkBehaviour,
        SubstreamProtocol, ToSwarm,
    },
    InboundUpgrade, OutboundUpgrade, PeerId, StreamProtocol,
};

use crate::controller::ButtonCombination;

use super::{streaming::Streaming, NodeRole};

/// Represents a message that can be sent to and from other nodes in the network.
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub enum MessageToFromNetwork {
    /// Controller data for a specific controller.
    ControllerData(u8, Box<crate::controller::ButtonCombination>),
    /// Part of the emulator audio video stream.
    EmulatorVideoStream(Vec<u8>),
    /// A request for a specific role in the network.
    RequestRole(PeerId, NodeRole),
    /// A request to own a specific controller in the emulator network.
    RequestController(PeerId, Option<u8>),
    /// A command to set the role of the recipient of the command.
    SetRole(NodeRole),
    /// A command to set the controller of the recipient of the command.
    SetController(Option<u8>),
}

/// The protocol ID
#[derive(Clone, Debug, PartialEq)]
pub struct ProtocolId {
    /// The message type/name.
    pub protocol: StreamProtocol,
}

impl AsRef<str> for ProtocolId {
    fn as_ref(&self) -> &str {
        self.protocol.as_ref()
    }
}

/// Represents the protocols compatible with the node.
#[derive(Clone)]
pub struct Protocol {
    /// The list of compatible protocols.
    protocols: Vec<ProtocolId>,
}

impl Default for Protocol {
    fn default() -> Self {
        Self {
            protocols: vec![ProtocolId {
                protocol: StreamProtocol::new("/nes/0.0.1"),
            }],
        }
    }
}

impl UpgradeInfo for Protocol {
    type Info = ProtocolId;
    type InfoIter = Vec<Self::Info>;

    fn protocol_info(&self) -> Self::InfoIter {
        self.protocols.clone()
    }
}

impl<Stream> InboundUpgrade<Stream> for Protocol
where
    Stream: Send + 'static,
{
    type Output = (Stream, Self::Info);
    type Error = ();
    type Future = Pin<Box<dyn Future<Output = Result<Self::Output, Self::Error>> + Send>>;

    fn upgrade_inbound(self, io: Stream, protocol: Self::Info) -> Self::Future {
        Box::pin(future::ok((io, protocol)))
    }
}

impl<Stream> OutboundUpgrade<Stream> for Protocol
where
    Stream: Send + 'static,
{
    type Output = (Stream, Self::Info);
    type Error = ();
    type Future = Pin<Box<dyn Future<Output = Result<Self::Output, Self::Error>> + Send>>;

    fn upgrade_outbound(self, io: Stream, protocol: Self::Info) -> Self::Future {
        Box::pin(future::ok((io, protocol)))
    }
}

/// This is used to translate messages to and from the network.
pub struct Codec {
    /// The length to be received from the network.
    length: Option<u32>,
    /// The received data from the network.
    received: VecDeque<u8>,
}

impl Codec {
    /// Create a new object.
    fn new() -> Self {
        Self {
            length: None,
            received: VecDeque::new(),
        }
    }
}

impl asynchronous_codec::Decoder for Codec {
    type Item = MessageToFromNetwork;

    type Error = std::io::Error;

    fn decode(
        &mut self,
        src: &mut asynchronous_codec::BytesMut,
    ) -> Result<Option<Self::Item>, Self::Error> {
        for el in src.iter() {
            self.received.push_back(*el);
        }
        src.clear();

        if self.length.is_none() && self.received.len() >= 4 {
            let v = (self.received[3] as u32)
                | (self.received[2] as u32) << 8
                | (self.received[1] as u32) << 16
                | (self.received[0] as u32) << 24;
            self.length = Some(v);
            self.received.pop_front();
            self.received.pop_front();
            self.received.pop_front();
            self.received.pop_front();
        }
        if let Some(l) = &self.length {
            if self.received.len() >= *l as usize {
                match bincode::deserialize::<MessageToFromNetwork>(&Vec::from(
                    self.received.clone(),
                )) {
                    Ok(i) => {
                        self.length = None;
                        self.received.clear();
                        Ok(Some(i))
                    }
                    Err(e) => {
                        println!("Error deserializing data {:?}", e);
                        self.length = None;
                        self.received.clear();
                        Ok(None) //TODO convert into an actual error
                    }
                }
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }
}

impl asynchronous_codec::Encoder for Codec {
    type Item<'a> = MessageToFromNetwork;

    type Error = std::io::Error;

    fn encode(
        &mut self,
        item: Self::Item<'_>,
        dst: &mut asynchronous_codec::BytesMut,
    ) -> Result<(), Self::Error> {
        let data = bincode::serialize(&item).unwrap();
        libp2p::bytes::BufMut::put_u32(dst, data.len() as u32);
        libp2p::bytes::BufMut::put_slice(dst, &data);
        Ok(())
    }
}

/// A struct representing an inbound substream
pub struct InboundSubstreamState {
    /// The stream
    stream: asynchronous_codec::Framed<libp2p::Stream, Codec>,
}

/// A struct representing an outbound substream
pub enum OutboundSubstreamState {
    /// The substream is being established
    Establishing,
    /// The substream has been established, and has a stream
    Established(Box<asynchronous_codec::Framed<libp2p::Stream, Codec>>),
}

/// The libp2p handler struct
pub struct Handler {
    /// The role played by this node in the network.
    role: NodeRole,
    /// The protocol to use for communication.
    listen_protocol: Protocol,
    /// The waker used to wake up the poll stuff when is is marked pending.
    waker: Arc<Mutex<Option<Waker>>>,
    /// The inbound stream
    inbound_stream: Option<InboundSubstreamState>,
    /// The outbound stream
    outbound_stream: Option<OutboundSubstreamState>,
    /// Messages that are pending to be sent to the network.
    pending_out: VecDeque<MessageToFromBehavior>,
    /// The struct used for converting video and audio data into a format that can be streamed.
    streaming: Streaming,
    /// This is how the stream data is obtained. Data is pulled from this and then sent over the network.
    avsource: Option<gstreamer_app::AppSink>,
}

impl Handler {
    /// Construct a new struct.
    fn new(protocol: Protocol, role: NodeRole) -> Self {
        Self {
            role,
            listen_protocol: protocol,
            waker: Arc::new(Mutex::new(None)),
            inbound_stream: None,
            outbound_stream: None,
            pending_out: VecDeque::new(),
            streaming: Streaming::new(),
            avsource: None,
        }
    }
}

/// Represents messages that can be send between the networkbehaviour and the connectionhandler.
#[derive(Debug)]
pub enum MessageToFromBehavior {
    /// Controller data for a specific controller.
    ControllerData(u8, Box<ButtonCombination>),
    /// A request for a specific role on the network.
    RequestRole(PeerId, NodeRole),
    /// A request for a specific controller on the emulator.
    RequestController(PeerId, Option<u8>),
    /// A command to give a specific controller to a node on the network.
    SetController(Option<u8>),
    /// A command to set the role of a node on the network.
    SetRole(NodeRole),
    /// Audio video data for the emulator.
    VideoStream(Vec<u8>),
}

impl ConnectionHandler for Handler {
    type FromBehaviour = MessageToFromBehavior;

    type ToBehaviour = MessageToFromBehavior;

    type InboundProtocol = Protocol;

    type OutboundProtocol = Protocol;

    type InboundOpenInfo = ();

    type OutboundOpenInfo = ();

    fn listen_protocol(
        &self,
    ) -> libp2p::swarm::SubstreamProtocol<Self::InboundProtocol, Self::InboundOpenInfo> {
        SubstreamProtocol::new(self.listen_protocol.clone(), ())
    }

    fn poll(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<
        libp2p::swarm::ConnectionHandlerEvent<
            Self::OutboundProtocol,
            Self::OutboundOpenInfo,
            Self::ToBehaviour,
        >,
    > {
        let mut waker = self.waker.lock().unwrap();

        if self.outbound_stream.is_none() {
            self.outbound_stream = Some(OutboundSubstreamState::Establishing);
            return std::task::Poll::Ready(ConnectionHandlerEvent::OutboundSubstreamRequest {
                protocol: self.listen_protocol(),
            });
        }
        if let Some(OutboundSubstreamState::Established(out)) = &mut self.outbound_stream {
            match out.poll_ready_unpin(cx) {
                std::task::Poll::Ready(Ok(())) => {
                    if let Some(m) = self.pending_out.pop_front() {
                        let m2 = match m {
                            MessageToFromBehavior::RequestController(i, c) => {
                                out.start_send_unpin(MessageToFromNetwork::RequestController(i, c))
                            }
                            MessageToFromBehavior::SetController(c) => {
                                out.start_send_unpin(MessageToFromNetwork::SetController(c))
                            }
                            MessageToFromBehavior::SetRole(r) => {
                                out.start_send_unpin(MessageToFromNetwork::SetRole(r))
                            }
                            MessageToFromBehavior::ControllerData(i, d) => {
                                out.start_send_unpin(MessageToFromNetwork::ControllerData(i, d))
                            }
                            MessageToFromBehavior::VideoStream(d) => {
                                out.start_send_unpin(MessageToFromNetwork::EmulatorVideoStream(d))
                            }
                            MessageToFromBehavior::RequestRole(p, r) => {
                                out.start_send_unpin(MessageToFromNetwork::RequestRole(p, r))
                            }
                        };
                        match m2 {
                            Ok(()) => match out.poll_flush_unpin(cx) {
                                std::task::Poll::Ready(Ok(())) => {}
                                std::task::Poll::Ready(Err(e)) => {
                                    println!("Error flushing message {:?}", e);
                                }
                                std::task::Poll::Pending => {}
                            },
                            Err(e) => {
                                println!("Failed to send message {:?}", e);
                            }
                        }
                    }
                    if let Some(source) = &mut self.avsource {
                        match source.pull_sample() {
                            Ok(a) => {
                                let c = a.buffer();
                                if let Some(buf) = c {
                                    let mut v: Vec<u8> = Vec::with_capacity(buf.size());
                                    if let Ok(()) = buf.copy_to_slice(buf.size(), &mut v) {
                                        match out.start_send_unpin(
                                            MessageToFromNetwork::EmulatorVideoStream(v),
                                        ) {
                                            Ok(()) => match out.poll_flush_unpin(cx) {
                                                std::task::Poll::Ready(Ok(())) => {}
                                                std::task::Poll::Ready(Err(e)) => {
                                                    println!("Error flushing message {:?}", e);
                                                }
                                                std::task::Poll::Pending => {}
                                            },
                                            Err(e) => {
                                                println!("Failed to send message {:?}", e);
                                            }
                                        }
                                    }
                                }
                            }
                            Err(_e) => {}
                        }
                    }
                }
                std::task::Poll::Ready(Err(e)) => {
                    println!("ERROR 2: {:?}", e);
                }
                std::task::Poll::Pending => {
                    println!("poll ready pending");
                }
            }
        }

        if let Some(inb) = &mut self.inbound_stream {
            let a = &mut inb.stream;
            match a.poll_next_unpin(cx) {
                std::task::Poll::Ready(m) => match m {
                    Some(Ok(m)) => {
                        let mr = match m {
                            MessageToFromNetwork::RequestController(i, c) => {
                                MessageToFromBehavior::RequestController(i, c)
                            }
                            MessageToFromNetwork::SetController(c) => {
                                MessageToFromBehavior::SetController(c)
                            }
                            MessageToFromNetwork::SetRole(r) => MessageToFromBehavior::SetRole(r),
                            MessageToFromNetwork::RequestRole(p, r) => {
                                MessageToFromBehavior::RequestRole(p, r)
                            }
                            MessageToFromNetwork::ControllerData(i, d) => {
                                MessageToFromBehavior::ControllerData(i, d)
                            }
                            MessageToFromNetwork::EmulatorVideoStream(d) => {
                                MessageToFromBehavior::VideoStream(d)
                            }
                        };
                        return std::task::Poll::Ready(ConnectionHandlerEvent::NotifyBehaviour(mr));
                    }
                    Some(Err(_e)) => {
                        println!("Error receiving message from network");
                    }
                    None => {
                        println!("Stream closed on remote side");
                        self.inbound_stream = None;
                    }
                },
                std::task::Poll::Pending => {}
            }
        }

        *waker = Some(cx.waker().clone());
        std::task::Poll::Pending
    }

    fn on_behaviour_event(&mut self, event: Self::FromBehaviour) {
        self.pending_out.push_back(event);
        let mut waker = self.waker.lock().unwrap();
        if let Some(waker) = waker.take() {
            waker.wake();
        }
    }

    fn on_connection_event(
        &mut self,
        event: libp2p::swarm::handler::ConnectionEvent<
            Self::InboundProtocol,
            Self::OutboundProtocol,
            Self::InboundOpenInfo,
            Self::OutboundOpenInfo,
        >,
    ) {
        match event {
            ConnectionEvent::FullyNegotiatedInbound(inb) => {
                println!(
                    "Got a fully negotiated inbound substream {:?}",
                    inb.protocol
                );
                let (a, _b) = inb.protocol;
                let c = Codec::new();
                if self.inbound_stream.is_none() {
                    self.inbound_stream = Some(InboundSubstreamState {
                        stream: asynchronous_codec::Framed::new(a, c),
                    });
                }
            }
            ConnectionEvent::FullyNegotiatedOutbound(fully_negotiated_outbound) => {
                println!(
                    "Got a new outbound substream {:?}",
                    fully_negotiated_outbound.protocol
                );
                let (a, _b) = fully_negotiated_outbound.protocol;
                let c = Codec::new();
                self.outbound_stream = Some(OutboundSubstreamState::Established(Box::new(
                    asynchronous_codec::Framed::new(a, c),
                )));
            }
            _ => {}
        }
    }
}

/// The message sent to the user after a [`RawMessage`] has been transformed by a
/// [`crate::DataTransform`].
pub struct Message {
    /// Id of the peer that published this message.
    pub source: Option<PeerId>,

    /// Content of the message.
    pub data: MessageToFromNetwork,
}

impl std::fmt::Debug for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Message")
            .field("data", &format_args!("{:?}", self.data))
            .field("source", &self.source)
            .finish()
    }
}

/// Represents a protocol configuration for the behavior.
#[derive(Clone, Default)]
pub struct Config {
    /// The supported protocol
    protocol: Protocol,
}

/// The struct for the network behavior
pub struct Behavior {
    /// The protocool configuration for the node.
    config: Config,
    /// Messages to be sent to the swarm.
    messages: VecDeque<ToSwarm<MessageToSwarm, MessageToFromBehavior>>,
    /// The waker to wake up the poll routine for this struct.
    waker: Arc<Mutex<Option<Waker>>>,
    /// The list of clients connected to this node.
    clients: Vec<PeerId>,
    /// The list of servers that this node is connected to. This might be converted to an Option<PeerId>
    servers: Vec<PeerId>,
}

impl Behavior {
    /// Construct a new Self
    pub fn new() -> Self {
        Self {
            config: Config::default(),
            messages: VecDeque::new(),
            waker: Arc::new(Mutex::new(None)),
            clients: Vec::new(),
            servers: Vec::new(),
        }
    }
}

impl Behavior {
    /// Send the given controller data to the host.
    pub fn send_controller_data(&mut self, index: u8, data: Box<ButtonCombination>) {
        for pid in &self.servers {
            self.messages.push_back(ToSwarm::NotifyHandler {
                peer_id: *pid,
                handler: libp2p::swarm::NotifyHandler::Any,
                event: MessageToFromBehavior::ControllerData(index, data.clone()),
            });
        }
        let mut waker = self.waker.lock().unwrap();
        if let Some(waker) = waker.take() {
            waker.wake();
        }
    }

    /// Used by a host to set the role of a connected user.
    pub fn set_user_role(&mut self, p: PeerId, r: NodeRole) {
        self.messages.push_back(ToSwarm::NotifyHandler {
            peer_id: p,
            handler: libp2p::swarm::NotifyHandler::Any,
            event: MessageToFromBehavior::SetRole(r),
        });
    }

    /// Used by a player to request observer status.
    pub fn request_observer_status(&mut self, p: PeerId) {
        for pid in &self.servers {
            self.messages.push_back(ToSwarm::NotifyHandler {
                peer_id: *pid,
                handler: libp2p::swarm::NotifyHandler::Any,
                event: MessageToFromBehavior::RequestRole(p, NodeRole::Observer),
            });
        }
        let mut waker = self.waker.lock().unwrap();
        if let Some(waker) = waker.take() {
            waker.wake();
        }
    }

    /// Used by a player to request posession of a specific controller in the emulator.
    pub fn request_controller(&mut self, p: PeerId, c: Option<u8>) {
        for pid in &self.servers {
            self.messages.push_back(ToSwarm::NotifyHandler {
                peer_id: *pid,
                handler: libp2p::swarm::NotifyHandler::Any,
                event: MessageToFromBehavior::RequestController(p, c),
            });
        }
        let mut waker = self.waker.lock().unwrap();
        if let Some(waker) = waker.take() {
            waker.wake();
        }
    }

    /// Used by a host to set the controller of a user.
    pub fn set_controller(&mut self, p: PeerId, c: Option<u8>) {
        self.messages.push_back(ToSwarm::NotifyHandler {
            peer_id: p,
            handler: libp2p::swarm::NotifyHandler::Any,
            event: MessageToFromBehavior::SetController(c),
        });
    }
}

/// Represents the types of messages that can be sent to the swarm from the network behaviour.
#[derive(Debug)]
pub enum MessageToSwarm {
    /// Controller data from a user
    ControllerData(u8, Box<ButtonCombination>),
    /// A role request from a user.
    RequestRole(PeerId, NodeRole),
    /// A command from a host to set the role of a user.
    SetRole(NodeRole),
    /// A request from a user to have a certain controller. None means to be just an observer.
    RequestController(PeerId, Option<u8>),
    /// A command from a host that sets the controller of a user.
    SetController(Option<u8>),
    /// A singal that indicates the node is connected to a host.
    ConnectedToHost,
}

impl NetworkBehaviour for Behavior {
    type ConnectionHandler = Handler;

    type ToSwarm = MessageToSwarm;

    fn handle_established_inbound_connection(
        &mut self,
        _connection_id: libp2p::swarm::ConnectionId,
        peer: libp2p::PeerId,
        _local_addr: &libp2p::Multiaddr,
        _remote_addr: &libp2p::Multiaddr,
    ) -> Result<libp2p::swarm::THandler<Self>, libp2p::swarm::ConnectionDenied> {
        println!("Received inbound connection from {:?}", peer);
        self.clients.push(peer);
        Ok(Handler::new(
            self.config.protocol.clone(),
            NodeRole::PlayerHost,
        ))
    }

    fn handle_established_outbound_connection(
        &mut self,
        _connection_id: libp2p::swarm::ConnectionId,
        peer: libp2p::PeerId,
        _addr: &libp2p::Multiaddr,
        _role_override: libp2p::core::Endpoint,
    ) -> Result<libp2p::swarm::THandler<Self>, libp2p::swarm::ConnectionDenied> {
        println!("Established outbound to {:?}", peer);
        self.servers.push(peer);
        self.messages
            .push_back(ToSwarm::GenerateEvent(MessageToSwarm::ConnectedToHost));
        Ok(Handler::new(
            self.config.protocol.clone(),
            NodeRole::Unknown,
        ))
    }

    fn on_swarm_event(&mut self, _event: libp2p::swarm::FromSwarm) {}

    fn on_connection_handler_event(
        &mut self,
        _peer_id: libp2p::PeerId,
        _connection_id: libp2p::swarm::ConnectionId,
        event: libp2p::swarm::THandlerOutEvent<Self>,
    ) {
        let mut waker = self.waker.lock().unwrap();
        match event {
            MessageToFromBehavior::RequestController(i, c) => {
                self.messages
                    .push_back(ToSwarm::GenerateEvent(MessageToSwarm::RequestController(
                        i, c,
                    )));
            }
            MessageToFromBehavior::SetController(c) => {
                self.messages
                    .push_back(ToSwarm::GenerateEvent(MessageToSwarm::SetController(c)));
            }
            MessageToFromBehavior::SetRole(r) => {
                self.messages
                    .push_back(ToSwarm::GenerateEvent(MessageToSwarm::SetRole(r)));
            }
            MessageToFromBehavior::ControllerData(i, d) => {
                self.messages
                    .push_back(ToSwarm::GenerateEvent(MessageToSwarm::ControllerData(i, d)));
            }
            MessageToFromBehavior::VideoStream(_v) => todo!(),
            MessageToFromBehavior::RequestRole(p, r) => {
                self.messages
                    .push_back(ToSwarm::GenerateEvent(MessageToSwarm::RequestRole(p, r)));
            }
        }
        if let Some(waker) = waker.take() {
            waker.wake();
        }
    }

    fn poll(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<libp2p::swarm::ToSwarm<Self::ToSwarm, libp2p::swarm::THandlerInEvent<Self>>>
    {
        let mut waker = self.waker.lock().unwrap();

        if let Some(a) = self.messages.pop_front() {
            return std::task::Poll::Ready(a);
        }

        *waker = Some(cx.waker().clone());
        std::task::Poll::Pending
    }
}
