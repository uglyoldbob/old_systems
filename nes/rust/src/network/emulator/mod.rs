use std::{
    collections::VecDeque,
    fmt::Display,
    pin::Pin,
    sync::{Arc, Mutex},
    task::Waker,
};

use futures::{future, Future, StreamExt, SinkExt};
use libp2p::{
    core::UpgradeInfo,
    swarm::{
        handler::{ConnectionEvent, FullyNegotiatedOutbound},
        ConnectionHandler, ConnectionHandlerEvent, NetworkBehaviour, SubstreamProtocol, ToSwarm,
    },
    InboundUpgrade, OutboundUpgrade, PeerId, StreamProtocol,
};

use super::NodeRole;

#[derive(Debug)]
pub enum MessageToNetwork {
    ControllerData(u8, crate::controller::ButtonCombination),
    Test,
}

#[derive(Debug)]
pub enum MessageFromNetwork {
    EmulatorVideoStream(Vec<u8>),
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
    received: VecDeque<u8>,
}

impl Codec {
    fn new() -> Self {
        Self {
            received: VecDeque::new(),
        }
    }
}

impl asynchronous_codec::Decoder for Codec {
    type Item = u8;

    type Error = std::io::Error;

    fn decode(
        &mut self,
        src: &mut asynchronous_codec::BytesMut,
    ) -> Result<Option<Self::Item>, Self::Error> {
        for el in src.iter() {
            self.received.push_back(*el);
        }
        if self.received.len() > 0 {
            Ok(Some(self.received.pop_front().unwrap()))
        }
        else {
            Ok(None)
        }
    }
}

impl asynchronous_codec::Encoder for Codec {
    type Item<'a> = u8;

    type Error = std::io::Error;

    fn encode(
        &mut self,
        item: Self::Item<'_>,
        dst: &mut asynchronous_codec::BytesMut,
    ) -> Result<(), Self::Error> {
        libp2p::bytes::BufMut::put_u8(dst, item);
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
}

impl Handler {
    fn new(protocol: Protocol) -> Self {
        Self {
            listen_protocol: protocol,
            waker: Arc::new(Mutex::new(None)),
            inbound_stream: None,
            outbound_stream: None,
        }
    }

    fn do_stuff(&mut self) {
        println!("Handler is just doing stuff, don't worry");
    }
}

#[derive(Debug)]
pub enum MessageToBehavior {
    Test,
}

#[derive(Debug)]
pub enum MessageFromBehavior {
    Test,
}

impl ConnectionHandler for Handler {
    type FromBehaviour = MessageFromBehavior;

    type ToBehaviour = MessageToBehavior;

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
            println!("outbound established");
            match out.poll_ready_unpin(cx) {
                std::task::Poll::Ready(Ok(())) => {
                    println!("Sending message");
                    match out.start_send_unpin(42) {
                        Ok(()) => {
                            println!("Message sent?");
                            return std::task::Poll::Ready(ConnectionHandlerEvent::NotifyBehaviour(MessageToBehavior::Test));
                        }
                        Err(e) => {
                            println!("Failed to send message");
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
                std::task::Poll::Ready(m) => {
                    match m {
                        Some(Ok(m)) => {
                            return std::task::Poll::Ready(ConnectionHandlerEvent::NotifyBehaviour(
                                MessageToBehavior::Test,
                            ));
                        }
                        Some(Err(e)) => {
                            println!("Error receiving message");
                        }
                        None => {

                        }
                    }
                }
                std::task::Poll::Pending => {}
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
        match event {
            MessageFromBehavior::Test => {}
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
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Message {
    /// Id of the peer that published this message.
    pub source: Option<PeerId>,

    /// Content of the message.
    pub data: Vec<u8>,

    /// A random sequence number.
    pub sequence_number: Option<u64>,
}

impl std::fmt::Debug for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Message")
            .field(
                "data",
                &format_args!("{:<20}", &hex_fmt::HexFmt(&self.data)),
            )
            .field("source", &self.source)
            .field("sequence_number", &self.sequence_number)
            .finish()
    }
}

#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MessageId(pub Vec<u8>);

impl MessageId {
    pub fn new(value: &[u8]) -> Self {
        Self(value.to_vec())
    }
}

impl<T: Into<Vec<u8>>> From<T> for MessageId {
    fn from(value: T) -> Self {
        Self(value.into())
    }
}

impl std::fmt::Display for MessageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", hex_fmt::HexFmt(&self.0))
    }
}

impl std::fmt::Debug for MessageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MessageId({})", hex_fmt::HexFmt(&self.0))
    }
}

#[derive(Debug)]
pub enum Event {
    Message(PeerId, MessageId, Message),
    TestEvent,
    UnsupportedPeer(PeerId),
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
    messages: VecDeque<MessageToNetwork>,
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
        self.messages.push_back(MessageToNetwork::Test);
    }

    pub fn set_role(&mut self, role: NodeRole) {
        self.role = role;
    }
}

impl NetworkBehaviour for Behavior {
    type ConnectionHandler = Handler;

    type ToSwarm = Event;

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

    fn on_swarm_event(&mut self, _event: libp2p::swarm::FromSwarm) {
        let mut waker = self.waker.lock().unwrap();
        if let Some(waker) = waker.take() {
            waker.wake();
        }
    }

    fn on_connection_handler_event(
        &mut self,
        _peer_id: libp2p::PeerId,
        _connection_id: libp2p::swarm::ConnectionId,
        event: libp2p::swarm::THandlerOutEvent<Self>,
    ) {
        println!("Recieved connection handler event {:?}", event);
        let mut waker = self.waker.lock().unwrap();
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
            match a {
                MessageToNetwork::ControllerData(_, _) => todo!(),
                MessageToNetwork::Test => todo!(),
            }
        }

        *waker = Some(cx.waker().clone());
        std::task::Poll::Pending
    }
}
