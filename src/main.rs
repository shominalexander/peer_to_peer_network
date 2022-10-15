use libp2p::{ core::upgrade
            , floodsub::{ Floodsub, FloodsubEvent, Topic                             }
            , futures::StreamExt
            , identity
            , mdns::{ Mdns, MdnsEvent                                                }
            , mplex
            , noise::{ Keypair, NoiseConfig, X25519Spec                              }
            , swarm::{ NetworkBehaviourEventProcess, Swarm, SwarmBuilder, SwarmEvent }
            , tcp::TokioTcpConfig
            , Multiaddr 
            , NetworkBehaviour
            , PeerId
            , Transport
};//use libp2p::{ core::upgrade

use once_cell::sync::Lazy                     ;
use serde::{ Deserialize, Serialize          };
use std::{ env, thread, time                 };
use tokio::{ io::AsyncBufReadExt, sync::mpsc };

#[derive(Debug, Deserialize, Serialize)]
struct Request { destination: String } 

#[derive(Debug, Deserialize, Serialize)]
struct Response { receiver: String
                , text    : String
                }
#[derive(NetworkBehaviour)]
struct Behaviour {                      floodsub: Floodsub
                 ,                      mdns    : Mdns    
                 , #[behaviour(ignore)] peer    : PeerId
                 , #[behaviour(ignore)] sender  : mpsc::UnboundedSender<Response>
                 , #[behaviour(ignore)] text    : String
                 }
enum EventType { Address(Multiaddr)
               , Input(String)
               , Response(Response)
               }
pub static DELAY: Lazy<u64> = Lazy::new(|| 400);

impl NetworkBehaviourEventProcess<FloodsubEvent> for Behaviour {
 fn inject_event(&mut self, event: FloodsubEvent) {
  match event {
   FloodsubEvent::Message(message) => {
    println!("message.source: {:?}", message.source); thread::sleep(time::Duration::from_millis(*DELAY));

    if let Ok(request) = serde_json::from_slice::<Request>(&message.data) {
     println!("request:        {:?}", request); thread::sleep(time::Duration::from_millis(*DELAY));

     if request.destination.trim().is_empty() || request.destination == self.peer.to_string() {
      if let Err(e) = self.sender.clone().send( Response { receiver: message.source.to_string(), text: self.text.to_string() } ) { 
       println!("{:?}", e); thread::sleep(time::Duration::from_millis(*DELAY));

      }//if let Err(e) = self.sender.clone().send( Response { receiver: message.source.to_string(), text: self.text.to_string() } ) { 
     }//if request.destination.trim().is_empty() || request.destination == self.peer.to_string() {

    } else if let Ok(response) = serde_json::from_slice::<Response>(&message.data) {
     println!("response:       {:?}", response); thread::sleep(time::Duration::from_millis(*DELAY));

    }//} else if let Ok(req) = serde_json::from_slice::<Request>(&msg.data) {
   }//FloodsubEvent::Message(message) => {

   _ => ()
  }//match event {
 }//fn inject_event(&mut self, event: FloodsubEvent) {
}//impl NetworkBehaviourEventProcess<FloodsubEvent> for Behaviour {

impl NetworkBehaviourEventProcess<MdnsEvent> for Behaviour {
 fn inject_event(&mut self, event: MdnsEvent) {
  match event {
   MdnsEvent::Discovered(discovered) => { 
    for (peer, addr) in discovered {         
     self.floodsub.add_node_to_partial_view(peer);   

     println!("discovered: {:?}, {:?}", peer, addr); thread::sleep(time::Duration::from_millis(*DELAY)); 
    }//for (peer, addr) in discovered {         
   }//MdnsEvent::Discovered(discovered) => { 

   MdnsEvent::Expired(expired) => { 
    for (peer, addr) in expired {
     if !self.mdns.has_node(&peer) { 
      self.floodsub.remove_node_from_partial_view(&peer);

      println!("discovered: {:?}, {:?}", peer, addr); thread::sleep(time::Duration::from_millis(*DELAY)); 
     }//if !self.mdns.has_node(&peer) { 
    }//for (peer, addr) in expired {
   }//MdnsEvent::Expired(expired) => { 
  }//match event {
 }//fn inject_event(&mut self, event: MdnsEvent) {
}//impl NetworkBehaviourEventProcess<MdnsEvent> for Behaviour {

#[tokio::main]
async fn main() {
 let identity: identity::Keypair = identity::Keypair::generate_ed25519();

 let peer: PeerId = PeerId::from(identity.public());

 let args: Vec<String> = env::args().collect();

 let text: String = args[1].clone();

 let (sender, mut recipient) = mpsc::unbounded_channel();

 let auth_keys = Keypair::<X25519Spec>::new().into_authentic(&identity).expect("can create auth keys");

 let transp = TokioTcpConfig::new().upgrade(upgrade::Version::V1).authenticate(NoiseConfig::xx(auth_keys).into_authenticated()).multiplex(mplex::MplexConfig::new()).boxed();

 let mut behaviour = Behaviour { floodsub: Floodsub::new(peer.clone()), mdns: Mdns::new(Default::default()).await.expect("can create mdns"), peer, sender, text };

 let topic: Topic = Topic::new("text");

 behaviour.floodsub.subscribe(topic.clone());

 let mut swarm = SwarmBuilder::new(transp, behaviour, peer.clone()).executor(Box::new(|fut| { tokio::spawn(fut); })).build();

 let mut stdin = tokio::io::BufReader::new(tokio::io::stdin()).lines();

 Swarm::listen_on(&mut swarm, "/ip4/0.0.0.0/tcp/0".parse().expect("can get a local socket")).expect("swarm can be started");

 println!("peer.to_string(): {:?}", peer.to_string()); thread::sleep(time::Duration::from_millis(*DELAY));

 loop {
  let option = { 
   tokio::select! { event    = swarm.select_next_some() => { match event { SwarmEvent::NewListenAddr { address, .. } => { Some(EventType::Address(address)) } _ => None } }
                  , line     = stdin.next_line()        => Some(EventType::Input(line.expect("can get line").expect("can read line from stdin")))
                  , response = recipient.recv()         => Some(EventType::Response(response.expect("response exists")))
                  }
  };//let option = { 

  if let Some(event) = option {
   match event {
    EventType::Address(address) => {
     println!("address: {:?}", address); thread::sleep(time::Duration::from_millis(*DELAY));

    }//EventType::Address(address) => {

    EventType::Input(line) => {
     println!("EventType::Input(line): {:?}", line); thread::sleep(time::Duration::from_millis(*DELAY));

     match &line[..] { "exit" => break
                     , "size" => println!("swarm.behaviour().mdns.discovered_nodes().len(): {:?}", swarm.behaviour().mdns.discovered_nodes().len())
                     , _      => swarm.behaviour_mut().floodsub.publish(topic.clone(), serde_json::to_string(&Request { destination: line }).expect("can jsonify request").as_bytes())
     }//match &line[..] {

    }//EventType::Input(line) => {

    EventType::Response(response) => {
     println!("EventType::Response(response): {:?}", response); thread::sleep(time::Duration::from_millis(*DELAY));

     let json = serde_json::to_string(&response).expect("can jsonify response");

     swarm.behaviour_mut().floodsub.publish(topic.clone(), json.as_bytes());
    }//EventType::Response(response) => {
   }//match event {
  }//if let Some(event) = option {
 }//loop {
}//async fn main() {
