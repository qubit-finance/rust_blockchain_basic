use serde::{Serialize, Deserialize};
use chrono::Utc;
use log::{error, warn};

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

        if self.is_valid(&block, latest_block){
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
            // difficulty를 만족하는지 확인함
            &hex::decode(&block.hash).expect("can decode from hex")
        ).starts_with(DIFFICULTY_PREFIX){
            warn!("block with id: {} has invalid difficulty", block.id);
            return false;
        }

    }

}