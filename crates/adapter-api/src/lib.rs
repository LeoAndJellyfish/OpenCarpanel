//! Contracts shared by all `OpenSimDash` game adapters.

mod adapter;

pub use adapter::{
    AdapterDescriptor, AdapterError, AdapterId, AdapterIdError, AdapterOutput, CapabilitySet,
    GameAdapter,
};
