use data_encoding::HEXUPPER;
use log::{debug, info};
use ring::digest::{Context, SHA256};
use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Transaction {
    sender: String,
    recipient: String,
    amount: f64,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Block {
    pub index: u32,
    pub timestamp: SystemTime,
    pub transactions: Vec<Transaction>,
    pub proof: u64,
    pub previous_hash: String,
}

impl Block {
    fn new(previous_txs: Vec<Transaction>, index: u32, previous_hash: String, proof: u64) -> Self {
        Block {
            previous_hash,
            proof,
            timestamp: SystemTime::now(),
            index,
            transactions: previous_txs,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Blockchain {
    pub node_id: Uuid,
    pub nodes: Vec<String>, // Vector of URL-like addresses
    pub chain: Vec<Block>,
    pub current_transactions: Vec<Transaction>,
}

impl Blockchain {
    pub fn new() -> Self {
        let mut blockchain = Blockchain {
            node_id: Uuid::new_v4(),
            chain: Vec::new(),
            current_transactions: Vec::new(),
            nodes: Vec::new(),
        };

        // Genesis block
        blockchain.new_block(100, Some("1".to_string()));

        return blockchain;
    }

    pub fn register_node(&mut self, addr: String) {
        // TODO: Add address validation.
        if !self.nodes.contains(&addr) {
            self.nodes.push(addr);
        }
    }

    pub fn new_block(&mut self, proof: u64, previous_hash: Option<String>) -> Block {
        let hash = match previous_hash {
            Some(h) => h,
            None => {
                let last_block = self.last_block().unwrap().clone();
                self.hash_block(&last_block) // Hash last block
            }
        };

        let block = Block::new(
            self.current_transactions.clone(),
            self.chain.len() as u32 + 1,
            hash,
            proof,
        );

        // Reset current transactions
        self.current_transactions.clear();
        self.chain.push(block.clone());
        block
    }

    pub fn new_transaction(&mut self, sender: String, recipient: String, amount: f64) -> u32 {
        let tx = Transaction {
            sender,
            recipient,
            amount,
        };

        self.current_transactions.push(tx);

        // Return index of the next block to be mined. Which will be the block
        // Containing this transaction.
        match self.last_block() {
            None => 1,
            Some(b) => b.index + 1,
        }
    }

    fn hash(&self, b: Vec<u8>) -> String {
        let mut context = Context::new(&SHA256);

        context.update(&b);

        let digest = context.finish();
        HEXUPPER.encode(digest.as_ref())
    }

    pub fn hash_block(&self, block: &Block) -> String {
        let bytes = bincode::serialize(&block).unwrap();
        self.hash(bytes)
    }

    // Simple Proof of Work Algorithm:
    //  - Find a number p' such that hash(pp') contains leading 4 zeroes [1], where p is the previous p'
    //  - p is the previous proof, and p' is the new proof
    // [1]: We're fixing it at 4 to make things simple. Bitcoin uses a dynamic number instead of a fixed one.
    pub fn proof_of_work(&self, last_block: Block) -> u64 {
        let last_proof = last_block.proof;
        let last_hash = self.hash_block(&last_block);

        let mut proof: u64 = 0;
        while self.valid_proof(last_proof, proof, &last_hash) != true {
            proof += 1;
        }
        proof
    }

    pub fn last_block(&self) -> Option<&Block> {
        self.chain.last()
    }

    pub fn mine(&mut self) -> Block {
        let last_block = self.last_block().unwrap().clone();

        // Mine next block by doing the proof of work
        let next_block_proof = self.proof_of_work(last_block.clone());
        info!("next_block_proof: {:?}", next_block_proof);

        // Get reward for doing the proof of work
        let node_id = self.node_id.clone().to_string();
        self.new_transaction("0".to_string(), node_id, 1.0);

        let previous_hash = self.hash_block(&last_block.clone());

        self.new_block(next_block_proof, Some(previous_hash))
    }

    // Validates the Proof: Does hash(last_proof, proof, last_hash) contain 4 leading zeroes?
    // Pretty sure there's some optimizations to be done here.
    fn valid_proof(&self, last_proof: u64, proof: u64, last_block_hash: &str) -> bool {
        let guess = format!("{}{}{}", last_proof, proof, last_block_hash);

        let hashed_guess = self.hash(guess.into_bytes());

        let last_four = hashed_guess
            .char_indices()
            .rev()
            .nth(3) // TODO: move this magic number to a configurable variable.
            .map(|(i, _)| &hashed_guess[i..])
            .unwrap();

        // Currently logging this to stdout because it's so cool to watch it guessing.
        println!("{:?}", hashed_guess);

        // TODO: This can be much better, consigurable, and dynamic.
        if last_four == "0000" {
            return true;
        }

        false
    }

    // Check if a blockchain is valid.
    // This is used to compare a chain that's longer than this node's chain.
    // In this case, we only compare the chain up to this node's chain length
    // i.e `peer_chain[0..self.chain.len()]`
    pub fn is_chain_valid(&self, peer_chain: Vec<Block>) -> bool {
        let is_valid: Result<Vec<_>, _> = peer_chain[0..self.chain.len()]
            // Get each adjacent pair of the chain
            .windows(2)
            // Check that each adjacent pair is valid
            .map(|block_pair| self.valid_adjacent_blocks(&block_pair[0], &block_pair[1]))
            .collect();

        match is_valid {
            Ok(_) => return true,
            Err(_) => return false,
        }
    }

    // Checks if two adjacent blocks are valid, i.e they have the correct hash
    // and the proof of work from the previous to the current block is correct.
    // This is used to validate the whole chain, see [`is_chain_valid`].
    fn valid_adjacent_blocks(&self, previous: &Block, current: &Block) -> Result<(), bool> {
        let last_block_hash = self.hash_block(previous);

        if current.previous_hash != last_block_hash {
            debug!("invalid chain: different hashes");
            return Err(false);
        }

        if !self.valid_proof(previous.proof, current.proof, &last_block_hash) {
            debug!("invalid chain: proof invalid");
            return Err(false);
        }

        Ok(())
    }

    pub fn resolve_conflicts(&mut self) -> bool {
        let mut new_chain: Vec<Block> = Vec::new();

        // Because we're looking for chains that are longer than this node's
        // we'll use this node's chain length.
        let mut max_length = self.chain.len();

        for node in &self.nodes {
            // Get the chain from a neighbour over HTTP.
            // Ideally this should be over RPC.
            let url = format!("{}/chain", node);
            let chain: Vec<Block> = reqwest::blocking::get(url).unwrap().json().unwrap();

            if chain.len() > max_length && self.is_chain_valid(chain.clone()) {
                max_length = chain.len();
                new_chain = chain;
            }
        }

        // We found a chain that's bigger than this node's.
        if new_chain.len() > 0 {
            info!("chain replaced");
            self.chain = new_chain;
            return true;
        }

        info!("chain not replaced");
        return false;
    }
}

#[cfg(test)]
mod tests {
    // use super::*;

    // TODO: I'd like to add way more tests :)
    // #[test]
    // fn creates_new_blockchain() {
    //     let blockchain = Blockchain::new();
    // }
}
