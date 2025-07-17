use serde::{Serialize, Deserialize};

use crate::handlers::Handle;
use crate::connection::ConnectionLogic;

#[derive(Serialize, Deserialize, Debug)]
pub struct CloseResponse {}
impl Handle for CloseResponse {
    async fn action(&self, connection: &mut ConnectionLogic) {
        connection.pipe.close().await;
    }
}