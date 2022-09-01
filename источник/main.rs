
use libp2p::{ core::upgrade
            , floodsub::{ Floodsub, FloodsubEvent, Topic                 }
            , futures::StreamExt
            , identity
            , mdns::{ Mdns, MdnsEvent                                    }
            , mplex
            , noise::{ Keypair, NoiseConfig, X25519Spec                  }
            , swarm::{ NetworkBehaviourEventProcess, Swarm, SwarmBuilder }
            , tcp::TokioTcpConfig
            , NetworkBehaviour
            , PeerId
            , Transport
}; //use libp2p::{ core::upgrade

use once_cell::sync::Lazy                    ;
use serde::{ Deserialize,Serialize          };
use std::{   env,thread,time                };
use tokio::{ io::AsyncBufReadExt,sync::mpsc };

static CONSOLE : Lazy<Vec<String>>       = Lazy::new(|| env::args().collect()                );
static IDENTITY: Lazy<identity::Keypair> = Lazy::new(|| identity::Keypair::generate_ed25519());
static PEER    : Lazy<PeerId>            = Lazy::new(|| PeerId::from(IDENTITY.public())      );
static TOPIC   : Lazy<Topic>             = Lazy::new(|| Topic::new("words")                  );
static WORD    : Lazy<String>            = Lazy::new(|| CONSOLE[1].clone()                   );

#[derive(Debug, Serialize, Deserialize)]
struct Request { peer: String } 

#[derive(Debug, Serialize, Deserialize)]
struct Response { receiver: String
                , word    : String
                }
#[derive(NetworkBehaviour)]
struct Behaviour {                      floodsub       : Floodsub
                 ,                      mdns           : Mdns    
                 , #[behaviour(ignore)] response_sender: mpsc::UnboundedSender<Response>
                 }
enum EventType { Input(String)
               , Response(Response)
               }
fn send(sender: mpsc::UnboundedSender<Response>, receiver: String) {
 tokio::spawn(async move { if let Err(e) = sender.send( Response { receiver: receiver, word: WORD.to_string() } ) { println!("{:?}", e); } });

} //fn send(sender: mpsc::UnboundedSender<ListResponse>, receiver: String) {

impl NetworkBehaviourEventProcess<FloodsubEvent> for Behaviour {
 fn inject_event(&mut self, event: FloodsubEvent) {
  match event {
   FloodsubEvent::Message(message) => {
    if let Ok(request) = serde_json::from_slice::<Request>(&message.data) {
     println!("request: {:?}", request);

     if request.peer.trim().is_empty() || request.peer == PEER.to_string() {
      let source = message.source.to_string();

      send(self.response_sender.clone(), source.clone());

      println!("Sender:    PEER.to_string(): {:?}", PEER.to_string()); 
      println!("Recipient: source.clone():   {:?}", source.clone()  ); 
     } //if request.peer.trim().is_empty() || request.peer == PEER.to_string() {

    } else if let Ok(response) = serde_json::from_slice::<Response>(&message.data) {
     println!("response:         {:?}", response        ); 
     println!("PEER.to_string(): {:?}", PEER.to_string()); 

     if response.receiver == PEER.to_string() {
      println!("response.word: {}", response.word);

     } //if response.receiver == PEER.to_string() {

    } //} else if let Ok(req) = serde_json::from_slice::<Request>(&msg.data) {
   } //FloodsubEvent::Message(message) => {

   _ => ()
  } //match event {
 } //fn inject_event(&mut self, event: FloodsubEvent) {
} //impl NetworkBehaviourEventProcess<FloodsubEvent> for Behaviour {

impl NetworkBehaviourEventProcess<MdnsEvent> for Behaviour {
 fn inject_event(&mut self, event: MdnsEvent) {
  match event {
   MdnsEvent::Discovered(discovered_list) => { for (peer, _addr) in discovered_list {                                 self.floodsub.add_node_to_partial_view(peer)      ;   } }
   MdnsEvent::Expired(expired_list)       => { for (peer, _addr) in expired_list    { if !self.mdns.has_node(&peer) { self.floodsub.remove_node_from_partial_view(&peer); } } }
  } //match event {
 } //fn inject_event(&mut self, event: MdnsEvent) {
} //impl NetworkBehaviourEventProcess<MdnsEvent> for Behaviour {

#[tokio::main]
async fn main() {
 let (response_sender, mut response_rcv) = mpsc::unbounded_channel();

 let auth_keys = Keypair::<X25519Spec>::new().into_authentic(&IDENTITY).expect("can create auth keys");

 let transp = TokioTcpConfig::new().upgrade(upgrade::Version::V1).authenticate(NoiseConfig::xx(auth_keys).into_authenticated()).multiplex(mplex::MplexConfig::new()).boxed();

 let mut behaviour = Behaviour { floodsub: Floodsub::new(PEER.clone()), mdns: Mdns::new(Default::default()).await.expect("can create mdns"), response_sender };

 behaviour.floodsub.subscribe(TOPIC.clone());

 let mut swarm = SwarmBuilder::new(transp, behaviour, PEER.clone()).executor(Box::new(|fut| { tokio::spawn(fut); })).build();

 let mut stdin = tokio::io::BufReader::new(tokio::io::stdin()).lines();

 Swarm::listen_on(&mut swarm, "/ip4/0.0.0.0/tcp/0".parse().expect("can get a local socket")).expect("swarm can be started");

 loop {
  let evt = { tokio::select! { event    = swarm.select_next_some() => None
                             , line     = stdin.next_line()        => Some(EventType::Input(line.expect("can get line").expect("can read line from stdin")))
                             , response = response_rcv.recv()      => Some(EventType::Response(response.expect("response exists")))
                             } 
            };
  if let Some(event) = evt {
   match event { 
    EventType::Input(line) => {
     println!("EventType::Input(line): {:?}", line);

     let command: &str = line.as_str();

     match command { "exit"   => break
                   , "others" => swarm.behaviour_mut().floodsub.publish(TOPIC.clone(), serde_json::to_string(&Request { peer: "".to_string() }).expect("can jsonify request").as_bytes())
                   , _        => println!("Command {} is unknown", command)
                   } 
    } //EventType::Input(line) => {

    EventType::Response(response) => { 
     println!("EventType::Response(response): {:?}", response);

     let json = serde_json::to_string(&response).expect("can jsonify response");

     swarm.behaviour_mut().floodsub.publish(TOPIC.clone(), json.as_bytes()); 
    } //EventType::Response(response) => { 
   } //match event { 
  } //if let Some(event) = evt {
 } //loop {
} //async fn main() {
