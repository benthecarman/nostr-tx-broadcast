use bitcoin::Network;
use clap::Parser;
use hex_string::HexString;
use nostr::prelude::*;
use nostr::Keys;
use nostr_sdk::Client;

#[derive(Parser)]
#[command()]
struct Args {
    #[clap(default_value_t = Network::Bitcoin, short, long)]
    network: Network,

    #[arg(short, long)]
    relays: Vec<String>,

    #[arg(num_args(1..))]
    txs: Vec<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Check that hex strings are valid.
    for tx in &args.txs {
        if let Err(_e) = HexString::from_string(&tx) {
            anyhow::bail!("hex decoding error");
        };
    }

    let my_keys = Keys::generate();

    let client = Client::new(&my_keys);
    for relay in args.relays {
        client.add_relay(relay, None).await?;
    }
    client.connect().await;

    let bitcoin_tx_kind = Kind::Custom(28333);

    let magic = args.network.magic();

    let net_tag = Tag::Generic(
        TagKind::Custom("magic".to_string()),
        vec![magic.to_string()],
    );
    let txs_tag = Tag::Generic(TagKind::Custom("transactions".to_string()), args.txs);
    let tags = vec![net_tag, txs_tag];

    let event: Event = EventBuilder::new(bitcoin_tx_kind, "", &tags).to_event(&my_keys)?;

    let id = client.send_event(event).await?;

    println!("Event id: {}", id);

    Ok(())
}
