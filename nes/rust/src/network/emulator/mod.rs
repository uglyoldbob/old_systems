use std::{
    collections::VecDeque,
    fmt::Display,
    pin::Pin,
    sync::{Arc, Mutex},
    task::Waker,
};

use futures::{future, Future, SinkExt, StreamExt};
use libp2p::{
    core::UpgradeInfo,
    swarm::{
        handler::{ConnectionEvent, FullyNegotiatedOutbound},
        ConnectionHandler, ConnectionHandlerEvent, NetworkBehaviour, SubstreamProtocol, ToSwarm,
    },
    InboundUpgrade, OutboundUpgrade, PeerId, StreamProtocol,
};

use crate::controller::ButtonCombination;

use super::NodeRole;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub enum MessageToFromNetwork {
    ControllerData(u8, crate::controller::ButtonCombination),
    EmulatorVideoStream(Vec<u8>),
    RequestRole(PeerId, NodeRole),
    SetRole(NodeRole),
    Test,
}

/// The protocol ID
#[derive(Clone, Debug, PartialEq)]
pub struct ProtocolId {
    /// The RPC message type/name.
    pub protocol: StreamProtocol,
}

impl AsRef<str> for ProtocolId {
    fn as_ref(&self) -> &str {
        self.protocol.as_ref()
    }
}

#[derive(Clone)]
pub struct Protocol {
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

pub struct Codec {
    length: Option<u32>,
    received: VecDeque<u8>,
}

impl Codec {
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

        if self.length.is_none() {
            if self.received.len() >= 4 {
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
        }
        if let Some(l) = &self.length {
            if self.received.len() >= *l as usize {
                match bincode::deserialize::<MessageToFromNetwork>(&Vec::from(
                    self.received.clone(),
                )) {
                    Ok(i) => {
                        println!("Success deserializing");
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

pub struct InboundSubstreamState {
    stream: asynchronous_codec::Framed<libp2p::Stream, Codec>,
}

pub enum OutboundSubstreamState {
    Establishing,
    Established(asynchronous_codec::Framed<libp2p::Stream, Codec>),
}

pub struct Handler {
    listen_protocol: Protocol,
    waker: Arc<Mutex<Option<Waker>>>,
    inbound_stream: Option<InboundSubstreamState>,
    outbound_stream: Option<OutboundSubstreamState>,
    pending_out: VecDeque<MessageToFromBehavior>,
}

impl Handler {
    fn new(protocol: Protocol) -> Self {
        Self {
            listen_protocol: protocol,
            waker: Arc::new(Mutex::new(None)),
            inbound_stream: None,
            outbound_stream: None,
            pending_out: VecDeque::new(),
        }
    }

    fn do_stuff(&mut self) {
        println!("Handler is just doing stuff, don't worry");
    }
}

#[derive(Debug)]
pub enum MessageToFromBehavior {
    ControllerData(u8, ButtonCombination),
    RequestRole(PeerId, NodeRole),
    SetRole(NodeRole),
    VideoStream(Vec<u8>),
    Test,
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
                            MessageToFromBehavior::SetRole(r) => {
                                out.start_send_unpin(MessageToFromNetwork::SetRole(r))
                            }
                            MessageToFromBehavior::ControllerData(i, d) => {
                                out.start_send_unpin(MessageToFromNetwork::ControllerData(i, d))
                            }
                            MessageToFromBehavior::Test => {
                                out.start_send_unpin(MessageToFromNetwork::Test)
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
                                std::task::Poll::Ready(Ok(())) => {
                                    return std::task::Poll::Ready(
                                        ConnectionHandlerEvent::NotifyBehaviour(
                                            MessageToFromBehavior::Test,
                                        ),
                                    );
                                }
                                std::task::Poll::Ready(Err(e)) => {
                                    println!("Error flushing message {:?}", e);
                                }
                                std::task::Poll::Pending => {}
                            },
                            Err(e) => {
                                println!("Failed to send message {:?}", e);
                            }
                        }
                    } else {
                        println!("NOT SURE WHAT TO DO HERE!");
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
            println!("Need to check inbound stream");
            let a = &mut inb.stream;
            match a.poll_next_unpin(cx) {
                std::task::Poll::Ready(m) => {
                    println!("Inbound stream is ready");
                    match m {
                        Some(Ok(m)) => {
                            println!("Received message from network {:?}", m);
                            let mr = match m {
                                MessageToFromNetwork::SetRole(r) => {
                                    MessageToFromBehavior::SetRole(r)
                                }
                                MessageToFromNetwork::RequestRole(p, r) => {
                                    MessageToFromBehavior::RequestRole(p, r)
                                }
                                MessageToFromNetwork::Test => MessageToFromBehavior::Test,
                                MessageToFromNetwork::ControllerData(i, d) => {
                                    MessageToFromBehavior::ControllerData(i, d)
                                }
                                MessageToFromNetwork::EmulatorVideoStream(d) => {
                                    MessageToFromBehavior::VideoStream(d)
                                }
                            };
                            return std::task::Poll::Ready(
                                ConnectionHandlerEvent::NotifyBehaviour(mr),
                            );
                        }
                        Some(Err(e)) => {
                            println!("Error receiving message from network");
                        }
                        None => {
                            println!("Stream closed on remote side");
                            self.inbound_stream = None;
                        }
                    }
                }
                std::task::Poll::Pending => {
                    println!("Inbound stream is pending");
                }
            }
        }

        *waker = Some(cx.waker().clone());
        std::task::Poll::Pending
    }

    fn on_behaviour_event(&mut self, event: Self::FromBehaviour) {
        println!(
            "Connection handler received {:?} from networkbehaviour",
            event
        );
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
                self.outbound_stream = Some(OutboundSubstreamState::Established(
                    asynchronous_codec::Framed::new(a, c),
                ));
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

#[derive(Clone)]
pub struct Config {
    protocol: Protocol,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            protocol: Protocol::default(),
        }
    }
}

pub struct Behavior {
    config: Config,
    messages: VecDeque<ToSwarm<MessageToSwarm, MessageToFromBehavior>>,
    waker: Arc<Mutex<Option<Waker>>>,
    clients: Vec<PeerId>,
    servers: Vec<PeerId>,
    role: super::NodeRole,
}

impl Behavior {
    pub fn new() -> Self {
        Self {
            config: Config::default(),
            messages: VecDeque::new(),
            waker: Arc::new(Mutex::new(None)),
            clients: Vec::new(),
            servers: Vec::new(),
            role: super::NodeRole::Observer,
        }
    }
}

impl Behavior {
    pub fn send_message(&mut self) {
        for pid in &self.clients {
            self.messages.push_back(ToSwarm::NotifyHandler {
                peer_id: *pid,
                handler: libp2p::swarm::NotifyHandler::Any,
                event: MessageToFromBehavior::Test,
            });
        }
        for pid in &self.servers {
            self.messages.push_back(ToSwarm::NotifyHandler {
                peer_id: *pid,
                handler: libp2p::swarm::NotifyHandler::Any,
                event: MessageToFromBehavior::Test,
            });
        }
        let mut waker = self.waker.lock().unwrap();
        if let Some(waker) = waker.take() {
            waker.wake();
        }
    }

    pub fn send_controller_data(&mut self, index: u8, data: ButtonCombination) {
        for pid in &self.servers {
            self.messages.push_back(ToSwarm::NotifyHandler {
                peer_id: *pid,
                handler: libp2p::swarm::NotifyHandler::Any,
                event: MessageToFromBehavior::ControllerData(index, data),
            });
        }
        let mut waker = self.waker.lock().unwrap();
        if let Some(waker) = waker.take() {
            waker.wake();
        }
    }

    pub fn set_role(&mut self, role: NodeRole) {
        self.role = role;
    }

    pub fn set_user_role(&mut self, p: PeerId, r: NodeRole) {
        self.messages.push_back(ToSwarm::NotifyHandler {
            peer_id: p,
            handler: libp2p::swarm::NotifyHandler::Any,
            event: MessageToFromBehavior::SetRole(r),
        });
    }

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
}

#[derive(Debug)]
pub enum MessageToSwarm {
    Test,
    ControllerData(u8, ButtonCombination),
    RequestRole(PeerId, NodeRole),
    SetRole(NodeRole),
}

#[derive(Debug)]
pub enum MessageFromSwarm {
    Test,
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
        Ok(Handler::new(self.config.protocol.clone()))
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
        Ok(Handler::new(self.config.protocol.clone()))
    }

    fn on_swarm_event(&mut self, _event: libp2p::swarm::FromSwarm) {}

    fn on_connection_handler_event(
        &mut self,
        _peer_id: libp2p::PeerId,
        _connection_id: libp2p::swarm::ConnectionId,
        event: libp2p::swarm::THandlerOutEvent<Self>,
    ) {
        println!("Recieved connection handler event {:?}", event);
        let mut waker = self.waker.lock().unwrap();
        match event {
            MessageToFromBehavior::SetRole(r) => {
                self.messages
                    .push_back(ToSwarm::GenerateEvent(MessageToSwarm::SetRole(r)));
            }
            MessageToFromBehavior::ControllerData(i, d) => {
                self.messages
                    .push_back(ToSwarm::GenerateEvent(MessageToSwarm::ControllerData(i, d)));
            }
            MessageToFromBehavior::VideoStream(v) => todo!(),
            MessageToFromBehavior::Test => todo!(),
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
            println!("Processing message {:?}", a);
            return std::task::Poll::Ready(a);
        }

        *waker = Some(cx.waker().clone());
        std::task::Poll::Pending
    }
}
