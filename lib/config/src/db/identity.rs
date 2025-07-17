use crate::db::{NodeDB, Node, Us};

pub trait Identity {
    fn generate_identity(&self) -> Result<Us, Box<dyn std::error::Error>>;
    fn get_identity(&self) -> Result<Us, Box<dyn std::error::Error>>;
}

const IDENTITY_TABLE:&str = "IDENTITY_TABLE";

impl Identity for NodeDB {
    fn generate_identity(&self) -> Result<Us, Box<dyn std::error::Error>> {
        let mut secret = [0u8; 32];
        rand::fill(&mut secret[..]); 
        Ok(Us::new(secret))
    }
    fn get_identity(&self) -> Result<Us, Box<dyn std::error::Error>> {
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
        Ok(Us::new(private_key))
    }
}