use serde::{Serialize, Deserialize};

use crate::handlers::{Handle, NetworkEvent};
use crate::connection::ConnectionLogic;

#[derive(Serialize, Deserialize, Debug)]
pub struct Error {
    message: String // 'we want to close this connection because reasons'
}

impl Handle for Error {
    async fn action(&self, connection: &mut ConnectionLogic) {

        
    }
}