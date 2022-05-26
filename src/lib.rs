use serde::{Serialize, Deserialize};
use chrono::Utc;
use log::{error, warn, info};
use sha2::{Sha256, Digest};

pub struct App {
    pub blocks: Vec<Block>
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Block{
    pub id: u64,  // 0부터 점점 올라감
    pub hash: String,  // sha256 hash
    pub previous_hash: String,
    pub timestamp: i64,
    pub data: String,
    pub nonce: u64
}

const DIFFICULTY_PREFIX: &str = "00";


fn hash_to_binary_representation(hash: &[u8]) -> String{
    let mut res: String = String::default();
    for c in hash{
        res.push_str(&format!("{:b}", c));
    }
    
    res
}
impl App {
    fn new() -> Self{
        Self {blocks: vec![]}
    }

    // 최초의 블록을 하드코딩으로 만드는 작업
    fn genesis(&mut self){
        let genesis_block = Block {
            id: 0,
            timestamp: Utc::now().timestamp(),
            previous_hash: String::from("genesis"),
            data: String::from("genesis!"),
            nonce: 2836,
            hash : "0000f816a87f806bb0073dcf026a64fb40c946b5abee2573702828694d5b4c43".to_string(),

        };
        self.blocks.push( genesis_block );
    }

    // 블록을 추가하는 함수
    // error handling을 대충해서 실제로는 문제가 있어 쓸 수 없음
    fn try_add_block(&mut self, block: Block){
        let latest_block = self.blocks.last()
                                    .expect("there's at least one block");

        if self.is_block_valid(&block, latest_block){
            self.blocks.push(block);
        }
        else{
            error!("could not add block - invalid");
        }

    }

    fn is_block_valid(&self, block: &Block, previous_block: &Block) -> bool {
        if block.previous_hash != previous_block.hash {
            // block의 prev hash가 맞는지 확인함.
            warn!("block with id: {} has wrong previous hash", block.id);
            return false;
        } else if !hash_to_binary_representation(
            // difficulty를 만족하는지 확인함, 실제로는 더 빠른 방법이 있다고 함?
            &hex::decode(&block.hash).expect("can decode from hex")
        ).starts_with(DIFFICULTY_PREFIX){
            warn!("block with id: {} has invalid difficulty", block.id);
            return false;
        }else if block.id != previous_block.id + 1{
            warn!("block with id: {} is not the next block after the latest : {}"
            , block.id, previous_block.id);
            return false;
        } else if hex::encode(calculate_hash(
            block.id,
            block.timestamp,
            &block.previous_hash,
            &block.data,
            block.nonce,

        )) != block.hash{
            warn!("block with id: {} has invalid hash", block.id);
            return false;
        }

        true

    }

    // 어떤 블록이 들어갈지 애매한 경우 (동시에 오는경우) 를 처리하기 위해서
    // consensus가 필요한 것이다. 이렇게 짤리면 보통 다음에 이득을 주지만 이번에는
    // 그런 거 없다

    fn is_chain_valid(&self, chain: &[Block]) -> bool {
        for i in 0..chain.len() {
            if i == 0 {
                continue;
            }
            let first = chain.get(i-1).expect("has to exist");
            let second = chain.get(i).expect("has to exist");
            if !self.is_block_valid(second, first){
                return false;
            }
        }

        true
    }

    // consensus에 따르면 어떤 chain이 나은지 확인하는 것이다.
    // 보통 difficulty 등등을 기준으로 보지만 우리는 길이로만 본다.
    fn choose_chain(&mut self, local: Vec<Block>, remote: Vec<Block>) -> Vec<Block>{
        let is_local_valid = self.is_chain_valid(&local);
        let is_remote_valid = self.is_chain_valid(&remote);


        if is_local_valid && is_remote_valid {
            if local.len() >= remote.len(){
                local
            } else{
                remote
            }
            
        }
        else if is_remote_valid && !is_local_valid{
            remote
        }
        else if is_local_valid && !is_remote_valid {
            local
        } else {
            panic!("local and remote chains are both invalid.");
        }

    }

}

// Mining을 이제 만들어야 한다.
impl Block {
    pub fn new(id: u64, previous_hash: String, data: String) -> Self{
        let now = Utc::now();
        let (nonce, hash) = mine_block(id, now.timestamp(), &previous_hash, &data);

        Self { id: id, hash: hash, previous_hash: previous_hash, timestamp: now.timestamp(), data: data, nonce: nonce }
    }
    
}


// 실제로 블록을 mine해서 nonce와 hash를 만드는 함수
fn mine_block(id: u64, timestamp: i64, previous_hash: &str, data: &str) -> (u64,String ){
    info!("mining block ...");

    let mut nonce = 0;

    loop {

        // 그냥 잘 보기 위함
        if nonce % 100000 == 0 {
            info!("nonce : {}", nonce);
        }

        let hash = calculate_hash(id, timestamp, previous_hash, data, nonce);
        let binary_hash = hash_to_binary_representation(&hash);
        if binary_hash.starts_with(DIFFICULTY_PREFIX){
            info!("mined! nonce : {}, hash : {}, binary_hash : {}", nonce, hex::encode(&hash), binary_hash);

            return (nonce, hex::encode(hash));
        }

        nonce += 1;

    }
}

//주어진 정보로 hash를 계산하는 방법
fn calculate_hash(id: u64, timestamp: i64, previous_hash: &str, data: &str, nonce: u64) -> Vec<u8> {
    let data = serde_json::json!( {
        "id": id,
        "previous_data": previous_hash,
        "data": data,
        "timestamp": timestamp,
        "nonce": nonce
    });

    let mut hasher = Sha256::new();
    hasher.update(data.to_string().as_bytes());
    // hasher.finalize().as_slice().to_owned()
    hasher.finalize()[..].to_owned()


}