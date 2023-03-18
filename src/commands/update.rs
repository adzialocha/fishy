use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use anyhow::{bail, Context as ErrorContext, Result};
use p2panda_rs::api::publish;
use p2panda_rs::document::traits::AsDocument;
use p2panda_rs::identity::KeyPair;
use p2panda_rs::operation::decode::decode_operation;
use p2panda_rs::operation::traits::Schematic;
use p2panda_rs::schema::system::{SchemaFieldView, SchemaView};
use p2panda_rs::schema::{SchemaId, SchemaName};
use p2panda_rs::storage_provider::traits::DocumentStore;

use crate::context::Context;
use crate::files::{LockFile, SchemaFile};
use crate::schema::Schema;

#[derive(Debug)]
struct Plan {}

fn plan_build(plan: &Schema, previous: Option<&p2panda_rs::schema::Schema>) -> Plan {
    Plan {}
}

pub async fn update(context: Context, private_key_path: &PathBuf) -> Result<()> {
    let schema_file_str = fs::read_to_string(&context.schema_path)?;
    let schema_file: SchemaFile =
        toml::from_str(&schema_file_str).with_context(|| "Invalid schema.toml format")?;

    let lock_file_str = fs::read_to_string(&context.lock_path)?;
    let lock_file: LockFile = toml::from_str(&lock_file_str)?;

    let private_key_file_str = fs::read_to_string(&private_key_path)?;
    let key_pair = KeyPair::from_private_key_str(&private_key_file_str)?;

    println!("{:?}", schema_file);
    println!("{:?}", lock_file);
    println!("{}", key_pair.public_key());

    let mut planned_schemas: Vec<Schema> = Vec::new();
    let mut built_schemas: HashMap<SchemaName, p2panda_rs::schema::Schema> = HashMap::new();

    for (schema_name, schema_item) in schema_file.iter() {
        let schema = Schema::new(schema_name, &schema_item.description, &schema_item.fields);

        if schema_item.fields.len() == 0 {
            bail!("Schema {schema_name} doesn't have any fields");
        }

        planned_schemas.push(schema);
    }

    for commit in lock_file.commits {
        let plain_operation = decode_operation(&commit.operation)?;

        let schema = match plain_operation.schema_id() {
            SchemaId::SchemaDefinition(version) => p2panda_rs::schema::Schema::get_system(
                SchemaId::SchemaDefinition(*version),
            )
            .with_context(|| {
                "Incompatible system schema definition version {version} used in schema.lock"
            })?,
            SchemaId::SchemaFieldDefinition(version) => p2panda_rs::schema::Schema::get_system(
                SchemaId::SchemaFieldDefinition(*version),
            )
            .with_context(|| {
                "Incompatible system schema field definition version {version} used in schema.lock"
            })?,
            value => bail!("Invalid schema id '{value}' detected in schema.lock"),
        };

        publish(
            &context.store,
            schema,
            &commit.entry,
            &plain_operation,
            &commit.operation,
        )
        .await?;
    }

    let definition_documents = context
        .store
        .get_documents_by_schema(&SchemaId::SchemaDefinition(1))
        .await?;

    for definition_document in definition_documents {
        let document_view = definition_document.view().unwrap();
        let schema_view = SchemaView::try_from(document_view).unwrap();
        let mut schema_field_views: Vec<SchemaFieldView> = Vec::new();

        for field_view_id in schema_view.fields().iter() {
            let field_document = context
                .store
                .get_document_by_view_id(field_view_id)
                .await
                .unwrap()
                .unwrap();

            let schema_field_view =
                SchemaFieldView::try_from(field_document.view().unwrap()).unwrap();

            schema_field_views.push(schema_field_view);
        }

        let schema =
            p2panda_rs::schema::Schema::from_views(schema_view, schema_field_views).unwrap();

        if built_schemas.insert(schema.id().name(), schema).is_some() {
            bail!("Duplicate schema name detected in schema.lock");
        }
    }

    for planned_schema in &planned_schemas {
        let built_schema = built_schemas.get(planned_schema.name());
        let plan = plan_build(&planned_schema, built_schema);
        println!("{:?}", plan);
    }

    // 1. Parse schema.toml, validate it
    // 2. Parse schema.lock, validate it
    // 3. Load data from schema.lock and materialize it, validate it (log integrity)
    // 4. Go through every schema item defined in it and create a `Schema` instance from it, add it
    //    to a global schema array
    // 5. If schema is external, make sure that materialized data matches
    //    a) If "name" was specified we're done by just comparing the schema.toml with schema
    //    instance
    //    b) If "id" was specified, check if view id exists
    // 6. Go through every relation in schema and resolve dependencies
    //    a) If dependency already exists (is in the schema array) all good
    //    b) If dependency is external, download via git into dependencies folder or load it from
    //       local folder
    //    c) Repeat steps 1 - 3 for this schema relation

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
