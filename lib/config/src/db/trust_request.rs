use crate::db::identity::Identity;
use crate::db::trust::Trust;
use crate::db::{NodeDB, IncomingPost, TrustRequest, construct_path_msg, Node};
use crate::db::handle_post::HandlePost;
use crate::db::score::Score;

pub trait HandleBlessing {
    fn construct_blessing(&self, post: &IncomingPost) -> Result<TrustRequest, Box<dyn std::error::Error>>;
    fn check_blessing(&self, trust_request: TrustRequest, from:&Node) -> Result<(), Box<dyn std::error::Error>>;
}

const MAX_PEERS:usize = 32;


impl HandleBlessing for NodeDB {
    fn construct_blessing(&self, post: &IncomingPost) -> Result<TrustRequest, Box<dyn std::error::Error>> {

        if post.history.len() < 2 {
            return Err("Post does not contain enough history (we are already directly trusted or node history was tampered)")?;
        }

        let mut history = post.history.clone();
        history.reverse();

        let given_to_us = history.get(0).ok_or("path history too short (we received it from a node that does not wish to share secondary peers")?;
        let given_to_inter = history.get(1).ok_or("path history too short (we are already directly connected to author)")?;
        

        if self.is_trusted(&given_to_inter.from)? {
            return Err("Already trusted")?;
        }

        let intermediate = given_to_inter.to.clone();
        return Ok(TrustRequest{
            recipient: given_to_inter.from.clone(),
            intermediate: intermediate,
            post: post.get_id(),
            signature: given_to_us.signature.clone()
        });

    }

    fn check_blessing(&self, trust_request: TrustRequest, from:&Node) -> Result<(), Box<dyn std::error::Error>> {
        
        let us = self.get_identity()?;

        if us.node.public_key == from.public_key {
            return Err("Tried to accept a trust request from ourself")?;
        }

        if us.node.public_key == trust_request.intermediate.public_key {
            return Err("Trust request used us as the intermediate node")?;
        }

        if !self.is_trusted(&trust_request.intermediate)? {
            return Err("Trust referenced untrusted intermediate node")?;
        }

        if !self.has_seen(&us.node, &trust_request.post)? {
            return Err("Trust referenced an unknown post")?;
        }

        if !self.has_seen(&trust_request.intermediate, &trust_request.post)? {
            return Err("Trust request referenced post that we did not send to the intermediate node")?;
        }

        let message = construct_path_msg(&trust_request.post, &trust_request.intermediate, &from);
        if trust_request.intermediate.verify(&message, &trust_request.signature).is_err() {
            return Err("Trust request cannot prove that they received post from intermediate node (signature failed)")?;
        }

        let mut trusted_nodes = self.get_trusted()?;

        if trusted_nodes.len() > MAX_PEERS {

            // Kick worst peer if from is better
            let from_score = self.get_score(&from, self.get_score(&trust_request.intermediate, 1200)?)?;

            trusted_nodes.sort_by(|(_node_a, score_a), (_node_b, score_b)| score_a.partial_cmp(score_b).unwrap());

            let first_node = trusted_nodes.get(0);
            if let Some((worst_trust, worst_score)) = first_node {
                if *worst_score < from_score {
                    self.untrust(worst_trust)?;
                } else {
                    Err("candidate node was not good enough to kick worst trusted node (too many peers)")?;
                }
            }

        } else {
            self.trust(&from)?;
        }

        
        Ok(())
    }

}

#[test]
fn test_trust_request() -> Result<(), Box<dyn std::error::Error>> {
    use crate::db::RawPost;

    let db1 = NodeDB::new(tempfile::TempDir::new()?, None)?;
    let node1 = db1.get_identity()?;

    let db2 = NodeDB::new(tempfile::TempDir::new()?, None)?;
    let node2 = db2.get_identity()?;

    let db3 = NodeDB::new(tempfile::TempDir::new()?, None)?;
    let node3 = db3.get_identity()?;

    // This test will try and get node1 to trust node3 via a trust request.
    db1.trust(&node2.node)?;
    db2.trust(&node3.node)?;

    // Source post to build history upon
    let raw_post = RawPost::new(node1.node.clone(), "".to_string());
    let signature = node1.sign(&raw_post.get_id().raw.to_vec());
    let post = IncomingPost::new(
        &raw_post, 
        &vec![],
        &signature,
        &node1
    )?;

    // Node1 sending post to Node2
    let out = db1.receive(&post)?;
    let out_post = out.last().expect("author did not send any posts").clone();

    // Node2 receiving post from Node1, then sending it to Node3
    let in_post = IncomingPost::new(&out_post.post, &out_post.history, &out_post.signature, &node2)?;
    let out = db2.receive(&in_post)?;
    let out_post = out.last().expect("author did not send any posts").clone();

    // Node3 receiving post from Node2
    let in_post = IncomingPost::new(&out_post.post, &out_post.history, &out_post.signature, &node3)?;
    let _out = db3.receive(&in_post)?;

    // Node3 creating trust request
    let blessing = db3.construct_blessing(&in_post)?;

    // Node3 proves to Node1 that Node3 received a post from Node2
    assert_eq!(db1.is_trusted(&node3.node)?, false);
    db1.check_blessing(blessing, &node3.node)?; 
    assert_eq!(db1.is_trusted(&node3.node)?, true);

    Ok(())
}
