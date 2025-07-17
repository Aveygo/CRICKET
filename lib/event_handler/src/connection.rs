use crate::handlers::NetworkEvent;
use crate::pipe::{NetworkEventError, Pipe};
use crate::handlers::Handle;
use std::sync::Arc;
use log::{info, warn};

pub struct ConnectionLogic {
    pub pipe: Pipe<NetworkEvent>
}

impl ConnectionLogic {
    pub fn new(pipe: Pipe<NetworkEvent>) -> Self {
        ConnectionLogic { pipe }
    }

    pub async fn handle(&mut self) {
        let outcome;
        loop {
            let response = self.pipe.receive().await;

            match response {
                Ok(response) => {
                    response.action(self).await;

                    // Special commands that require stop
                    match response {
                        NetworkEvent::CloseRequest(_) => {outcome = Ok(()); break},
                        NetworkEvent::CloseResponse(_) => {outcome = Ok(()); break},
                        _ => {}
                    }
                },
                Err(e) => {
                    outcome = Err(e); break;
                }
            }
        }

        match outcome {
            Ok(_r) => info!("Connection with {:?} safely stopped", self.pipe.public),
            Err(e) => {
                let e = Box::new(e);
                warn!("Connection stopped {:?} with error {:?}", self.pipe.public, e);
            }
        };

    }



}