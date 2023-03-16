use crate::context::Context;

pub fn publish(context: Context, endpoint: &str) {
    // @TODO
    // Parse entries + operations from schema.lock, validate them
    // .. Store them in storage provider
    // .. Materialize a Schema Document from them, validate it
    // Compare operations w. what the server has
    // .. show diff to user
    // Ask user to confirm it
    // Send new entries + operations via GraphQL to node
}
