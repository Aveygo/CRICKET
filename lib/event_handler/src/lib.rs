
pub mod handlers;
pub mod pipe;
pub mod connection;

/*
use serde::{Serialize, Deserialize};
use std::{thread, time};
use log::warn;

pub mod pipe;

#[derive(Serialize, Deserialize, Debug)]
pub struct Ping { }

#[derive(Serialize, Deserialize, Debug)]
pub struct Pong { }

#[derive(Serialize, Deserialize, Debug)]
pub struct DataTransfer { }

#[derive(Serialize, Deserialize, Debug)]
pub struct Heartbeat { }

#[derive(Serialize, Deserialize, Debug)]
pub struct CloseRequest { }

#[derive(Serialize, Deserialize, Debug)]
pub struct CloseResponse { }


#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", content = "data")]
pub enum NetworkEvent {
    Ping(Ping),
    Pong(Pong),
	Heartbeat(Heartbeat),
    CloseRequest(CloseRequest),
	CloseResponse(CloseResponse),
}



pub struct ConnectionLogic {
	pub pipe:pipe::Pipe<NetworkEvent>
}

trait Handle {
    async fn action(self, connection: &mut ConnectionLogic);
}

impl Handle for Ping {
	async fn action(self, connection: &mut ConnectionLogic) {
		connection.pipe.send(NetworkEvent::Pong(Pong{})).await
	}
}

impl ConnectionLogic {
    pub fn new(pipe:pipe::Pipe<NetworkEvent>) -> Self {
    	ConnectionLogic{
			pipe: pipe
    	}
    }


	pub async fn received_pong(&mut self, _pong:Pong) {
		self.pipe.send(NetworkEvent::CloseRequest(CloseRequest{})).await;
	}

	pub async fn received_close_request(&mut self, _close:CloseRequest) {
		self.pipe.send(NetworkEvent::CloseResponse(CloseResponse{})).await;
		self.pipe.wait_for_close().await;
	}

	pub async fn received_close_response(&mut self, _close:CloseResponse) {
		self.pipe.close().await;
	}

	pub async fn received_heart(&mut self, _heart:Heartbeat) {
		thread::sleep(time::Duration::from_secs(1));
		self.pipe.send(NetworkEvent::Heartbeat(Heartbeat{})).await;
	}
	
	pub async fn handle(&mut self) {
		
		loop {
			let response = self.pipe.receive().await;
			
			if let Ok(response) = response {

				match response {
					NetworkEvent::Ping(ping) => { ping.action(self).await },
					NetworkEvent::Pong(pong) => { self.received_pong(pong).await },
					NetworkEvent::Heartbeat(heart) => { self.received_heart(heart).await },
					NetworkEvent::CloseRequest(close) => { self.received_close_request(close).await; return}
					NetworkEvent::CloseResponse(close) => { self.received_close_response(close).await; return}
				};

			} else {
				warn!("Pipe encountered: {:?}", response);
				return;
			}

		}
	}


}


 */