use std::fs;
use std::path::PathBuf;

use anyhow::{bail, Context as ErrorContext, Result};
use p2panda_rs::identity::KeyPair;

use crate::context::Context;
use crate::lock_file::LockFile;
use crate::schema_file::SchemaFile;

pub fn update(context: Context, private_key_path: &PathBuf) -> Result<()> {
    let schema_file_str = fs::read_to_string(&context.schema_path)?;
    let schema_file: SchemaFile = toml::from_str(&schema_file_str)?;

    let lock_file_str = fs::read_to_string(&context.lock_path)?;
    let lock_file: LockFile = toml::from_str(&lock_file_str)?;

    let private_key_file_str = fs::read_to_string(&private_key_path)?;
    let key_pair = KeyPair::from_private_key_str(&private_key_file_str)?;

    println!("{:?}", schema_file);
    println!("{:?}", lock_file);
    println!("{}", key_pair.public_key());

    // @TODO
    // Parse schema from schema.toml, validate it
    // Parse entries + operations from schema.lock, validate them
    // .. Store them in storage provider
    // .. Materialize a Schema Document from them, validate it
    // Compare schema from .toml w. materialized document
    // .. show diff to user
    // Generate operations for each diff
    // Ask user to confirm it
    // Sign operations and store them in schema.lock

    Ok(())
}
