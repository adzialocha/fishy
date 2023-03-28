use std::fs;

use anyhow::{bail, Result};
use gql_client::Client;
use indicatif::ProgressBar;
use p2panda_rs::entry::decode::decode_entry;
use p2panda_rs::entry::traits::AsEntry;
use p2panda_rs::entry::{LogId, SeqNum};
use p2panda_rs::hash::Hash;
use serde::Deserialize;

use crate::context::Context;
use crate::files::LockFile;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct NextArguments {
    log_id: LogId,
    seq_num: SeqNum,
    skiplink: Option<Hash>,
    backlink: Option<Hash>,
}

/// GraphQL response for `nextArgs` query.
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct NextArgsResponse {
    next_args: NextArguments,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct PublishResponse {
    publish: NextArguments,
}

pub async fn publish(context: Context, endpoint: &str) -> Result<()> {
    let lock_file_str = fs::read_to_string(&context.lock_path)?;
    let lock_file: LockFile = toml::from_str(&lock_file_str)?;
    let commits = lock_file.commits.unwrap_or(Vec::new());

    if commits.is_empty() {
        bail!("Nothing to commit");
    }

    let mut skipped = 0;
    let total = commits.len();

    let bar = ProgressBar::new(total as u64);

    let client = Client::new(endpoint);

    for commit in commits {
        let entry = decode_entry(&commit.entry).unwrap();

        let query = format!(
            r#"
            {{
                nextArgs(publicKey: "{}", viewId: "{}") {{
                    logId
                    seqNum
                    skiplink
                    backlink
                }}
            }}
            "#,
            entry.public_key(),
            commit.entry_hash,
        );

        let response = client.query_unwrap::<NextArgsResponse>(&query).await;

        if let Ok(result) = response {
            let args = result.next_args;

            if entry.log_id() != &args.log_id {
                bail!("Inconsistency detected");
            }

            if entry.seq_num() < &args.seq_num {
                bar.inc(1);
                skipped += 1;

                // Skip this one
                continue;
            }
        }

        let query = format!(
            r#"
            mutation Publish {{
                publish(entry: "{}", operation: "{}") {{
                    logId
                    seqNum
                    skiplink
                    backlink
                }}
            }}
            "#,
            commit.entry, commit.operation
        );

        client
            .query_unwrap::<PublishResponse>(&query)
            .await
            .expect("GraphQL mutation `publish` failed");

        bar.inc(1);
    }

    println!(
        "Done. Published {} commits (ignored {}).",
        total - skipped,
        skipped,
    );

    Ok(())
}
