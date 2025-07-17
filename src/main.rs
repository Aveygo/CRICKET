use std::str::FromStr;
use clap::Parser;
use config::db::{NodeDB, PostId};
use config::db::Node as Peer;
use iroh::PublicKey;
use node::Node;
use env_logger::Builder;
use log::{self, info};
use std::thread;
use std::time::Duration;
use std::io;
use std::io::Write;
use event_handler::handlers::{NetworkEvent, ping, peer};
use config::db::search::Search;


#[derive(Parser)]
struct Args {
    src: String,
    bootstrap_nodes: Option<Vec<String>>
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    Builder::from_env(env_logger::Env::new().default_filter_or("cricket=warn,event_handler=warn,node=warn,db=warn")).init();

    info!("start");

    let args = Args::parse();

    let cleaned_nodes = match args.bootstrap_nodes.clone() {
        Some(bootstrap_nodes) => {
            Some(bootstrap_nodes.iter().map(|public| {
                let dest:[u8; 32] = hex::decode(public).expect("could not decode").as_slice().try_into().unwrap();
                Peer::new(dest)
            }).collect())
        },
        None => {None}
    };
    
    let config_loader = NodeDB::new(args.src.to_string(), cleaned_nodes).expect("Could not create database");

    // TODO rename Node to Listener?
    let node = Node::new(config_loader).await;

    /*
    if let Some(bootstraps ) = args.bootstrap_nodes {
        let dest = &bootstraps.get(0).unwrap().clone();
        let dest:[u8; 32] = hex::decode(dest).expect("could not decode").as_slice().try_into().unwrap();
        let dest = PublicKey::from_bytes(&dest).unwrap();
        let event = NetworkEvent::Ping(ping::Ping{});
        //node.pipe_tx.send((dest, event)).unwrap();
        //node.push(dest, event).await
    }
     */

    let node_clone = node.clone();
    tokio::spawn(async move {
        node_clone.accept_connections().await;
    });

    let node_clone = node.clone();
    let us_public_key_bytes = node.public_key.as_bytes().clone();
    tokio::spawn(async move {
        let mut after:Option<PostId> = None;
        loop {
            let posts = node_clone.db.search_posts(&after, 10).unwrap();
            for (post, score) in posts {
                let author = &hex::encode(&post.post.author.public_key)[..6];
                if post.post.author.public_key == us_public_key_bytes {
                    continue;
                }

                let content = &post.post.content;
                println!("{}: {}", author, content.strip_suffix("\n").unwrap());
                io::stdout().flush().unwrap();

                after = Some(post.post.get_id())
            }
            

            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    });


    let mut input_string = String::new();

    loop {
        input_string.clear();
        io::stdin().read_line(&mut input_string).unwrap();
        
        if input_string == "exit" {
            return Ok(())
        }

        node.send_post(&input_string).await;

    }

    
}