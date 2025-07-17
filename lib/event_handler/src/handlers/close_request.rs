use serde::{Serialize, Deserialize};

use crate::handlers::{Handle, NetworkEvent, close_response::CloseResponse};
use crate::connection::ConnectionLogic;

#[derive(Serialize, Deserialize, Debug)]
pub struct CloseRequest {}
impl Handle for CloseRequest {
    async fn action(&self, connection: &mut ConnectionLogic) {
        connection.pipe.send(NetworkEvent::CloseResponse(CloseResponse{})).await;
        connection.pipe.wait_for_close().await;
    }
}