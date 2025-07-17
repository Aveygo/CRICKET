use crate::db::{NodeDB, Node};
use crate::misc::get_epoch;

use super::score::Score;

pub trait Trust {
    fn trust(&self, node: &Node) -> Result<(), Box<dyn std::error::Error>>;
    fn untrust(&self, node: &Node) -> Result<(), Box<dyn std::error::Error>>;
    fn is_trusted(&self, node: &Node) -> Result<bool, Box<dyn std::error::Error>>;
    fn get_trusted(&self) -> Result<Vec<(Node, usize)>, Box<dyn std::error::Error>>;
    fn num_trusted(&self) ->  Result<usize, Box<dyn std::error::Error>>;
}

// If a node is within the table, then they were trusted
// Unseen nodes are by default untrusted
const TRUST_TABLE:&str = "TRUST_TABLE";

impl Trust for NodeDB {
    fn trust(&self, node: &Node) -> Result<(), Box<dyn std::error::Error>> {
        let trusted = self.db.open_tree(TRUST_TABLE)?;
        trusted.insert(&node.public_key,  bincode::serialize(&get_epoch())?)?;
        Ok(())
    }

    fn num_trusted(&self) ->  Result<usize, Box<dyn std::error::Error>> {
        let trusted = self.db.open_tree(TRUST_TABLE)?;
        Ok(trusted.len())
    }

    fn untrust(&self, node: &Node) -> Result<(), Box<dyn std::error::Error>> {
        if self.is_trusted(node)? {

            if self.num_trusted()? <= 2 { // Ourselves and the bootstrap node
                Err("Hit minimum number of trusted nodes")?;
            }

            let trusted = self.db.open_tree(TRUST_TABLE)?;
            trusted.remove(&node.public_key)?;
        }

        Ok(())
    }

    fn is_trusted(&self, node: &Node) -> Result<bool, Box<dyn std::error::Error>> {
        let trusted = self.db.open_tree(TRUST_TABLE)?;
        Ok(trusted.contains_key(&node.public_key.clone())?)
    }

    fn get_trusted(&self) -> Result<Vec<(Node, usize)>, Box<dyn std::error::Error>> {
        let trusted = self.db.open_tree(TRUST_TABLE)?;
        
        let mut results = vec![];

        for (_idx, node) in trusted.iter().enumerate() {
            if let Ok((node, _time)) = node {

                let node: Node = bincode::deserialize(&node)?;
                let score = self.get_score(&node, 1200)?;
                results.push((node, score));

            }
        }

        Ok(results)
    }
}



#[test]
fn basic_trust_management() -> Result<(), Box<dyn std::error::Error>> {
    let db = NodeDB::new(tempfile::TempDir::new()?, None)?;
    let node1 = Node::new([0u8; 32]);
    let node2 = Node::new([1u8; 32]);
    let node3 = Node::new([2u8; 32]);
    assert_eq!(db.is_trusted(&node1)?, false);
    db.trust(&node1)?;
    assert_eq!(db.is_trusted(&node1)?, true);

    db.trust(&node2)?;
    db.trust(&node3)?;

    assert_eq!(db.is_trusted(&node3)?, true);
    db.untrust(&node3)?;
    assert_eq!(db.is_trusted(&node3)?, false);
    Ok(())
}

#[test]
fn trust_fetching() -> Result<(), Box<dyn std::error::Error>> {
    let db = NodeDB::new(tempfile::TempDir::new()?, None)?;
    let node1 = Node::new([0u8; 32]);

    assert_eq!(db.get_trusted()?.len(), 0);
    db.trust(&node1)?;
    
    assert_eq!(db.num_trusted()?, 1);
    let trusted = db.get_trusted()?;
    assert_eq!(trusted.last().unwrap(), &(node1, 1200 as usize));

    Ok(())
}