use std::collections::BTreeMap;

use p2panda_rs::schema::{FieldName, SchemaDescription, SchemaName};

use crate::files::SchemaFields;

#[derive(Clone, Debug, PartialEq)]
pub struct Schema {
    name: SchemaName,
    description: SchemaDescription,
    fields: SchemaFields,
}

impl Schema {
    pub fn new(name: &SchemaName, description: &SchemaDescription, fields: &SchemaFields) -> Self {
        Self {
            name: name.clone(),
            description: description.clone(),
            fields: fields.clone(),
        }
    }

    pub fn name(&self) -> &SchemaName {
        &self.name
    }

    pub fn description(&self) -> &SchemaDescription {
        &self.description
    }

    pub fn fields(&self) -> &SchemaFields {
        &self.fields
    }
}
