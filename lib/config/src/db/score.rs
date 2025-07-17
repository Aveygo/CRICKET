use tempfile::tempfile;

use crate::db::{NodeDB, Node, PostId, TrustRequest};
// use crate::misc::get_epoch;
use crate::db::identity::Identity;
use crate::db::handle_post::HandlePost;
use crate::db::trust_request::HandleBlessing;

use super::trust::Trust;
use super::IncomingPost;

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

pub trait Score {
    fn set_score(&self, node:&Node, value:usize) -> Result<(), Box<dyn std::error::Error>>;
    fn update_scores(&self, promote_us:bool, post: &IncomingPost) -> Result<Option<RecommendedAction>, Box<dyn std::error::Error>>;
    fn promote(&self, post: &PostId) -> Result<Option<TrustRequest>, Box<dyn std::error::Error>>;
    fn demote(&self, post: &PostId) -> Result<Option<TrustRequest>, Box<dyn std::error::Error>>;
    fn get_score(&self, node: &Node, default_score:usize) -> Result<usize, Box<dyn std::error::Error>>;
}

const SCORES_TABLE:&str = "SCORE_TABLE";

pub enum RecommendedAction {
    Trust(TrustRequest),
    Distrust
}

impl Score for NodeDB {

    fn set_score(&self, node:&Node, value:usize) -> Result<(), Box<dyn std::error::Error>> {
        let scores = self.db.open_tree(SCORES_TABLE)?;
        scores.insert(&node.public_key, bincode::serialize(&value)?)?;
        return Ok(());
    }
    
    fn update_scores(&self, promote_us:bool, post: &IncomingPost) -> Result<Option<RecommendedAction>, Box<dyn std::error::Error>> {
        let us = self.get_identity()?;
        let author = post.post.author.clone();

        if author == us.node {
            return Err("Cannot promote our own post")?;
        }

        let mut our_score = self.get_score(&us.node, 1200)?;
        let mut their_score = self.get_score(&author, 1200)?;

        if promote_us {
            (our_score, their_score) = calculate_new_elo(our_score, their_score);
        } else {
            (their_score, our_score) = calculate_new_elo(their_score, our_score);
        }
        
        self.set_score(&us.node, our_score)?;
        self.set_score(&author, their_score)?;

        if calculate_p_win(their_score, our_score) > 0.5 + 0.1 {
            let blessing = self.construct_blessing(post)?;
            return Ok(Some(RecommendedAction::Trust(blessing)));
        }

        if calculate_p_win(their_score, our_score) > 0.5 - 0.1 {
            return Ok(Some(RecommendedAction::Distrust));
        }



        Ok(None)
    }

    fn promote(&self, post_id: &PostId) -> Result<Option<TrustRequest>, Box<dyn std::error::Error>> {
        let post =self.resolve(post_id)?;

        let action = self.update_scores(false, &post)?;

        match action {
            Some(RecommendedAction::Trust(blessing)) => {
                Ok(Some(blessing))
            },
            _ => {Ok(None)}
        }
    }
    
    fn demote(&self, post: &PostId) -> Result<Option<TrustRequest>, Box<dyn std::error::Error>> {
        let post = self.resolve(post)?;
        let action = self.update_scores(true, &post)?;

        match action {
            Some(RecommendedAction::Distrust) => {
                self.untrust(&post.post.author)?
            },
            _ => {}
        }

        Ok(None)

    }

    fn get_score(&self, node: &Node, default_score:usize) -> Result<usize, Box<dyn std::error::Error>> {
        let scores = self.db.open_tree(SCORES_TABLE)?;
        let score = scores.get(&node.public_key)?;

        if let Some(score) = score {
            let score:usize = bincode::deserialize(&score)?;
            return Ok(score);
        }
        
        return Ok(default_score); //Err("User was not scored yet - need to accept a blessing from them first")?;
    }
}

#[test]
fn basic_scoring() -> Result<(), Box<dyn std::error::Error>> {
    let db = NodeDB::new(tempfile::TempDir::new()?, None)?;
    let node = Node::new([0u8; 32]);
    assert_eq!(db.get_score(&node, 1200)?, 1200);
    db.set_score(&node, 1000)?;
    assert_eq!(db.get_score(&node, 1200)?, 1000);
    Ok(())
}

#[test]
fn promote_test() -> Result<(), Box<dyn std::error::Error>> {
    use crate::db::RawPost;
    let db = NodeDB::new(tempfile::TempDir::new()?, None)?;
    let us = db.get_identity()?;
    let author = db.generate_identity()?;

    let raw_post = RawPost::new(author.node.clone(), "".to_string());
    let signature = author.sign(&raw_post.get_id().raw.to_vec());
    let post = IncomingPost::new(
        &raw_post, 
        &vec![],
        &signature,
        &us
    )?;    

    assert_eq!(db.get_score(&us.node, 1200)?, 1200);
    assert_eq!(db.get_score(&author.node, 1200)?, 1200);
    
    db.receive(&post)?;
    db.promote(&post.get_id())?;

    assert_eq!(db.get_score(&us.node, 1200)? < 1200, true);
    assert_eq!(db.get_score(&author.node, 1200)? > 1200, true);

    Ok(())
}