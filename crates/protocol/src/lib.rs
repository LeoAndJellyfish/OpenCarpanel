//! Wire messages shared by the Host and generated `TypeScript` bindings.

mod message;
mod schema;

pub use message::{
    CapabilitiesMessage, ClientHello, ClientMessage, ClientPayload, ErrorCode, ErrorMessage,
    EventAckMessage, EventMessage, PROTOCOL_VERSION, ResyncRequiredMessage, ServerHello,
    ServerMessage, ServerPayload, SnapshotMessage, StaleMessage, StaleReason, WireDecodeError,
    decode_client_message, decode_server_message,
};
pub use schema::{
    SchemaDocument, SchemaExportError, generate_schema_documents, write_schema_documents,
};
