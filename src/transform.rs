//! Pure, I/O-free message-role-coercion logic.
//!
//! vLLM's chat template rejects any `role: "system"` message that is not at
//! index 0 of the `messages` array. This module rewrites every *other*
//! system-role message to `role: "user"`, prepending a configurable notice
//! prefix, before the request reaches the upstream model server.
