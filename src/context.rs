use std::ops::Deref;
use std::path::PathBuf;
use std::sync::Arc;

use p2panda_rs::storage_provider::traits::{DocumentStore, EntryStore, LogStore, OperationStore};
use p2panda_rs::test_utils::memory_store::MemoryStore;

pub struct InnerContext<S>
where
    S: EntryStore + OperationStore + LogStore + DocumentStore,
{
    pub store: S,
    pub schema_path: PathBuf,
    pub lock_path: PathBuf,
}

pub struct Context<S: EntryStore + OperationStore + LogStore + DocumentStore = MemoryStore>(
    pub Arc<InnerContext<S>>,
);

impl<S> Context<S>
where
    S: EntryStore + OperationStore + LogStore + DocumentStore,
{
    pub fn new(store: S, schema_path: &PathBuf, lock_path: &PathBuf) -> Self {
        Self(Arc::new(InnerContext {
            store,
            schema_path: schema_path.clone(),
            lock_path: lock_path.clone(),
        }))
    }
}

impl<S> Clone for Context<S>
where
    S: EntryStore + OperationStore + LogStore + DocumentStore,
{
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<S> Deref for Context<S>
where
    S: EntryStore + OperationStore + LogStore + DocumentStore,
{
    type Target = InnerContext<S>;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}
