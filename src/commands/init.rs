use std::fs::File;
use std::io::Write;
use std::path::Path;

use anyhow::{bail, Context as ErrorContext, Result};
use p2panda_rs::identity::KeyPair;
use p2panda_rs::schema::{SchemaDescription, SchemaName};

use crate::context::Context;
use crate::files::{FieldType, SchemaFields, SchemaFile};

fn write_file(path: &str, content: &str) -> Result<()> {
    let mut file = File::create(&path)?;
    file.write_all(content.as_bytes())?;
    Ok(())
}

pub fn init(_context: Context, name: &str) -> Result<()> {
    ["schema.toml", "schema.lock", "secret.txt"]
        .iter()
        .try_for_each(|file_name| {
            if Path::new(file_name).exists() {
                bail!("Found an already existing '{file_name}' file")
            }

            Ok(())
        })
        .with_context(|| "Could not initialise schema in this folder")?;

    let key_pair = KeyPair::new();

    let private_key_file_str = hex::encode(key_pair.private_key());
    write_file("secret.txt", &private_key_file_str)
        .with_context(|| "Could not create secret.txt file")?;

    let schema_name =
        SchemaName::new(name).with_context(|| format!("Invalid schema name: '{name}'"))?;
    let schema_description = SchemaDescription::new("")?;

    let mut schema_fields = SchemaFields::new();
    schema_fields.insert(
        "my_field".to_string(),
        crate::files::SchemaField::Field {
            field_type: FieldType::String,
        },
    );

    let mut schema_file = SchemaFile::new();
    schema_file.add_schema(&schema_name, &schema_description, &schema_fields);

    let schema_file_str = toml::to_string_pretty(&schema_file)?;
    write_file("schema.toml", &schema_file_str)
        .with_context(|| "Could not create schema.toml file")?;

    println!("Generated and stored private key in secret.txt and template schema.toml file");

    Ok(())
}
