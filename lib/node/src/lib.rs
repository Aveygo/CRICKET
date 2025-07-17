use config::db::{IncomingPost, NodeDB, RawPost, Hashable};
use config::db::identity::Identity;

use iroh::{Endpoint, PublicKey};
use std::sync::Arc;
use log::{info, warn};
use std::thread;

use std::sync::mpsc::{Sender, Receiver};
use std::sync::mpsc;

use event_handler::{connection::ConnectionLogic, handlers::NetworkEvent, pipe::Pipe};
use event_handler::handlers::peer::share_post;

const CHAT_ALPN: &[u8] = b"pkarr-discovery-demo-chat";

pub struct Node {
    pub endpoint: Arc<Endpoint>,
    pub public_key: PublicKey,
    pub db: Arc<NodeDB>,
    pub pipe_tx: Sender<(PublicKey, NetworkEvent)> // So the pipe can create new connections to other peers
}

impl Node {
    pub async fn new(db:NodeDB) -> Arc<Self> {
        
        /*
            TODO, I am feeling sick, so i might leave this project for a sec
            for future me, I just finished lib/config, and main.rs will be broken, but the logic should be ready to integrate with my custom event_handler
            I wrote peer.rs before I redesigned the protocol, but it is roughly how it should be structured

            [DONE] You need to create an arc for NodeDB so it can be passed to each generated pipe
            From the db, the main functions you need to worry about are in trust_request
            You also need to write manual logic for the bootstrap node to accept connections without trusting first (otherwise the bootstrap node will reject all new connections because a trust request is not possible to make without receiving a post first)
            The DB also assumes you manually trust the user and the bootstrap node, so make sure you do that 
            
            The protocol should have the messages:
                - Post
                - TrustRequest
                - TrustResponse
                - TrustBootstrap
            
            The user should be able to:
                - FetchPosts
                - PromotePost
                - DemotePost

            You will need to consider the case where you cannot connect to a given peer (demote?)
            You could also do something where the node does not construct a pipe if the connecting peer scores too low. 

            Anyways, hope you feel better.
        */

        let raw_secret = db.get_identity().unwrap();
        let secret_key = iroh::SecretKey::from_bytes(&raw_secret.private_key.clone());
        let public_key = secret_key.public();

        info!("We are {:?}", public_key);

        let discovery = iroh::discovery::pkarr::dht::DhtDiscovery::builder().dht(true)
            .n0_dns_pkarr_relay()
            .secret_key(secret_key.clone())
            .build()
            .unwrap();

        let endpoint = Endpoint::builder()
            .alpns(vec![CHAT_ALPN.to_vec()])
            .secret_key(secret_key)
            .discovery(Box::new(discovery))
            .bind()
            .await
            .unwrap();
        
        let (pipe_tx, pipe_rx): (Sender<(PublicKey, NetworkEvent)>, Receiver<(PublicKey, NetworkEvent)>) = mpsc::channel();

        let node = Node {
            endpoint: Arc::new(endpoint),
            public_key: public_key,
            db: Arc::new(db),
            pipe_tx: pipe_tx
        };

        let node = Arc::new(node);
        let node_first_copy = node.clone();

        // This is the worst code of my life
        // So basically, we need to spawn a pipe for every new connection to a node,
        // but a pipe might need to spawn a connection from within.
        // So we create a channel that allows each pipe to spawn new connections
        // TODO, make it so the pipe can create *and* then get the output from other pipes?
        let rt = tokio::runtime::Runtime::new().unwrap();
        thread::spawn(move || {
            loop {
                let (destination, event) = pipe_rx.recv().unwrap();
                let node_second_copy = node_first_copy.clone();
                
                rt.spawn(async move {
                    node_second_copy.push(destination, event).await;
                });
            }
        });

        node
        
    }

    pub fn push_to_thread(&self, mut connection: ConnectionLogic) {
        thread::spawn(move || {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                connection.handle().await;
            });
        });
    }

    pub async fn send_post(&self, content:&String) {
        let us = self.db.get_identity().unwrap();
        let raw = RawPost::new(us.node.clone(),content.clone());
        let signature = us.sign(&raw.hash());
        let post = IncomingPost::new(&raw, &vec![], &signature, &us).unwrap();

        share_post(post, &self.db, &self.pipe_tx).await;

    }

    pub async fn push(&self, destination:PublicKey, event:NetworkEvent) {
        let pipe = self.connect_to_node(destination).await;
        let mut connection = ConnectionLogic::new(pipe);
        connection.pipe.send(event).await;
        self.push_to_thread(connection);

    }

    pub async fn connect_to_node(&self, node:PublicKey) -> Pipe<NetworkEvent> {
        info!("Connecting to {:?}", node); 
        let connection = self.endpoint.connect(node, CHAT_ALPN).await.unwrap();     
        info!("Connection made with {:?}", node);             
        let (send, recv) = connection.open_bi().await.unwrap();
        let db_ref = self.db.clone();
        Pipe::new(send, recv, node, connection, db_ref, self.pipe_tx.clone())
    }
   
    pub async fn accept_connections(&self) {
        while let Some(incoming) = self.endpoint.accept().await {

            let connecting = match incoming.accept() {
                Ok(connecting) => connecting,
                Err(err) => {
                    warn!("Unstable incoming connection: {err:#}");
                    continue;
                }
            };
            
            let connection = connecting.await.unwrap();
            let node = connection.remote_node_id().unwrap();
            info!("Connection made with {:?}", node);

            let (send, recv) = connection.accept_bi().await.unwrap();

            let db_ref = self.db.clone();

            let pipe:Pipe<NetworkEvent> = Pipe::new(send, recv, node, connection, db_ref, self.pipe_tx.clone());
            let connection = ConnectionLogic::new(pipe);
            self.push_to_thread(connection);
        }
    }



}