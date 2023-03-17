use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::schema::Schema;

#[derive(Debug, Serialize, Deserialize)]
pub struct SchemaField {
    #[serde(rename = "type")]
    field_type: String,
}

impl SchemaField {
    fn new(field_type: String) -> Self {
        Self { field_type }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SchemaItem {
    description: String,
    fields: BTreeMap<String, SchemaField>,
}

impl From<&Schema> for SchemaItem {
    fn from(value: &Schema) -> Self {
        let mut fields = BTreeMap::new();

        value.fields().iter().for_each(|(field_name, field_type)| {
            fields.insert(
                field_name.to_string(),
                SchemaField::new(field_type.to_string()),
            );
        });

        Self {
            description: value.description().to_string(),
            fields,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SchemaFile(BTreeMap<String, SchemaItem>);

impl SchemaFile {
    pub fn new() -> Self {
        Self(BTreeMap::new())
    }

    pub fn add_schema(&mut self, schema: &Schema) {
        self.0.insert(schema.name().to_string(), schema.into());
    }
}
