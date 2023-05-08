#![cfg(feature = "ip2country")]

use std::{collections::HashMap, path::Path};

use futures_util::stream::TryStreamExt;
use maxminddb_writer::paths::IpAddrWithMask;
use tokio::{io::AsyncBufReadExt, sync::mpsc};
use tokio_util::io::StreamReader;

const OUTPUT_PATH: &str = "ip2country.mmdb";

async fn load_entries_from_url(
    url: &str,
    sender: mpsc::Sender<(IpAddrWithMask, String)>,
) -> anyhow::Result<()> {
    let response = reqwest::get(url).await?;
    let mut reader = StreamReader::new(
        response
            .bytes_stream()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e)),
    );

    let mut line = String::new();
    loop {
        // read a line
        line.clear();
        if reader.read_line(&mut line).await? == 0 {
            break;
        }

        // skip comments
        if line.starts_with('#') {
            continue;
        }

        let parts = line.split('|').collect::<Vec<_>>();

        // skip lines that are not IP allocations
        if parts.len() != 8
            || parts[1] == "*"
            || parts[3] == "*"
            || (parts[2] != "ipv4" && parts[2] != "ipv6")
        {
            continue;
        }

        // extract country code
        let country_code = parts[1].to_string();
        if country_code == "ZZ" {
            continue;
        }

        // extract IP address and mask
        let Ok(ip) = parts[3].parse::<std::net::IpAddr>() else { continue; };
        let Ok(count) = parts[4].parse::<usize>() else { continue; };
        for ip_with_mask in IpAddrWithMask::from_count(ip, count) {
            sender.send((ip_with_mask, country_code.clone())).await?;
        }
    }

    Ok(())
}

fn validate(path: impl AsRef<Path>, entries: &[(IpAddrWithMask, String)]) -> anyhow::Result<()> {
    let db = maxminddb::Reader::open_readfile(&path)?;
    for (ip_with_mask, expected_country_code) in entries {
        let country_code: String = db.lookup::<String>(ip_with_mask.addr)?;
        assert_eq!(
            &country_code, expected_country_code,
            "ip={}",
            ip_with_mask.addr
        );
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (tx, mut rx) = mpsc::channel(100);

    for url in [
        "http://localhost:8080/list/afrinic.txt",
        "http://localhost:8080/list/apnic.txt",
        "http://localhost:8080/list/arin.txt",
        "http://localhost:8080/list/lacnic.txt",
        "http://localhost:8080/list/ripencc.txt",
    ] {
        tokio::spawn(load_entries_from_url(url, tx.clone()));
    }
    drop(tx);

    let mut db = maxminddb_writer::Database::default();
    let mut country_refs = HashMap::new();
    let mut validation_data = Vec::new();

    while let Some((ip_with_mask, country_code)) = rx.recv().await {
        match ip_with_mask.addr {
            std::net::IpAddr::V4(_) => {
                validation_data.push((ip_with_mask, country_code.clone()));
                let country_code_ref =
                    *country_refs.entry(country_code.clone()).or_insert_with(|| {
                        db.insert_value(country_code.clone())
                            .expect("failed to insert country code")
                    });
                db.insert_node(ip_with_mask, country_code_ref);
            }
            std::net::IpAddr::V6(addr) => {
                log::info!("skipping IPv6 address {}", addr);
            }
        }
    }

    db.write_to(std::fs::File::create(OUTPUT_PATH)?)?;

    validate(OUTPUT_PATH, &validation_data)?;

    Ok(())
}
