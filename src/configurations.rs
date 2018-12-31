//! A module that contains functions required to read, parse and return the configuration settings
//! from the file `Config.toml` at the root of this crate.

// TODO: handle errors more appropriately.

use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::str::FromStr;

use config::{Config, File};

pub fn get_config(file_name: &str) -> HashMap<String, (usize, SocketAddrV4)> {
    let c = read_config(file_name);
    parse_config(&c)
}

fn read_config(file_name: &str) -> HashMap<String, HashMap<String, String>> {
    let mut c = Config::default();
    c.merge(File::with_name(file_name)).unwrap();
    c.try_into::<HashMap<String, HashMap<String, String>>>().expect("Could not try_into")
}

fn parse_config(c: &HashMap<String, HashMap<String, String>>) -> HashMap<String, (usize, SocketAddrV4)> {
    c.iter().map(|(key, value)| {
        (
            key.clone(),
            (
                value["size"].parse().unwrap(),
                SocketAddrV4::new(
                    Ipv4Addr::from_str(&value["host"]).unwrap(),
                    value["port"].parse().unwrap(),
                ),
            ),
        )
    }).collect()
}
