use std::collections::btree_map::Iter;
use std::collections::BTreeMap;

use p2panda_rs::schema::{FieldName, SchemaDescription, SchemaId, SchemaName};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum FieldType {
    #[serde(rename = "bool")]
    Boolean,

    #[serde(rename = "float")]
    Float,

    #[serde(rename = "int")]
    Integer,

    #[serde(rename = "str")]
    String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum RelationType {
    Relation,
    RelationList,
    PinnedRelation,
    PinnedRelationList,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase", deny_unknown_fields)]
pub enum RelationId {
    Name(SchemaName),
    Id(SchemaId),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase", deny_unknown_fields)]
pub enum RelationSource {
    Git(String),
    Path(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RelationSchema {
    #[serde(flatten)]
    pub id: RelationId,

    #[serde(flatten)]
    pub external: Option<RelationSource>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged, deny_unknown_fields)]
pub enum SchemaField {
    Field {
        #[serde(rename = "type")]
        field_type: FieldType,
    },
    Relation {
        #[serde(rename = "type")]
        field_type: RelationType,
        schema: RelationSchema,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SchemaFields(BTreeMap<FieldName, SchemaField>);

impl SchemaFields {
    pub fn new() -> Self {
        Self(BTreeMap::new())
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn insert(&mut self, field_name: FieldName, field: SchemaField) {
        self.0.insert(field_name, field);
    }

    pub fn iter(&self) -> Iter<FieldName, SchemaField> {
        self.0.iter()
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SchemaItem {
    pub description: SchemaDescription,
    pub fields: SchemaFields,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SchemaFile(BTreeMap<SchemaName, SchemaItem>);

impl SchemaFile {
    pub fn new() -> Self {
        Self(BTreeMap::new())
    }

    pub fn iter(&self) -> Iter<SchemaName, SchemaItem> {
        self.0.iter()
    }

    pub fn add_schema(
        &mut self,
        name: &SchemaName,
        description: &SchemaDescription,
        fields: &SchemaFields,
    ) {
        self.0.insert(
            name.clone(),
            SchemaItem {
                description: description.to_owned(),
                fields: fields.to_owned(),
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{
        FieldType, RelationId, RelationSchema, RelationSource, RelationType, SchemaField,
        SchemaFields, SchemaFile, SchemaItem,
    };

    #[test]
    fn lala() {
        let mut schema_item = SchemaItem {
            description: "lala".parse().unwrap(),
            fields: SchemaFields::new(),
        };

        schema_item.fields.insert(
            "some_field".into(),
            SchemaField::Relation {
                field_type: RelationType::RelationList,
                schema: RelationSchema {
                    id: RelationId::Name("test".parse().unwrap()),
                    external: Some(RelationSource::Git(
                        "https://github.com/pigoz/effect-crashcourse".into(),
                    )),
                },
            },
        );

        schema_item.fields.insert(
            "another_field".into(),
            SchemaField::Field {
                field_type: FieldType::String,
            },
        );

        let mut schema_file = SchemaFile::new();
        schema_file.0.insert("test".parse().unwrap(), schema_item);
        let schema_file_str = toml::to_string_pretty(&schema_file).unwrap();
        println!("{schema_file_str}");
    }
}
