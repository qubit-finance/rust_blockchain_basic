use super::{App, Block};

use libp2p::{
    NetworkBehaviour,
    PeerId,
    identity,
    floodsub::{Floodsub, FloodsubEvent, Topic},
    mdns::{Mdns,MdnsEvent},
    swarm::{NetworkBehaviourEventProcess, Swarm},
};

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tokio::sync::mpsc;
use log::{error, info};
use once_cell::sync::Lazy;

pub static KEYS: Lazy<identity::Keypair> = Lazy::new(identity::Keypair::generate_ed25519);
pub static PEER_ID: Lazy<PeerId> = Lazy::new(|| PeerId::from(KEYS.public()));
pub static CHAIN_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("chains"));
pub static BLOCK_TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("blocks"));

// 누군가 보내는 양식
#[derive(Debug, Serialize, Deserialize)]
pub struct ChainResponse{
    pub blocks: Vec<Block>,
    pub receiver: String,
}


// 이걸 보내면 상대가 본인의 chain을 보내준다.
#[derive(Debug, Serialize, Deserialize)]
pub struct LocalChainRequest {
    pub from_peer_id: String
}


pub enum EventType{
    LocalChainResponse(ChainResponse),
    Input(String),
    Init,
}


#[derive(NetworkBehaviour)]
pub struct AppBehaviour {
    pub floodsub: Floodsub,
    pub mdns: Mdns,
    #[behaviour(ignore)]
    pub response_sender: mpsc::UnboundedSender<ChainResponse>,
    #[behaviour(ignore)]
    pub init_sender: mpsc::UnboundedSender<bool>,
    #[behaviour(ignore)]
    pub app: App,
}


impl AppBehaviour{
    pub async fn new(
        app: App,
        response_sender: mpsc::UnboundedSender<ChainResponse>,
        init_sender: mpsc::UnboundedSender<bool>,
    ) -> Self {
        let mut behaviour = Self {
            app: app,
            floodsub: Floodsub::new(*PEER_ID),
            mdns: Mdns::new(Default::default()).await.expect("can create mdns"),
            response_sender: response_sender,
            init_sender: init_sender,
        };

        behaviour.floodsub.subscribe(CHAIN_TOPIC.clone());
        behaviour.floodsub.subscribe(BLOCK_TOPIC.clone());

        behaviour
    }
}


impl NetworkBehaviourEventProcess<MdnsEvent> for AppBehaviour{
    fn inject_event(&mut self, event: MdnsEvent){
        match event {
            // 새롭게 발견되면 노드 추가
            MdnsEvent::Discovered(discoverd_list) => {
                for (peer, _addr) in discoverd_list{
                    self.floodsub.add_node_to_partial_view(peer);
                }
            },
            // expire되면 삭제
            MdnsEvent::Expired(expired_list) => {
                for (peer, _addr) in expired_list {
                    if !self.mdns.has_node(&peer){
                        self.floodsub.remove_node_from_partial_view(&peer);
                    }
                }
            }
        }
    }
}

//incoming event handler
impl NetworkBehaviourEventProcess<FloodsubEvent> for AppBehaviour{
    fn inject_event(&mut self, event: FloodsubEvent){
        // Message라는거는 우리가 예상한 형태로 무언가 왔다는 뜻이다.
        if let FloodsubEvent::Message(msg) = event {
            // response가 chain response인 경우
            if let Ok(resp) = serde_json::from_slice::<ChainResponse>(&msg.data){
                // 실제로 내가 받는사람인지 확인한다.
                if resp.receiver == PEER_ID.to_string(){
                    info!("Response from {}:", msg.source);
                    resp.blocks.iter().for_each(|r| info!("{:?}", r));

                    self.app.blocks = self.app.choose_chain(self.app.blocks.clone(), resp.blocks);

                }
             } //response가 local chain request인 경우
             else if let Ok(resp) = serde_json::from_slice::<LocalChainRequest>(&msg.data) {
                 info!("sending local chain to {}", msg.source.to_string());
                 let peer_id = resp.from_peer_id;
                 if PEER_ID.to_string() == peer_id {
                     // peer id가 일치하며는 reponse로 chain을 보내줌
                     if let Err(e) = self.response_sender.send(ChainResponse {
                        blocks: self.app.blocks.clone(), 
                        receiver: msg.source.to_string(),
                     }) {
                         error!("error sending response via channel, {}", e);
                     }
                 }
             }
             // block이 날아온 경우
             else if let Ok(block) = serde_json::from_slice::<Block>(&msg.data) {
                 info!("received new block from {}", msg.source.to_string());
                 self.app.try_add_block(block);
             }
            
        }
    }
}

pub fn get_list_peers(swarm: &Swarm<AppBehaviour>) -> Vec<String> {
    info!("Discovered Peers: ");
    let nodes = swarm.behaviour().mdns.discovered_nodes();
    let mut unique_peers = HashSet::new();
    for peer in nodes {
        unique_peers.insert(peer);
    }

    unique_peers.iter().map(|p| p.to_string()).collect()
}

pub fn handle_print_peers(swarm: &Swarm<AppBehaviour>){
    let peers = get_list_peers(swarm);
    peers.iter().for_each(|p| info!("{}", p));
}

pub fn handle_print_chain(swarm: &Swarm<AppBehaviour>){
    info!("Local Blockchain:");
    let pretty_json =
        serde_json::to_string_pretty(&swarm.behaviour().app.blocks).expect("can jsonify blocks");
    info!("{}", pretty_json);
}

pub fn handle_create_block(cmd: &str, swarm: &mut Swarm<AppBehaviour>) {
    if let Some(data) = cmd.strip_prefix("create b") {
        let behaviour = swarm.behaviour_mut();
        let latest_block = behaviour
            .app
            .blocks
            .last()
            .expect("there is at least one block");
        let block = Block::new(
            latest_block.id + 1,
            latest_block.hash.clone(),
            data.to_owned(),
        );
        let json = serde_json::to_string(&block).expect("can jsonify request");
        behaviour.app.blocks.push(block);
        info!("broadcasting new block");
        behaviour
            .floodsub
            .publish(BLOCK_TOPIC.clone(), json.as_bytes());
    }
}