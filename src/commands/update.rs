use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;

use anyhow::{anyhow, bail, Context as ErrorContext, Result};
use async_trait::async_trait;
use p2panda_rs::api::publish;
use p2panda_rs::document::traits::AsDocument;
use p2panda_rs::document::DocumentViewId;
use p2panda_rs::entry::traits::AsEncodedEntry;
use p2panda_rs::hash::Hash;
use p2panda_rs::identity::KeyPair;
use p2panda_rs::operation::decode::decode_operation;
use p2panda_rs::operation::encode::encode_operation;
use p2panda_rs::operation::traits::Schematic;
use p2panda_rs::operation::{
    Operation, OperationAction, OperationBuilder, OperationValue, PinnedRelationList,
};
use p2panda_rs::schema::system::{SchemaFieldView, SchemaView};
use p2panda_rs::schema::{
    FieldName, FieldType as PandaFieldType, Schema as PandaSchema, SchemaDescription, SchemaId,
    SchemaName,
};
use p2panda_rs::storage_provider::traits::DocumentStore;
use p2panda_rs::test_utils::memory_store::helpers::send_to_store;

use crate::context::Context;
use crate::files::{Commit, FieldType, LockFile, RelationType, SchemaFile};
use crate::schema::Schema;

fn write_file(path: &str, content: &str) -> Result<()> {
    let mut file = File::create(&path)?;
    file.write_all(content.as_bytes())?;
    Ok(())
}

struct Executor {
    context: Context,
    key_pair: KeyPair,
    commits: Vec<Commit>,
}

impl Executor {
    pub async fn sign(&mut self, operation: &Operation, schema_id: SchemaId) -> Result<Hash> {
        let schema = PandaSchema::get_system(schema_id)?;

        let encoded_operation = encode_operation(&operation)?;

        let (encoded_entry, _) =
            send_to_store(&self.context.store, &operation, &schema, &self.key_pair)
                .await
                .map_err(|err| anyhow!("{err}"))?;

        let entry_hash = encoded_entry.hash();

        self.commits
            .push(Commit::new(&encoded_entry, &encoded_operation));

        Ok(entry_hash)
    }
}

#[async_trait]
trait Executable {
    async fn execute(&self, executor: &mut Executor) -> Result<DocumentViewId>;
}

#[derive(Debug, Clone, PartialEq)]
enum FieldTypePlan {
    Field(FieldType),
    Relation(RelationType, SchemaPlan),
}

#[derive(Debug, Clone, PartialEq)]
struct FieldPlan {
    name: FieldName,
    current: Option<SchemaFieldView>,
    field_type: FieldTypePlan,
}

#[async_trait]
impl Executable for FieldPlan {
    async fn execute(&self, executor: &mut Executor) -> Result<DocumentViewId> {
        let field_type = match &self.field_type {
            FieldTypePlan::Field(FieldType::String) => PandaFieldType::String,
            FieldTypePlan::Field(FieldType::Boolean) => PandaFieldType::Boolean,
            FieldTypePlan::Field(FieldType::Float) => PandaFieldType::Float,
            FieldTypePlan::Field(FieldType::Integer) => PandaFieldType::Integer,
            FieldTypePlan::Relation(relation, schema_plan) => {
                let view_id = schema_plan.execute(executor).await?;
                let schema_id = SchemaId::new_application(&schema_plan.name, &view_id);

                match relation {
                    RelationType::Relation => PandaFieldType::Relation(schema_id),
                    RelationType::RelationList => PandaFieldType::RelationList(schema_id),
                    RelationType::PinnedRelation => PandaFieldType::PinnedRelation(schema_id),
                    RelationType::PinnedRelationList => {
                        PandaFieldType::PinnedRelationList(schema_id)
                    }
                }
            }
        };

        let operation: Option<Operation> = match &self.current {
            Some(current) => {
                if current.field_type() != &field_type {
                    let operation = OperationBuilder::new(&SchemaId::SchemaFieldDefinition(1))
                        .action(OperationAction::Update)
                        .previous(current.id())
                        .fields(&[("type", field_type.clone().into())])
                        .build()?;

                    Some(operation)
                } else {
                    None
                }
            }
            None => {
                let operation = OperationBuilder::new(&SchemaId::SchemaFieldDefinition(1))
                    .action(OperationAction::Create)
                    .fields(&[
                        ("name", self.name.clone().into()),
                        ("type", field_type.into()),
                    ])
                    .build()?;

                Some(operation)
            }
        };

        match operation {
            Some(operation) => {
                let entry_hash = executor
                    .sign(&operation, SchemaId::SchemaFieldDefinition(1))
                    .await?;

                Ok(entry_hash.into())
            }
            None => Ok(self.current.as_ref().unwrap().id().to_owned().into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct SchemaPlan {
    name: SchemaName,
    current: Option<SchemaView>,
    description: SchemaDescription,
    fields: Vec<FieldPlan>,
}

#[async_trait]
impl Executable for SchemaPlan {
    async fn execute(&self, executor: &mut Executor) -> Result<DocumentViewId> {
        let mut schema_fields: Vec<DocumentViewId> = Vec::new();

        for field in &self.fields {
            let field_view_id = field.execute(executor).await?;
            schema_fields.push(field_view_id);
        }

        let mut fields: Vec<(&str, OperationValue)> = vec![("name", self.name.to_string().into())];

        let operation: Option<Operation> = match &self.current {
            Some(current) => {
                let mut update = false;

                if &self.description.to_string() != current.description() {
                    fields.push(("description", self.description.to_string().into()));
                    update = true;
                }

                let pinned_list = PinnedRelationList::new(schema_fields.clone());
                if current.fields() != &pinned_list {
                    fields.push(("fields", schema_fields.clone().into()));
                    update = true;
                }

                if update {
                    let operation = OperationBuilder::new(&SchemaId::SchemaDefinition(1))
                        .previous(current.view_id())
                        .action(OperationAction::Update)
                        .fields(&fields)
                        .build()?;

                    Some(operation)
                } else {
                    None
                }
            }
            None => {
                fields.push(("description", self.description.to_string().into()));
                fields.push(("fields", schema_fields.into()));

                let operation = OperationBuilder::new(&SchemaId::SchemaDefinition(1))
                    .action(OperationAction::Create)
                    .fields(&fields)
                    .build()?;

                Some(operation)
            }
        };

        match operation {
            Some(operation) => {
                let entry_hash = executor
                    .sign(&operation, SchemaId::SchemaDefinition(1))
                    .await?;

                Ok(entry_hash.into())
            }
            None => Ok(self.current.as_ref().unwrap().view_id().clone().into()),
        }
    }
}

async fn do_it(
    current: HashMap<SchemaName, (PandaSchema, SchemaView, Vec<SchemaFieldView>)>,
    _planned: Vec<Schema>,
    context: Context,
) -> Result<Vec<Commit>> {
    let schema_name = SchemaName::new("schema_a").unwrap();

    let schema_current = current
        .get(&schema_name)
        .and_then(|schema| Some(schema.1.clone()));

    let schema_field = current
        .get(&schema_name)
        .and_then(|schema| schema.2.iter().find(|field| field.name() == "a").cloned());

    let field_a = FieldPlan {
        name: "a".into(),
        current: schema_field,
        field_type: FieldTypePlan::Field(FieldType::String),
    };

    let schema_field = current
        .get(&schema_name)
        .and_then(|schema| schema.2.iter().find(|field| field.name() == "b").cloned());

    let field_b = FieldPlan {
        name: "b".into(),
        current: schema_field,
        field_type: FieldTypePlan::Field(FieldType::Integer),
    };

    let schema_field = current
        .get(&schema_name)
        .and_then(|schema| schema.2.iter().find(|field| field.name() == "c").cloned());

    let field_c = FieldPlan {
        name: "c".into(),
        current: schema_field,
        field_type: FieldTypePlan::Field(FieldType::Float),
    };

    let schema_a = SchemaPlan {
        name: schema_name,
        current: schema_current,
        description: SchemaDescription::new("test").unwrap(),
        fields: vec![field_a, field_b, field_c],
    };

    // =====

    let schema_name = SchemaName::new("schema_b").unwrap();

    let schema_current = current
        .get(&schema_name)
        .and_then(|schema| Some(schema.1.clone()));

    let schema_field = current
        .get(&schema_name)
        .and_then(|schema| schema.2.iter().find(|field| field.name() == "d").cloned());

    let field_d = FieldPlan {
        name: "d".into(),
        current: schema_field,
        field_type: FieldTypePlan::Relation(RelationType::RelationList, schema_a),
    };

    let schema_field = current
        .get(&schema_name)
        .and_then(|schema| schema.2.iter().find(|field| field.name() == "e").cloned());

    let field_e = FieldPlan {
        name: "e".into(),
        current: schema_field,
        field_type: FieldTypePlan::Field(FieldType::Boolean),
    };

    let schema_field = current
        .get(&schema_name)
        .and_then(|schema| schema.2.iter().find(|field| field.name() == "f").cloned());

    let field_f = FieldPlan {
        name: "f".into(),
        current: schema_field,
        field_type: FieldTypePlan::Field(FieldType::Boolean),
    };

    let schema_b = SchemaPlan {
        name: SchemaName::new("schema_b").unwrap(),
        current: schema_current,
        description: SchemaDescription::new("testt").unwrap(),
        fields: vec![field_d, field_e, field_f],
    };

    // =====

    let mut executor = Executor {
        context,
        key_pair: KeyPair::new(),
        commits: Vec::new(),
    };

    schema_b.execute(&mut executor).await?;

    return Ok(executor.commits);
}

pub async fn update(context: Context, private_key_path: &PathBuf) -> Result<()> {
    let schema_file_str = fs::read_to_string(&context.schema_path)?;
    let schema_file: SchemaFile =
        toml::from_str(&schema_file_str).with_context(|| "Invalid schema.toml format")?;

    let lock_file_str = fs::read_to_string(&context.lock_path);
    let mut lock_file = match lock_file_str {
        Ok(file_str) => {
            let lock_file: LockFile = toml::from_str(&file_str)?;
            lock_file
        }
        Err(_) => LockFile::new(vec![]),
    };

    let private_key_file_str = fs::read_to_string(&private_key_path)?;
    let key_pair = KeyPair::from_private_key_str(&private_key_file_str)?;

    println!("{}", key_pair.public_key());

    // GET THE PLANNED SCHEMAS

    let mut planned_schemas: Vec<Schema> = Vec::new();
    let mut built_schemas: HashMap<SchemaName, (PandaSchema, SchemaView, Vec<SchemaFieldView>)> =
        HashMap::new();

    for (schema_name, schema_item) in schema_file.iter() {
        let schema = Schema::new(schema_name, &schema_item.description, &schema_item.fields);

        if schema_item.fields.len() == 0 {
            bail!("Schema {schema_name} doesn't have any fields");
        }

        planned_schemas.push(schema);
    }

    // GET THE CURRENT SCHEMAS

    if let Some(commits) = &lock_file.commits {
        for commit in commits {
            let plain_operation = decode_operation(&commit.operation)?;

            let schema = match plain_operation.schema_id() {
            SchemaId::SchemaDefinition(version) => PandaSchema::get_system(
                SchemaId::SchemaDefinition(*version),
            )
            .with_context(|| {
                "Incompatible system schema definition version {version} used in schema.lock"
            })?,
            SchemaId::SchemaFieldDefinition(version) => PandaSchema::get_system(
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
                PandaSchema::from_views(schema_view.clone(), schema_field_views.clone()).unwrap();

            if built_schemas
                .insert(
                    schema.id().name(),
                    (schema, schema_view, schema_field_views),
                )
                .is_some()
            {
                bail!("Duplicate schema name detected in schema.lock");
            }
        }
    }

    // DO IT

    let mut new_commits = do_it(built_schemas, planned_schemas, context).await?;
    println!("Writing {} new commits", new_commits.len());

    // TODO: ASK IF WE'RE OKAY W. THAT

    let mut commits: Vec<Commit> = Vec::new();

    if let Some(current_commits) = lock_file.commits.as_mut() {
        commits.append(current_commits);
    }
    commits.append(&mut new_commits);

    // WRITE TO .LOCK FILE

    let lock_file = LockFile::new(commits);

    let lock_file_str = format!(
        "{}\n\n{}",
        "# This file is automatically generated by fishy.\n# It is not intended for manual editing.",
        toml::to_string_pretty(&lock_file)?
    );

    write_file("schema.lock", &lock_file_str)
        .with_context(|| "Could not create schema.lock file")?;

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
