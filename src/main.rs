use chrono::{DateTime, Local, Utc};
use dotenv::dotenv;
use helius::types::*;
use helius::Helius;
use solana_client::rpc_client::GetConfirmedSignaturesForAddress2Config;
use solana_client::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::signature::Signature;
use std::env;
use std::str::FromStr;
use teloxide::{prelude::*, utils::command::BotCommands};

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    dotenv().ok();
    let api_key = env::var("TELOXIDE_TOKEN");
    log::info!("api-key: {:?}", api_key);
    log::info!("Starting command bot...");

    let bot = Bot::from_env();

    Command::repl(bot, answer).await;
}

#[derive(BotCommands, Clone, Debug)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
enum Command {
    #[command(description = "show this help message.")]
    Help,
    #[command(description = "handle a username.")]
    AllTx(String),
}

async fn answer(bot: Bot, msg: Message, cmd: Command) -> ResponseResult<()> {
    log::info!("bot: {:?}, msg: {:?}, cmd: {:?}", bot, msg, cmd);
    match cmd {
        Command::Help => {
            bot.send_message(msg.chat.id, Command::descriptions().to_string())
                .await?
        }
        Command::AllTx(address) => {
            let rpc_endpoint = env::var("RPC_ENDPOINT").unwrap();
            let helius_api_key = env::var("HELIUS_API_KEY").unwrap();
            let commitment = CommitmentConfig::confirmed();
            let rpc_client = RpcClient::new_with_commitment(rpc_endpoint, commitment);
            let address = solana_sdk::pubkey::Pubkey::from_str(&address).unwrap();

            let mut all_txs = Vec::new();
            let mut before = None;
            loop {
                let config = GetConfirmedSignaturesForAddress2Config {
                    before,
                    until: None,
                    limit: Some(1000),
                    commitment: Some(CommitmentConfig::confirmed()),
                };
                let mut result = rpc_client
                    .get_signatures_for_address_with_config(&address, config)
                    .unwrap()
                    .into_iter()
                    .collect::<Vec<_>>();
                let last_signature = result.last();
                log::info!("last_signature: {:?}", last_signature);
                before = Some(
                    Signature::from_str(
                        &result
                            .last()
                            .ok_or(anyhow::anyhow!("get signatures is empty"))
                            .unwrap()
                            .signature
                            .clone(),
                    )
                    .unwrap(),
                );
                if result.len() < 1000 {
                    all_txs.append(&mut result);
                    break;
                } else {
                    all_txs.append(&mut result);
                    continue;
                }
            }
            log::info!("Address {} have {} transacition", address, all_txs.len());

            let all_txs = all_txs
                .into_iter()
                .filter(|tx| tx.err.is_none())
                .map(|item| item.signature)
                .collect::<Vec<_>>();

            log::info!(
                "Address {} have {} success transacition",
                address,
                all_txs.len()
            );

            let cluster: Cluster = Cluster::MainnetBeta;
            let helius: Helius = Helius::new(&helius_api_key, cluster).unwrap();

            let request: Vec<ParseTransactionsRequest> =
                ParseTransactionsRequest::from_slice(&all_txs[0..20]);
            log::info!("request: {:?}", request.len());

            let mut msgs = String::new();
            for req in request {
                let response: Vec<EnhancedTransaction> = helius
                    .parse_transactions(req)
                    .await
                    .unwrap()
                    .into_iter()
                    .filter(|v| !v.description.is_empty()) // filter description is empty
                    .filter(|v| !v.description.contains("nft")) // filter nft tx
                    //.filter(|v| !v.description.contains("mint")) // filter mint tx
                    .filter(|v| !v.description.contains("multiple")) // filter contain multiple tx
                    .filter(|v| {
                        !v.description.contains("0.000000001 SOL")
                            && !v.description.contains("0 SOL")
                            && !v.description.contains("0.0001")
                    }) // filet contain 0.000000001 SOL tx
                    // .filter(|v| v.events.swap)
                    .collect();

                for (_idx, tx) in response.iter().enumerate() {
                    let dt = DateTime::<Utc>::from_utc(
                        chrono::NaiveDateTime::from_timestamp(tx.timestamp as i64, 0),
                        Utc,
                    );
                    let local_dt = dt.with_timezone(&Local);
                    msgs.push_str(&format!("🌟{} 🌟🌟🌟 {}\n", local_dt, tx.description));
                }
            }

            bot.send_message(msg.chat.id, msgs).await?
        }
    };

    Ok(())
}
