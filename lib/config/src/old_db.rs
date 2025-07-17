use sled::Db;
use serde::{Serialize, Deserialize};
use std::str::FromStr;
use bincode;
use sha2::{Digest, Sha256};
use std::time::SystemTime;

use iroh::PublicKey;
use ed25519;

#[derive(Debug, PartialEq, Serialize)]
struct User {
    private_key:[u8; 32]
}

impl User {
    fn get_public_key(&self) -> String {
        let secret_key = iroh::SecretKey::from_bytes(&self.private_key);
        secret_key.public().to_string()
    }

    fn sign(&self, content: &[u8]) -> String {
        let secret_key = iroh::SecretKey::from_bytes(&self.private_key);
        let signature = secret_key.sign(content);
        return signature.to_string();
    }

    fn verify(&self, message: Vec<u8>, signature:&str) -> Result<bool, Box<dyn std::error::Error>> {
        let public_key = PublicKey::from_str(&self.get_public_key())?;
        public_key.verify(&message, &ed25519::Signature::from_str(signature)?)?;
        return Ok(true);
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Blessing {
    public_key: String,
    post_hash: [u8; 32],
    signature: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
struct Post {
    author: String,
    content: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
struct SeenBy {
    public_key: String,
    sent_to_public_key: String,
    signature: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
struct SignedPost {
    post: Post,
    signature: String,
    timestamp: u128,
    from_public_key: String,
    history: Vec<SeenBy>
}

fn sha256(serialized_data:Vec<u8>) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(&serialized_data);
    let result = hasher.finalize();
    result.into()
}

pub trait Hashable: Serialize {
    fn hash(&self) -> [u8; 32] {
        let serialized = bincode::serialize(self).unwrap();
        sha256(serialized)
    }
}


impl SignedPost {
    fn new(post:Post, signature:String, from_public_key:String, node_history:Vec<SeenBy>) -> Result<Self, Box<dyn std::error::Error>> {
        let post_hash = post.hash();
        let public_key = PublicKey::from_str(&post.author)?;
        public_key.verify(&post_hash, &ed25519::Signature::from_str(&signature)?)?;

        let duration_since_epoch = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap();
        let timestamp_nanos = duration_since_epoch.as_nanos();
        SignedPost::verify_history(&node_history, &post)?;

        return Ok(Self { 
            post: post, 
            signature: signature, 
            timestamp: timestamp_nanos, 
            from_public_key:from_public_key, 
            history: node_history
        })
    }

    fn verify_history(node_history:&Vec<SeenBy>, post: &Post) -> Result<(), Box<dyn std::error::Error>> {
        let post_hash = post.hash();
        for (idx, node) in node_history.iter().enumerate() {

            if idx < node_history.len() -1 {
                let peak_next_node = node_history.get(idx + 1).expect("broken array?");
                if peak_next_node.public_key != node.sent_to_public_key {
                    return Err("Invalid history")?
                }
            }

            let public_key = PublicKey::from_str(&node.public_key)?;
            let message = sha256([post_hash, sha256(node.sent_to_public_key.as_bytes().to_vec())].concat());
            public_key.verify(&message, &ed25519::Signature::from_str(&node.signature)?)?;
        }

        return Ok(());
    }

    fn add_to_history(&mut self, target_public_key:String, us:&User) -> Result<(), Box<dyn std::error::Error>> {
        
        let message = sha256([self.post.hash(), sha256(target_public_key.as_bytes().to_vec())].concat());

        let signature = us.sign(&message);

        let seen_by_us = SeenBy{
            public_key: us.get_public_key(),
            sent_to_public_key: target_public_key,
            signature: signature
        };

        self.history.push(seen_by_us);

        Ok(())
    }
}

impl Hashable for Post {}
impl Hashable for SignedPost {}
impl Hashable for User {}


struct NodeDB {
    db: Db,
}

const SCORES_TABLE:&str = "scores";                         // Our score for each peer (including ourselves)
const IDENTITY_TABLE:&str = "self_identity";                // Our private key
const POSTS_TABLE:&str = "posts";                           // All posts that we have received
const HAS_SEEN_TABLE:&str = "has_seen";                     // Which peers have seen what posts from us
const POST_EPOCH_INDEX:&str = "post_epoch_index";           // Posts ordered by received time
const MAX_EXCLUSIVE_INDEX:&str = "max_exclusive_index";     // Key within post_epoch_index for number of posts, todo, replace with len?

const DEFAULT_SCORE:usize = 1200;

fn calculate_p_win(winner_rating: usize, loser_rating: usize) -> f64 {
    let (winner_rating, loser_rating) = (winner_rating as f64, loser_rating as f64);
    return 1.0 / (1.0 + 10.0f64.powf((loser_rating - winner_rating) / 400.0)); 
}

fn calculate_new_elo(winner_rating: usize, loser_rating: usize) -> (usize, usize) {
    let k = 32.0;
    let p_winner = calculate_p_win(loser_rating, winner_rating);
    let p_loser = calculate_p_win(winner_rating, loser_rating);

    let new_winner_rating = winner_rating as f64 + k * (1.0 - p_winner);
    let new_loser_rating = loser_rating as f64 + k * (0.0 - p_loser);
    (new_winner_rating as usize, new_loser_rating as usize)
}

fn u8_slice_to_f64(slice: &[u8; 32]) -> f64 {
    // Combine the u8 slice into a large integer (u128 in this case).
    let mut combined = 0u128;
    for &byte in slice {
        combined = combined << 8 | byte as u128;
    }

    // Normalize to the range [0, 1]
    let max_value = u128::MAX;
    combined as f64 / max_value as f64
}

/*

As a node on the network, we need to keep track of:
1. What we posted
2. What we have received from other nodes
3. What nodes have received which posts from us (to prevent duplicate sending)
4. The scores for each node

In addition to this, we also need to determine if we want to share a post that we received to our peers
Currently, I take a stochastic approach - we compare the score of the node that gave us the post, and the node
that we want to share the post with. We need to accept both receiving a post, and sharing it.

*/

#[allow(dead_code)]
impl NodeDB {
    // must trust ourselves and the bootstrap node
    fn new(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let db = sled::open(path)?;
        Ok(NodeDB { db })
    }

    pub fn trust(&self, public_key: &str, score:usize)  -> Result<(), Box<dyn std::error::Error>> {
        let scores = self.db.open_tree(SCORES_TABLE)?;
        scores.insert(public_key, bincode::serialize(&score)?)?;
        return Ok(());
    }

    fn construct_blessing(&self, target_public_key: &str) -> Result<Option<Blessing>, Box<dyn std::error::Error>> {
        let mut posts = self.get_posts_from(target_public_key)?;

        posts.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
        let most_recent = posts.last();

        if let Some(most_recent) = most_recent {
            
            let intermedate = most_recent.history.last();

            if let Some(intermedate) = intermedate {
                
                return Ok(Some(Blessing {
                    public_key: intermedate.public_key.clone(),
                    post_hash: most_recent.hash(),
                    signature: intermedate.signature.clone()
                }));

            }
            
        }


        todo!();
    }

    /* Returns yes on accept blessing, no for reject */
    pub fn process_blessing(&self, blessing:Blessing, from_public_key: String) -> Result<(), Box<dyn std::error::Error>> {
        
        if self.is_trusted(&from_public_key) {
            return Err("Blessing node was already trusted")?;
        }

        if !self.is_trusted(&blessing.public_key) {
            return Err("Do not trust intermedate node of blessing")?;
        }

        let us = self.get_identity()?;
        if !self.has_seen_hash(&us.get_public_key(), &blessing.post_hash)? {
            return Err("Blessing references missing post")?; 
        }

        if !self.has_seen_hash(&blessing.public_key, &blessing.post_hash)? {
            return Err("Blessing references post that we did not send to intermediate node")?; 
        }

        let message = sha256([blessing.post_hash, sha256(blessing.public_key.as_bytes().to_vec())].concat());
        us.verify(message.to_vec(), &blessing.signature)?;

        let score = self.get_score(&blessing.public_key)?;
        self.trust(&from_public_key, score)?;

        return Ok(());
    }

    /*
        When we receive a post (or want to share it), this function returns
        a vector of signed posts.
        Within each post is the node history, where the last element contains the intended recipient
        it is important to send each signed post to the intended recipient as it contains proof of intent.
     */
    pub fn received(&self, post:SignedPost, max_recipients:usize) -> Result<Vec<SignedPost>, Box<dyn std::error::Error>> {
        let us = self.get_identity()?;

        self.register_post(&post)?;

        let scores = self.db.open_tree(SCORES_TABLE)?;
        let mut results = vec![];
        for item in scores.iter().rev() {
            if let Ok((key, _value)) = item {

                let public_key:&str = bincode::deserialize(&key)?;

                if self.should_share(&post, public_key)? && results.len() < max_recipients {
                    let recipient = public_key.to_string();
                    let mut specific_post = post.clone();
                    specific_post.add_to_history(recipient, &us)?;
                    results.push(specific_post);
                }
            }
        }

        return Ok(results);

    }

    fn update_scores(&self, public_key:&str, promote_us:bool) -> Result<Option<Blessing>, Box<dyn std::error::Error>> {
        let us = self.get_identity()?;
        let mut our_score = self.get_score(&us.get_public_key())?;
        let mut their_score = self.get_score(public_key)?;

        if promote_us {
            (our_score, their_score) = calculate_new_elo(our_score, their_score);
        } else {
            (their_score, our_score) = calculate_new_elo(their_score, our_score);
        }
        
        self.set_score(&us.get_public_key(), our_score)?;
        self.set_score(public_key, their_score)?;

        // Create blessing if their score is high enough
        if calculate_p_win(their_score, our_score) > 0.68 { // 1 std, ~20 posts
            return Ok(self.construct_blessing(public_key)?);
        }

        Ok(None)
    }

    /*
        Here we received a post from a peer and we need to decide if we want to share to it to another peer we know
     */ 
    fn should_share(&self, _post:&SignedPost, target_public_key: &str) -> Result<bool, Box<dyn std::error::Error>> {
        
        // Create a 'soup' index to prevent targeted attacks
        // we don't want to be manipulated into sending specially crafted messages
        // we use our secret to prevent this
        // this does not prevent spam attacks - posts can spam 100s of messages, but their score *should* plummet

        return Ok(self.is_trusted(target_public_key));
        
        /*
        let us = self.get_identity()?;
        let us_hash:&[u8] = &us.hash()[..];
        let post_hash:&[u8] = &post.hash()[..];
        let from:&[u8] = post.from_public_key.as_bytes();
        let to:&[u8] = target_public_key.as_bytes();
        let index = [us_hash, post_hash, from, to].concat();
        
        /* Do we accept the post from the incoming peer? */
        let mut hasher = Sha256::new();
        hasher.update(&index);
        let index:[u8; 32] = hasher.finalize().into();
        let seed_prob = u8_slice_to_f64(&index);
        let p_accept_from = calulate_p_win(
            self.get_score(&post.from_public_key)?, 
            self.get_score(&us.get_public_key())?
        );

        if seed_prob > p_accept_from {
            return Ok(false);
        }

        /* Do we accept sending the post to the target peer? */
        let mut hasher = Sha256::new();
        hasher.update(&index);
        let index:[u8; 32] = hasher.finalize().into();
        let seed_prob = u8_slice_to_f64(&index);
        
        let p_accept_to = calulate_p_win(
            self.get_score(&target_public_key)?, 
            self.get_score(&us.get_public_key())?
        );

        if seed_prob > p_accept_to {
            return Ok(false);
        } 

        Ok(true)
         */
    }

    /* Increase the score of a peer */
    pub fn promote(&self, public_key:&str) -> Result<Option<Blessing>, Box<dyn std::error::Error>> {
        Ok(self.update_scores(public_key, false)?)
    }

    /* Decreate the score of a peer */
    pub fn demote(&self, public_key:&str) -> Result<Option<Blessing>, Box<dyn std::error::Error>> {
        Ok(self.update_scores(public_key, true)?)
    }

    /*
        Checks if the post has already been seen, if not then indexs it and adds
        data about author.
     */
    fn register_post(&self, signedpost:&SignedPost) -> Result<(), Box<dyn std::error::Error>> {
        let posts = self.db.open_tree(POSTS_TABLE)?;
        let scores = self.db.open_tree(SCORES_TABLE)?;

        // We only register posts from connections that are trusted
        // This process is initialised by trusting the bootstrap node
        if !self.is_trusted(&signedpost.from_public_key) {
            return Err("Tried to register a post from an untrusted user")?
        }
        
        if posts.contains_key(signedpost.hash())? {
            return Err("Post already exists in the db")?;
        };
        posts.insert(signedpost.hash(), bincode::serialize(&signedpost)?)?;
        self.index_post(&signedpost)?;

        let us = self.get_identity()?;
        self.add_to_seen(&us.get_public_key(), &signedpost.hash())?;

        todo!("Go through the history and add_to_seen for each node in the chain");
        
        if !scores.contains_key(&signedpost.post.author)? {
            self.set_score(&signedpost.post.author, DEFAULT_SCORE)?;
        };

        Ok(())
    }

    /* 
        Adds signature to the post
    */
    pub fn sign(&self, post:Post)  -> Result<SignedPost, Box<dyn std::error::Error>> {
        let user = self.get_identity()?;
        let signature = user.sign(&post.hash());
        let post = SignedPost::new(post, signature, user.get_public_key(), vec![])?;

        self.register_post(&post)?;
        return Ok(post);
    }

    /*
        Adds the post in a way that allows it to be return from the db in the order that it
        was added in
     */
    fn index_post(&self, signedpost: &SignedPost) -> Result<(), Box<dyn std::error::Error>> {
        let post_epoch_index = self.db.open_tree(POST_EPOCH_INDEX)?;

        let max_idx = post_epoch_index.get(MAX_EXCLUSIVE_INDEX)?;
        let max_idx: usize = match max_idx {
            Some(max_idx) => {
                bincode::deserialize(&max_idx)?
            }
            None => {
                post_epoch_index.insert(MAX_EXCLUSIVE_INDEX, bincode::serialize(&(0 as usize))?)?;                
                0 as usize
            }
        };

        
        let post_id = bincode::serialize(&max_idx)?;
        post_epoch_index.insert(post_id, bincode::serialize(&signedpost.hash())?)?;
        let max_id = bincode::serialize(&(max_idx + 1))?;
        post_epoch_index.insert(MAX_EXCLUSIVE_INDEX, max_id.clone())?;
        
        Ok(())
    }

    /*
        Returns if the public key has seen the post.
     */
    fn has_seen_hash(&self, public_key: &str, post_hash:&[u8; 32]) -> Result<bool, Box<dyn std::error::Error>> {
        let has_seen = self.db.open_tree(HAS_SEEN_TABLE)?;
        
        let post_hash:&[u8] = &post_hash[..];
        let index = [public_key.as_bytes(), post_hash].concat();
        let epoch = has_seen.get(&index)?;

        match epoch {
            Some(_epoch) => {
                return Ok(true); 
            },
            None => {
                return Ok(false)
            }
        }
    }

    fn get_posts_from(&self, from_public_key: &str) -> Result<Vec<SignedPost>, Box<dyn std::error::Error>> {
        // check all the posts that the public key saw & that they were the second last one in the history 
        // Need to remember that we can receive a post directly so we dont need to give them our blessing 
        // Im sorry
        todo!();
    }

    /*
        Returns if the public key has seen the post. 
     */
    fn has_seen(&self, public_key: &str, post: &SignedPost) -> Result<bool, Box<dyn std::error::Error>> {
        return self.has_seen_hash(public_key, &post.hash());
    }

    fn add_to_seen(&self, public_key: &str, post_hash:&[u8; 32]) -> Result<(), Box<dyn std::error::Error>> {
        let has_seen = self.db.open_tree(HAS_SEEN_TABLE)?;
        
        let post_hash:&[u8] = &post_hash[..];
        let index = [public_key.as_bytes(), post_hash].concat();
        let epoch = has_seen.get(&index)?;
        if let Some(_) = epoch {
            return Err("Cannot add already seen node to seen index")?
        }

        let duration_since_epoch = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap();
        let timestamp_nanos = duration_since_epoch.as_nanos();
        has_seen.insert(index, bincode::serialize(&timestamp_nanos)?)?;

        return Ok(());
    }
    

    fn get_posts(&self, max_posts: Option<usize>) -> Result<Vec<SignedPost>, Box<dyn std::error::Error>>{
        // TODO currently this functions returns oldest to newest, but it'll be better to do newest to oldest
        // this would require reversing the time index (pain).
        // Currently I use .rev() which i really hope doesnt load the entire db in memory but I might be wrong cause i spent like
        // 3 seconds of research.

        let post_epoch_index = self.db.open_tree(POST_EPOCH_INDEX)?;
        let posts = self.db.open_tree(POSTS_TABLE)?;

        let mut results = vec![];

        
        let max_idx = post_epoch_index.get(MAX_EXCLUSIVE_INDEX)?;

        match max_idx {
            None => { // no maxindex, there is not a single post
                return Ok(results);
            },
            Some(max_idx) => {

                for item in post_epoch_index.range(.. max_idx).rev() {
                    if let Ok((key, value)) = item {
                        
                        let _post_index:usize = bincode::deserialize(&key)?;
                        let value:[u8; 32] = bincode::deserialize(&value)?;

                        let raw_signed_post = posts.get(value)?.ok_or("POST_EPOCH_INDEX referenced a dropped post")?;
                        let signed_post:SignedPost = bincode::deserialize(&raw_signed_post)?;

                        results.push(signed_post);

                    }
                }
            }
        }
        
        Ok(results)

    }

    fn is_trusted(&self, public_key:&str) -> bool {
        return self.get_score(public_key).is_ok();
    }

    /*
        Manually sets the score of a node
        should only be used once a node is blessed, will reject otherwise 
     */
    fn set_score(&self, public_key:&str, value:usize) -> Result<(), Box<dyn std::error::Error>> {
        let scores = self.db.open_tree(SCORES_TABLE)?;

        // reject unknown nodes
        if !self.is_trusted(public_key) {
            return Err("Cannot set score of untrusted node")?
        }

        scores.insert(public_key, bincode::serialize(&value)?)?;
        return Ok(());
    }

    /*
        Gets the score of a node, will set and return default value if none
     */
    fn get_score(&self, public_key:&str) -> Result<usize , Box<dyn std::error::Error>> {
        let scores = self.db.open_tree(SCORES_TABLE)?;
        let score = scores.get(public_key)?;

        if let Some(score) = score {
            let score:usize = bincode::deserialize(&score)?;
            return Ok(score);
        }
        
        return Err("User was not scored yet - need to accept a blessing from them first")?;
    }

    /*
        Returns the pirvate key and other data about our node, will initialise if none
     */
    fn get_identity(&self) -> Result<User, Box<dyn std::error::Error>>{
        let identity = self.db.open_tree(IDENTITY_TABLE)?;
        let private_key = identity.get(b"private_key")?;

        let private_key:[u8; 32] = match private_key {
            Some(private_key) => {
                bincode::deserialize(&private_key)?
            },
            None => {
                let mut secret = [0u8; 32];
                rand::fill(&mut secret[..]); 
                identity.insert(b"private_key", bincode::serialize(&secret)?)?;
                secret
            }
        };

        Ok(User {private_key})
    }

    /*
        Returns the private key of our node
     */
    pub fn private_key(&self) -> Result<[u8; 32], Box<dyn std::error::Error>>{
        let us = self.get_identity()?;
        return Ok(us.private_key);
    }


}

const _TMP_DB_NAME:&str = "tmp.db";

// Use only one thread cause sled only wants one
// cargo test -- --show-output --test-threads=1

#[cfg(test)]
mod tests {
    use super::*;

    fn create_post(user: &User, content: String) -> Result<SignedPost, Box<dyn std::error::Error>> {
        let post = Post{
            author: user.get_public_key(),
            content: content
        };

        let signature = user.sign(&post.hash());
        Ok(SignedPost::new(post, signature, user.get_public_key(), vec![])?)
    }

    #[test]
    fn can_store_stuff() -> Result<(), Box<dyn std::error::Error>>{
        {
            let db = NodeDB::new(_TMP_DB_NAME)?;
            let u1 = db.get_identity()?;
            let u2 = db.get_identity()?;
            assert_eq!(u1, u2);
        }
       
        std::fs::remove_dir_all(_TMP_DB_NAME).unwrap();
        Ok(())
    }

    #[test]
    fn check_scores() -> Result<(), Box<dyn std::error::Error>>{
        {
            let db = NodeDB::new(_TMP_DB_NAME)?;
            let u1 = db.get_identity()?;

            db.set_score(&u1.get_public_key(), 1200)?;
            let score = db.get_score(&u1.get_public_key())?;

            assert_eq!(score, 1200 as usize);
        }

        std::fs::remove_dir_all(_TMP_DB_NAME).unwrap();
        Ok(())
    }

    #[test]
    fn check_signature() -> Result<(), Box<dyn std::error::Error>>{
        {
            let db = NodeDB::new(_TMP_DB_NAME)?;
            let u1 = db.get_identity()?;
            create_post(&u1, "test".to_string())?;
        }

        std::fs::remove_dir_all(_TMP_DB_NAME).unwrap();
        Ok(())
    }    
}