use std::str::FromStr;
use sled::Db;
use serde::{Serialize, Deserialize};
use crate::misc::{get_epoch, sha256};
use rand::Rng;

pub trait Hashable: Serialize {
    fn hash(&self) -> [u8; 32] {
        let serialized = bincode::serialize(self).unwrap();
        sha256(serialized)
    }
}


#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct Node {
    pub public_key: [u8; 32],
}

impl Node {
    pub fn new(public_key: [u8; 32]) -> Self {
        Self {
            public_key: public_key,
        }
    }

    fn verify(&self, message: &[u8; 32], signature:&str) -> Result<(), Box<dyn std::error::Error>> {
        let public_key = iroh::PublicKey::from_bytes(&self.public_key)?;
        public_key.verify(message, &ed25519::Signature::from_str(signature)?)?;
        Ok(())
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct Us {
    pub node: Node,
    pub private_key: [u8; 32]
}

impl Us {
    fn new(private_key: [u8; 32]) -> Self {
        let secret_key = iroh::SecretKey::from_bytes(&private_key);
        return Us {
            node: Node::new(secret_key.public().as_bytes().clone()),
            private_key: private_key
        };

    }

    pub fn sign(&self, content: &[u8]) -> String {
        let secret_key = iroh::SecretKey::from_bytes(&self.private_key);
        let signature = secret_key.sign(content);
        return signature.to_string();
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct RawPost {
    pub author: Node,
    pub content: String,
    pub message_id: u128,
}

impl RawPost {
    pub fn new(author: Node, content: String) -> Self {

        let message_id:u128 = rand::rng().random();
        Self {
            author,
            content,
            message_id
        }

    }

    pub fn get_id(&self) -> PostId {
        PostId { raw: self.hash() }
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct Path {
    pub from: Node,
    pub to: Node,
    pub signature: String, // sign(post + from + to, from private key)
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct IncomingPost {
    pub post: RawPost,
    pub history: Vec<Path>,
    pub received: u64,
    pub signature: String // sign(post.hash(), author private key)
}


impl Hashable for IncomingPost {}


impl IncomingPost {

    // Need to have a method that allows us to create a post to send to the network

    pub fn new(post:&RawPost, history: &Vec<Path>, signature:&String, us: &Us) -> Result<Self, Box<dyn std::error::Error>> {
        IncomingPost::verify_history(&history, &post, &us)?;
        IncomingPost::verify_signature(&post, signature)?;

        Ok(IncomingPost {
            post: post.clone(),
            history: history.clone(),
            received: get_epoch(),
            signature: signature.clone()
        })

    }
    fn verify_signature(post: &RawPost, signature:&String) -> Result<(), Box<dyn std::error::Error>> {
        post.author.verify(&post.hash(), signature)?;
        Ok(())
    }

    fn verify_history(history: &Vec<Path>, post: &RawPost, us: &Us) -> Result<(), Box<dyn std::error::Error>> {
        let post_id = PostId { raw: post.hash() };
        for (idx, path) in history.iter().enumerate() {
            let message = construct_path_msg(&post_id, &path.from, &path.to);
            path.from.verify(&message, &path.signature)?;

            if idx < history.len() - 1 {
                let next_node = history.get(idx + 1).unwrap();
                if path.to.public_key != next_node.from.public_key {
                    Err("History contained broken chain")?;
                }
            }
        }

        if let Some(last) = history.last() { // Might receive an empty history
            if last.to.public_key != us.node.public_key {
                Err("We got a post that was not intended for us")?;
            }
        }

        Ok(())
    }

    fn get_id(&self) -> PostId {
        self.post.get_id()
    }
}


#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct OutgoingPost {
    pub post: RawPost,
    pub history: Vec<Path>, // modified to include us, but only two degrees
    pub signature: String // sign(post.hash(), author private key)
}

fn construct_path_msg(post: &PostId, from: &Node, to:&Node) -> [u8; 32] {
    sha256([post.raw, from.public_key, to.public_key].concat())
}

impl OutgoingPost {
    fn from_incoming(post:&IncomingPost, us: &Us, to:&Node) -> Self {
        
        let intermediate_node = post.history.last();
        
        let mut history = vec![];
        if let Some(intermediate_node) = intermediate_node {
            history.push(intermediate_node.clone())
        }

        let message = construct_path_msg(&post.get_id(), &us.node, to);
        let signature = us.sign(&message);
        
        history.push(Path { from: us.node.clone(), to: to.clone(), signature: signature });

        OutgoingPost {
            post: post.post.clone(),
            history: history,
            signature: post.signature.clone()
        }
    }

    
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct PostId {
    raw: [u8; 32]
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
struct TrustRequest {
    recipient: Node,
    intermediate: Node,
    post: PostId,       // Proof that we share the intermediate node
    signature: String   // proof that the intermediate node gave us the post 
                        // sign(post_id + us public key, intermediate private key)
}


impl Hashable for RawPost {}

pub struct NodeDB {
    pub db: Db,
    pub bootstrap_nodes: Option<Vec<Node>>
}

impl NodeDB {
    pub fn new<P: AsRef<std::path::Path>>(path: P, bootstrap_nodes:Option<Vec<Node>>) -> Result<Self, Box<dyn std::error::Error>> {
        let db = sled::open(path)?;
        
        Ok(Self {
            db: db,
            bootstrap_nodes: bootstrap_nodes
        })
    }
}

pub mod identity;
pub mod trust_request;

pub mod handle_post;

pub mod trust;
pub mod score;
pub mod search;