use serde::{Serialize, Deserialize};

use crate::handlers::{Handle, NetworkEvent};
use crate::connection::ConnectionLogic;
use std::{thread, time};


#[derive(Serialize, Deserialize, Debug)]
pub struct Heartbeat {}
impl Handle for Heartbeat {
    async fn action(&self, connection: &mut ConnectionLogic) {
        thread::sleep(time::Duration::from_secs(1));
        connection.pipe.send(NetworkEvent::Heartbeat(Heartbeat{})).await;
    }
}