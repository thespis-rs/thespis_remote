//! Test error handling.
//!
//! For all possible errors in thespis_remote, test:
//! - local result through pharos
//! - remote result
//! - peer gone scenarios
//!
//! When using send, the remote should never receive any feedback as soon as the msg has been sent
//! successfully over the wire.
//!
//! Errors:
//! - for all sending and receiving, the underlying transport can error.
//! - for all received messages:
//!   - fail to deserialize
//!   - max message size exceeded
//!   - remote errors: what happens on processing them
//!
//! - for all requests:
//!   - sid/cid combination makes sense, eg. sid null and full.
//!   - must have service map
//!   - must have handler
//!   - handler must be alive
//!   - msg must deserialize
//!
//!   - for calls:
//!     - TODO: all operations of setting up the response channel
//!     - timeouts
//!
//! - TODO: everything specific to relaying.
