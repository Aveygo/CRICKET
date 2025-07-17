
use crate::db::{identity::Identity, trust::Trust, IncomingPost, NodeDB, OutgoingPost, PostId, Node};
use crate::misc::get_epoch;

pub trait HandlePost {
    fn resolve(&self, post: &PostId) -> Result<IncomingPost, Box<dyn std::error::Error>>;
    fn receive(&self, post: &IncomingPost) -> Result<Vec<OutgoingPost>, Box<dyn std::error::Error>>;
    fn has_seen(&self, node: &Node, post:&PostId) -> Result<bool, Box<dyn std::error::Error>>;
    fn register_seen(&self, node: &Node, post:&PostId) -> Result<(), Box<dyn std::error::Error>>;
}

pub const SEEN_TABLE:&str = "SEEN_TABLE";
pub const POSTS_TABLE:&str = "POSTS_TABLE";

impl HandlePost for NodeDB {
    fn resolve(&self, post: &PostId) -> Result<IncomingPost, Box<dyn std::error::Error>> {
        let posts = self.db.open_tree(POSTS_TABLE)?;
        let raw_post = posts.get(post.raw)?.ok_or("Could not find post")?;
        let post:IncomingPost = bincode::deserialize(&raw_post)?;
        Ok(post)
    }

    fn receive(&self, post: &IncomingPost) -> Result<Vec<OutgoingPost>, Box<dyn std::error::Error>> {
        let posts = self.db.open_tree(POSTS_TABLE)?;
        let us = self.get_identity()?;

        // Make sure this post hasn't been given to us already
        if self.has_seen(&us.node,&post.get_id() )? {
            return Err("We have already seen this post")?;
        }
        self.register_seen(&us.node, &post.get_id())?;

        // register seen for each node in history
        for node_pth in &post.history {
            self.register_seen(&node_pth.from, &post.get_id())?;
        }

        // Insert the post into the database for future fetching / searching
        posts.insert(post.get_id().raw, bincode::serialize(&post)?)?;

        // Get our peers and add our signature to confirm that we sent it to them 
        let trusted_nodes = self.get_trusted()?;
        let mut result = vec![];
        for (node, _score) in trusted_nodes {
            
            if !self.has_seen(&node, &post.get_id()).unwrap() {
                // Register seen for future trust requests 
                self.register_seen(&node, &post.get_id())?;

                let outgoing_post = OutgoingPost::from_incoming(post, &us, &node);
                result.push(outgoing_post);
            }
        }

        Ok(result)
    }

    fn has_seen(&self, node: &Node, post:&PostId) -> Result<bool, Box<dyn std::error::Error>>{
        let seen = self.db.open_tree(SEEN_TABLE)?;
        let key = [node.public_key, post.raw].concat();
        Ok(seen.contains_key(key)?)
    }

    fn register_seen(&self, node: &Node, post:&PostId) -> Result<(), Box<dyn std::error::Error>> {
        let seen = self.db.open_tree(SEEN_TABLE)?;
        let key = [node.public_key, post.raw].concat();
        let time = get_epoch();
        seen.insert(key, bincode::serialize(&time)?)?;
        Ok(())
    }

}

#[test]
fn check_seen() -> Result<(), Box<dyn std::error::Error>> {
    use crate::db::{Us, RawPost};

    let db = NodeDB::new(tempfile::TempDir::new()?, None)?;
    let us = db.get_identity()?;
    let raw_post = RawPost::new(us.node.clone(),"".to_string());
    let signature = us.sign(&raw_post.get_id().raw.to_vec());
    let post = IncomingPost::new(
        &raw_post, 
        &vec![],
        &signature,
        &us
    )?;

    let result = db.receive(&post)?;
    let built_post = db.resolve(&post.get_id())?;

    assert_eq!(built_post, post);
    
    Ok(())
}