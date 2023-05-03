use anyhow::anyhow;
use base64::{engine::general_purpose, Engine as _};
use bitcoin::consensus::{serialize, Decodable};
use bitcoin::network::Magic;
use bitcoin::Transaction;
use hex_string::HexString;
use nostr::prelude::*;
use nostr::Keys;
use nostr_sdk::relay::pool::RelayPoolNotification::*;
use nostr_sdk::Client;
use std::str::FromStr;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let my_keys = Keys::generate();

    let client = Client::new(&my_keys);
    client.add_relay("wss://nostr.wine", None).await?;
    client.add_relay("wss://nos.lol", None).await?;
    client.add_relay("wss://nostr.fmt.wiz.biz", None).await?;
    client.add_relay("wss://nostr.zebedee.cloud", None).await?;
    client.add_relay("wss://relay.damus.io", None).await?;

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
                    let decoded = general_purpose::STANDARD.decode(event.content)?;
                    let transaction = Transaction::consensus_decode(&mut decoded.as_slice())?;

                    // calculate network from magic
                    let magic = event
                        .tags
                        .clone()
                        .into_iter()
                        .find(|t| t.kind() == TagKind::Custom("magic".to_string()))
                        .and_then(|t| {
                            if let Tag::Generic(_, magic) = t {
                                let str = magic.first().unwrap().clone();
                                Magic::from_str(&str).ok()
                            } else {
                                None
                            }
                        });

                    match magic {
                        Some(magic) => {
                            broadcast_tx(transaction, magic).await?;
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

async fn broadcast_tx(tx: Transaction, magic: Magic) -> anyhow::Result<()> {
    let client = reqwest::Client::builder().build()?;

    let mutinynet = Magic::from_bytes([0xA5, 0xDF, 0x2D, 0xCB]);

    let url = match magic {
        Magic::BITCOIN => Ok("https://mempool.space/api/tx"),
        Magic::TESTNET => Ok("https://mempool.space/testnet/api/tx"),
        Magic::SIGNET => Ok("https://mempool.space/signet/api/tx"),
        magic if magic == mutinynet => Ok("https://mutinynet.com/api/tx"),
        magic => Err(anyhow!("Magic: {magic} is unknown")),
    }?;

    let bytes = serialize(&tx);
    let body = HexString::from_bytes(&bytes).as_string();

    client
        .post(url)
        .body(body)
        .send()
        .await?
        .error_for_status()?;

    Ok(println!("Broadcasted tx: {}", tx.txid()))
}
