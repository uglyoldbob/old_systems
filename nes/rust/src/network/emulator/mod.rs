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
    swarm::{ConnectionHandler, NetworkBehaviour, SubstreamProtocol, ToSwarm},
    InboundUpgrade, OutboundUpgrade, PeerId, StreamProtocol,
};

use super::{MessageFromNetwork, MessageToNetwork};

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

impl<Stream> OutboundUpgrade<Stream> for Protocol {
    type Output = u16;
    type Error = ();
    type Future = Pin<Box<dyn Future<Output = Result<Self::Output, Self::Error>> + Send>>;

    fn upgrade_outbound(self, io: Stream, protocol: Self::Info) -> Self::Future {
        Box::pin(future::ok(42))
    }
}

#[derive(Debug)]
pub enum ConnectionError {
    Unknown,
}

impl serde::ser::StdError for ConnectionError {}

impl Display for ConnectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub struct Handler {
    listen_protocol: Protocol,
}

impl Handler {
    fn new(protocol: Protocol) -> Self {
        Self {
            listen_protocol: protocol,
        }
    }

    fn do_stuff(&mut self) {
        println!("Handler is just doing stuff, don't worry");
    }
}

impl ConnectionHandler for Handler {
    type FromBehaviour = MessageFromNetwork;

    type ToBehaviour = MessageToNetwork;

    type Error = ConnectionError;

    type InboundProtocol = Protocol;

    type OutboundProtocol = Protocol;

    type InboundOpenInfo = ();

    type OutboundOpenInfo = ();

    fn listen_protocol(
        &self,
    ) -> libp2p::swarm::SubstreamProtocol<Self::InboundProtocol, Self::InboundOpenInfo> {
        SubstreamProtocol::new(self.listen_protocol.clone(), ())
    }

    fn connection_keep_alive(&self) -> libp2p::swarm::KeepAlive {
        libp2p::swarm::KeepAlive::Yes
    }

    fn poll(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<
        libp2p::swarm::ConnectionHandlerEvent<
            Self::OutboundProtocol,
            Self::OutboundOpenInfo,
            Self::ToBehaviour,
            Self::Error,
        >,
    > {
        std::task::Poll::Pending
    }

    fn on_behaviour_event(&mut self, _event: Self::FromBehaviour) {
        todo!()
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
    /// Events that need to be yielded to the outside when polling.
    events: VecDeque<ToSwarm<Event, MessageFromNetwork>>,
    messages: VecDeque<MessageToNetwork>,
    waker: Arc<Mutex<Option<Waker>>>,
}

impl Behavior {
    pub fn new() -> Self {
        Self {
            config: Config::default(),
            events: VecDeque::new(),
            messages: VecDeque::new(),
            waker: Arc::new(Mutex::new(None)),
        }
    }
}

impl Behavior {
    pub fn send_message(&mut self) {
        self.messages.push_back(MessageToNetwork::Test);
    }
}

impl NetworkBehaviour for Behavior {
    type ConnectionHandler = Handler;

    type ToSwarm = Event;

    fn handle_established_inbound_connection(
        &mut self,
        _connection_id: libp2p::swarm::ConnectionId,
        _peer: libp2p::PeerId,
        _local_addr: &libp2p::Multiaddr,
        _remote_addr: &libp2p::Multiaddr,
    ) -> Result<libp2p::swarm::THandler<Self>, libp2p::swarm::ConnectionDenied> {
        Ok(Handler::new(self.config.protocol.clone()))
    }

    fn handle_established_outbound_connection(
        &mut self,
        _connection_id: libp2p::swarm::ConnectionId,
        _peer: libp2p::PeerId,
        _addr: &libp2p::Multiaddr,
        _role_override: libp2p::core::Endpoint,
    ) -> Result<libp2p::swarm::THandler<Self>, libp2p::swarm::ConnectionDenied> {
        Ok(Handler::new(self.config.protocol.clone()))
    }

    fn on_swarm_event(&mut self, event: libp2p::swarm::FromSwarm<Self::ConnectionHandler>) {
        let mut waker = self.waker.lock().unwrap();
        self.events
            .push_back(ToSwarm::GenerateEvent(Event::TestEvent));
        if let Some(waker) = waker.take() {
            waker.wake();
        }
    }

    fn on_connection_handler_event(
        &mut self,
        _peer_id: libp2p::PeerId,
        _connection_id: libp2p::swarm::ConnectionId,
        _event: libp2p::swarm::THandlerOutEvent<Self>,
    ) {
    }

    fn poll(
        &mut self,
        cx: &mut std::task::Context<'_>,
        _params: &mut impl libp2p::swarm::PollParameters,
    ) -> std::task::Poll<libp2p::swarm::ToSwarm<Self::ToSwarm, libp2p::swarm::THandlerInEvent<Self>>>
    {
        let mut waker = self.waker.lock().unwrap();
        if let Some(event) = self.events.pop_front() {
            println!("There is an event {:?}", event);
            return std::task::Poll::Ready(event);
        }
        *waker = Some(cx.waker().clone());
        std::task::Poll::Pending
    }
}
