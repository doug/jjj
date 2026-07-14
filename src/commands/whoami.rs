//! `jjj whoami` — show the resolved coordination identity for this process.
//!
//! In an agent swarm every write is attributed and every pod pushes to its own
//! single-writer bookmark. Both are resolved from a small precedence chain
//! (env override → sync-state file → jj config) that is otherwise invisible.
//! This command makes the resolution auditable so an operator can confirm a pod
//! is running under the identity and push ref they intended before it writes.

use crate::context::CommandContext;
use crate::error::Result;
use crate::storage::sync_state::SyncState;

/// Print the resolved actor, pod, and push bookmark. With `--json`, emit a
/// single object for scripting (e.g. a swarm supervisor asserting each pod's id).
pub fn execute(ctx: &CommandContext, json: bool) -> Result<()> {
    let store = &ctx.store;
    let actor = store.get_current_user()?;
    let state = SyncState::load(store.meta_path());
    let pod = state.pod.clone();
    let push_bookmark = state.push_bookmark();

    if json {
        let obj = serde_json::json!({
            "actor": actor,
            "pod": pod,
            "push_bookmark": push_bookmark,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&obj).map_err(crate::error::JjjError::JsonParse)?
        );
    } else {
        println!("actor:         {actor}");
        println!("pod:           {}", pod.as_deref().unwrap_or("(none)"));
        println!("push bookmark: {push_bookmark}");
    }
    Ok(())
}
