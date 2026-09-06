//! `role-coercion-proxy` — an HTTP proxy sitting in front of a vLLM chat-completions
//! endpoint that rewrites mid-conversation `role: "system"` messages so vLLM's chat
//! template does not reject them.

fn main() {
    println!("role-coercion-proxy skeleton");
}
