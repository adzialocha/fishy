use std::fs;

use anyhow::{bail, Result};
use gql_client::Client;
use p2panda_rs::entry::{LogId, SeqNum};
use p2panda_rs::hash::Hash;
use serde::Deserialize;
use indicatif::ProgressBar;

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

    let bar = ProgressBar::new(commits.len() as u64);

    let client = Client::new(endpoint);

    for commit in commits {
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

    println!("Done.");

    Ok(())
}
