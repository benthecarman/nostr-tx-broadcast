use base64::{Engine as _, engine::general_purpose};
use bitcoin::consensus::{Decodable, serialize};
use bitcoin::Transaction;
use hex_string::HexString;
use nostr::Keys;
use nostr::prelude::*;
use nostr_sdk::Client;
use nostr_sdk::relay::pool::RelayPoolNotification::*;

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

                    broadcast_tx(transaction).await?;
                }
            }
            Ok(())
        })
        .await?;
    Ok(())
}

async fn broadcast_tx(tx: Transaction) -> anyhow::Result<()> {
    let client = reqwest::Client::builder().build()?;

    let bytes = serialize(&tx);
    let body = HexString::from_bytes(&bytes).as_string();

    client
        .post(&format!("https://mempool.space/api/tx"))
        .body(body)
        .send()
        .await?
        .error_for_status()?;

    Ok(println!("Broadcasted tx: {}", tx.txid()))
}
