use serde::{Serialize, Deserialize};

use crate::handlers::{Handle, NetworkEvent, close_request::CloseRequest};
use crate::connection::ConnectionLogic;

#[derive(Serialize, Deserialize, Debug)]
pub struct Pong {}
impl Handle for Pong {
    async fn action(&self, connection: &mut ConnectionLogic) {
        connection.pipe.send(NetworkEvent::CloseRequest(CloseRequest{})).await;
    }
}