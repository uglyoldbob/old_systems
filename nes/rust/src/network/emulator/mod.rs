use std::{
    collections::VecDeque,
    fmt::Display,
    pin::Pin,
    sync::{Arc, Mutex},
    task::Waker,
};

use futures::{future, Future};
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

impl<Stream> InboundUpgrade<Stream> for Protocol {
    type Output = u16;
    type Error = ();
    type Future = Pin<Box<dyn Future<Output = Result<Self::Output, Self::Error>> + Send>>;

    fn upgrade_inbound(self, io: Stream, protocol: Self::Info) -> Self::Future {
        Box::pin(future::ok(42))
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

pub struct OutboundSubstreamState {
    stream: libp2p::Stream,
}

pub struct Handler {
    listen_protocol: Protocol,
    waker: Arc<Mutex<Option<Waker>>>,
    /// The single long-lived outbound substream.
    outbound_substream: Option<OutboundSubstreamState>,
    /// Flag indicating that an outbound substream is being established to prevent duplicate
    /// requests.
    outbound_substream_establishing: bool,
}

impl Handler {
    fn new(protocol: Protocol) -> Self {
        Self {
            listen_protocol: protocol,
            waker: Arc::new(Mutex::new(None)),
            outbound_substream: None,
            outbound_substream_establishing: false,
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

        // determine if we need to create the outbound stream
        if self.outbound_substream.is_none() && !self.outbound_substream_establishing {
            self.outbound_substream_establishing = true;
            println!("Requesting a new outbound substream");
            return std::task::Poll::Ready(ConnectionHandlerEvent::OutboundSubstreamRequest {
                protocol: SubstreamProtocol::new(self.listen_protocol.clone(), ()),
            });
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
        if event.is_outbound() {
            self.outbound_substream_establishing = false;
        }
        match event {
            ConnectionEvent::FullyNegotiatedInbound(inb) => {
                println!(
                    "Got a fully negotiated inbound substream {:?}",
                    inb.protocol
                );
            }
            ConnectionEvent::FullyNegotiatedOutbound(fully_negotiated_outbound) => {
                println!("Got a new outbound substream");
                let (a, b) = fully_negotiated_outbound.protocol;
                self.outbound_substream = Some(OutboundSubstreamState { stream: a });
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
