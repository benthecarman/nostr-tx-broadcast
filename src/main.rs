use anyhow::anyhow;
use bitcoin::consensus::{serialize, Decodable};
use bitcoin::network::Magic;
use bitcoin::Transaction;
use clap::Parser;
use hex_string::HexString;
use nostr::prelude::*;
use nostr::Keys;
use nostr_sdk::relay::pool::RelayPoolNotification::*;
use nostr_sdk::Client;
use std::str::FromStr;

#[derive(Parser)]
#[command()]
struct Args {
    #[arg(short, long)]
    relays: Vec<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let my_keys = Keys::generate();

    let client = Client::new(&my_keys);

    if args.relays.len() == 0 {
        anyhow::bail!("No relay(s) provided");
    }

    for relay in args.relays {
        client.add_relay(relay, None).await?;
    }
    client.connect().await;

    let bitcoin_tx_kind = Kind::Custom(28333);
    let subscription = Filter::new()
        .kinds(vec![bitcoin_tx_kind])
        .since(Timestamp::now());

    client.subscribe(vec![subscription]).await;

    println!("Listening for bitcoin txs...");
    client
        .handle_notifications(|notification| async {
            if let Event(_, event) = notification {
                if event.kind == bitcoin_tx_kind {
                    // calculate network from magic
                    let magic = event
                        .tags
                        .clone()
                        .into_iter()
                        .find(|t| t.kind() == TagKind::Custom("magic".to_string()))
                        .and_then(|t| {
                            if let Tag::Generic(_, magic) = t {
                                magic.first().and_then(|m| Magic::from_str(m).ok())
                            } else {
                                None
                            }
                        });

                    // get transactions
                    let txs: Vec<Transaction> = event
                        .tags
                        .clone()
                        .into_iter()
                        .find(|t| t.kind() == TagKind::Custom("transactions".to_string()))
                        .map(|t| {
                            if let Tag::Generic(_, txs) = t {
                                txs.iter().filter_map(|tx| {
                                    HexString::from_string(tx).ok().and_then(|hex| {
                                        Transaction::consensus_decode(&mut hex.as_bytes().as_slice()).ok()
                                    })
                                }).collect()
                            } else {
                                vec![]
                            }
                        }).unwrap_or_default();

                    match magic {
                        Some(magic) => {
                            if let Err(e) = broadcast_txs(txs, magic).await {
                                println!("Error broadcasting txs: {e}");
                            }
                        }
                        None => {
                            println!("Network: unknown");
                        }
                    }
                }
            }
            Ok(())
        })
        .await?;
    Ok(())
}

async fn broadcast_txs(txs: Vec<Transaction>, magic: Magic) -> anyhow::Result<()> {
    if txs.is_empty() {
        return Ok(());
    }

    let client = reqwest::Client::builder().build()?;

    let mutinynet = Magic::from_bytes([0xA5, 0xDF, 0x2D, 0xCB]);

    let url = match magic {
        Magic::BITCOIN => Ok("https://mempool.space/api/tx"),
        Magic::TESTNET => Ok("https://mempool.space/testnet/api/tx"),
        Magic::SIGNET => Ok("https://mempool.space/signet/api/tx"),
        magic if magic == mutinynet => Ok("https://mutinynet.com/api/tx"),
        magic => Err(anyhow!("Magic: {magic} is unknown")),
    }?;

    for tx in txs {
        let bytes = serialize(&tx);
        let body = HexString::from_bytes(&bytes).as_string();

        client
            .post(url)
            .body(body)
            .send()
            .await?
            .error_for_status()?;

        println!("Broadcasted tx: {}", tx.txid());
    }

    Ok(())
}
