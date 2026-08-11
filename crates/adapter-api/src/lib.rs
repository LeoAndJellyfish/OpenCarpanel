//! Contracts shared by all `OpenCarpanel` game adapters.

mod adapter;

pub use adapter::{
    AdapterDescriptor, AdapterError, AdapterId, AdapterIdError, AdapterOutput, CapabilitySet,
    GameAdapter,
};
