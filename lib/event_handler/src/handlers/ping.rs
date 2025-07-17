use config::db::identity::Identity;
use serde::{Serialize, Deserialize};

use crate::handlers::{Handle, NetworkEvent, pong::Pong};
use crate::connection::ConnectionLogic;

#[derive(Serialize, Deserialize, Debug)]
pub struct Ping {}
impl Handle for Ping {
    async fn action(&self, connection: &mut ConnectionLogic) {
        connection.pipe.send(NetworkEvent::Pong(Pong{})).await;
    }
}