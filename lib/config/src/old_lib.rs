use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::path::Path;
use serde_json;
use rand;
use hex;

mod db;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    pub secret: String,
    pub bootstrap_addr: Option<String>,
    pub peers: Vec<String>,
    
}
#[derive(Debug, Deserialize, Serialize, Clone)]

pub struct ConfigLoader {
    pub path: String,
    pub config: Config
}

impl ConfigLoader {
    pub fn new(config_src: String) -> Self {

        let config_path = Path::new(&config_src);

        let config: Config = if config_path.exists() {
            let file = File::open(config_path).unwrap();
            serde_json::from_reader(file).unwrap()
        } else {
            let mut secret = [0u8; 32];
            rand::fill(&mut secret[..]); 

            Config {
                secret: hex::encode(&secret),
                bootstrap_addr: Some("f63m9dfim9pd6mxptgskm86py5yfhqghoreez6xe91f4e4gf3pay".to_string()),
                peers: vec![]
            }
        };

        let loader = ConfigLoader{
            path: config_src,
            config: config
        };

        loader.dump();

        loader

    }

    
    pub fn dump(&self) {
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(self.path.clone())
            .unwrap();
    
        serde_json::to_writer(&file, &self.config).unwrap();
    }
}



