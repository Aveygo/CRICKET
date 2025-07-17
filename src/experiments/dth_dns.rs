//! An example chat application using the iroh endpoint and
//! pkarr node discovery.
//!
//! Starting the example without args creates a server that publishes its
//! address to the DHT. Starting the example with a node id as argument
//! looks up the address of the node id in the DHT and connects to it.
//!
//! You can look at the published pkarr DNS record using <https://app.pkarr.org/>.
//!
//! To see what is going on, run with `RUST_LOG=iroh_pkarr_node_discovery=debug`.
use std::{io::Read, str::FromStr};

use clap::Parser;
use iroh::{Endpoint, NodeId, PublicKey};
use tracing::warn;
use url::Url;

use std::time::Instant;
use sha2::{Sha256, Sha512, Digest};
use pkarr::{dns, Keypair, PkarrClient, Result, SignedPacket};

const CHAT_ALPN: &[u8] = b"p2psocial/genesis/0.1.0";

async fn push(secret_bytes:&[u8; 32], key:&str, value:&str) -> anyhow::Result<()> {
    let client = PkarrClient::builder().build().unwrap();
    let keypair = Keypair::from_secret_key(secret_bytes);
    println!("keypair: {:?}", keypair);
    println!("see https://app.pkarr.org/?pk={}", keypair.to_z32());

    let mut packet = dns::Packet::new_reply(0);
    packet.answers.push(dns::ResourceRecord::new(
        dns::Name::new(key).unwrap(),
        dns::CLASS::IN,
        30,
        dns::rdata::RData::TXT(value.try_into()?),
    ));

    let signed_packet = SignedPacket::from_packet(&keypair, &packet)?;

    let instant = Instant::now();

    println!("\nPublishing {} ...", keypair.public_key());

    match client.publish(&signed_packet) {
        Ok(()) => {
            println!(
                "\nSuccessfully published {} in {:?}",
                keypair.public_key(),
                instant.elapsed(),
            );

            Ok(())
        }
        Err(err) => {
            println!("\nFailed to publish {} \n {}", keypair.public_key(), err);
            Err(err.into())
        }
    }
}

async fn pull(secret_bytes:&[u8; 32], key:&str) -> anyhow::Result<String, ()> {
    let client = PkarrClient::builder().build().unwrap();
    let keypair = Keypair::from_secret_key(secret_bytes);
    let public_key = keypair.public_key(); 
    let fetched = client.resolve(&public_key).unwrap();

    let mut key = key.to_string();
    key.push_str(".");
    key.push_str(&public_key.to_string());

    match fetched {
        Some(packet) => {
            let packet = packet.packet();
            let answers = packet.answers.clone();

            for response in answers {                
                if response.name.to_string() == key {

                    match response.rdata.clone() {
                        dns::rdata::RData::TXT(response) => {

                            let response = response.attributes();
                            let response:Vec<String> = response.keys().cloned().collect();
                            let response = (*response.get(0).unwrap()).clone();
                            return Ok(response);
                        },
                        _ => {}                
                    }

                }

                
            }
        },
        None => {
            println!("Got nothing back...")
        }
    }

    Err(())

}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    
    let mut hasher = Sha256::new();
    hasher.update(CHAT_ALPN);
    let result = hasher.finalize();
    let secret_bytes:&[u8; 32] = result.as_slice().try_into()?;

    push(secret_bytes, "foo", "hello world 2").await.unwrap();
    println!("Pulled {:?}", pull(secret_bytes, "foo").await.unwrap());

    Ok(())
}