use config::db::identity::Identity;
use iroh::PublicKey;
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use crate::handlers::{Handle, NetworkEvent, close_request::CloseRequest};
use crate::connection::ConnectionLogic;
use config::db::{IncomingPost, NodeDB, OutgoingPost};
use config::db::trust::Trust;
use std::sync::mpsc::Sender;
use log::{info, warn};

use config::db::handle_post::HandlePost;

#[derive(Serialize, Deserialize, Debug)]
pub struct Post {
    data: OutgoingPost
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TrustRequest {

}

#[derive(Serialize, Deserialize, Debug)]
pub struct TrustResponse {

}


#[derive(Serialize, Deserialize, Debug)]
pub struct TrustBootstrap {

}

pub async fn share_post(post: IncomingPost, db: &Arc<NodeDB>, pusher: &Sender<(PublicKey, NetworkEvent)>) {
    // We need to trust any message that come from the bootstrap node,
    // but also *any* message if *we* are the bootstrap node (empty bootstrap list)
    let bootstrap_nodes = &db.bootstrap_nodes;
    match bootstrap_nodes {
        Some(bootstrap_nodes) => {
            // We are a peer node - trust all bootstrap nodes
            for node in bootstrap_nodes {                    
                db.trust(node).unwrap();
            }
        },
        None => {
            // We are a bootstrap node - trust all peers
            if post.history.len() > 0 {
                let from = &post.history.last().unwrap().from;
                db.trust(from).unwrap();
            }
        }
    }

    let r = db.receive(&post).unwrap();

    for outgoing in r {
        
        let to_node = &outgoing.history.last().unwrap().to;
        let to_public = PublicKey::from_bytes(&to_node.public_key).unwrap();
        let event = NetworkEvent::Post(Post{data:outgoing});

        pusher.send((to_public, event)).unwrap();
    }
}

impl Handle for Post {
    /*
        A node sent their outgoing post to us.
     */ 

    async fn action(&self, connection: &mut ConnectionLogic) {
        let recv_post = &self.data;
        let post = IncomingPost::new(
            &recv_post.post,
            &recv_post.history,
            &recv_post.signature,
            &connection.pipe.db.get_identity().unwrap()
        );

        match post {
            Ok(post) => {
                share_post(post, &connection.pipe.db, &connection.pipe.pusher).await;
            },
            Err(e) => {
                warn!("Rejected post due to: {:?}", e);
            }
        };

        
        connection.pipe.send(NetworkEvent::CloseRequest(CloseRequest{})).await;
    }
}