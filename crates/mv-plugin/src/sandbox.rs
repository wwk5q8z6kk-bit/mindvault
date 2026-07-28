//! Permission gate for WASM plugin sandboxing.

use crate::manifest::PluginPermission;

/// Gates host-function calls against the plugin's declared permissions.
pub struct PermissionGate {
    permissions: Vec<PluginPermission>,
}

impl PermissionGate {
    pub fn new(permissions: Vec<PluginPermission>) -> Self {
        Self { permissions }
    }

    /// Check whether `action` is allowed by the plugin's permissions.
    pub fn check(&self, action: &str) -> Result<(), String> {
        let required = match action {
            "mv_read_node" => PluginPermission::ReadNodes,
            "mv_search" => PluginPermission::Search,
            "mv_write_node" => PluginPermission::WriteNodes,
            "mv_read_work_orders" => PluginPermission::ReadWorkOrders,
            "mv_log" => return Ok(()), // logging is always allowed
            _ => return Err(format!("unknown action: {action}")),
        };
        if self.permissions.contains(&required) {
            Ok(())
        } else {
            Err(format!(
                "permission denied: {required:?} required for {action}"
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_always_allowed() {
        let gate = PermissionGate::new(vec![]);
        assert!(gate.check("mv_log").is_ok());
    }

    #[test]
    fn read_requires_permission() {
        let gate = PermissionGate::new(vec![]);
        assert!(gate.check("mv_read_node").is_err());

        let gate = PermissionGate::new(vec![PluginPermission::ReadNodes]);
        assert!(gate.check("mv_read_node").is_ok());
    }

    #[test]
    fn write_requires_permission() {
        let gate = PermissionGate::new(vec![PluginPermission::ReadNodes]);
        assert!(gate.check("mv_write_node").is_err());

        let gate = PermissionGate::new(vec![PluginPermission::WriteNodes]);
        assert!(gate.check("mv_write_node").is_ok());
    }

    /// Constitutional law 3: extensions reach capabilities through the same
    /// governed boundary as first-party clients, never the database.
    #[test]
    fn reading_the_execution_graph_requires_permission() {
        // Holding every other permission grants nothing here.
        let gate = PermissionGate::new(vec![
            PluginPermission::ReadNodes,
            PluginPermission::WriteNodes,
            PluginPermission::Search,
            PluginPermission::GraphAccess,
        ]);
        assert!(gate.check("mv_read_work_orders").is_err());

        let gate = PermissionGate::new(vec![PluginPermission::ReadWorkOrders]);
        assert!(gate.check("mv_read_work_orders").is_ok());
    }

    /// There is no write path into the execution graph, and the gate fails
    /// closed on anything it does not recognize — so a plugin cannot reach one
    /// by guessing a method name.
    #[test]
    fn extensions_cannot_command_the_execution_graph() {
        let gate = PermissionGate::new(vec![
            PluginPermission::ReadWorkOrders,
            PluginPermission::WriteNodes,
        ]);
        for forbidden in [
            "mv_write_work_orders",
            "mv_record_gate",
            "mv_approve_run",
            "mv_admit_work_order",
        ] {
            assert!(
                gate.check(forbidden).is_err(),
                "{forbidden} must not be reachable: an extension that produced \
                 gate evidence would be manufacturing the governance that \
                 constrains it"
            );
        }
    }

    #[test]
    fn search_requires_permission() {
        let gate = PermissionGate::new(vec![]);
        assert!(gate.check("mv_search").is_err());

        let gate = PermissionGate::new(vec![PluginPermission::Search]);
        assert!(gate.check("mv_search").is_ok());
    }

    #[test]
    fn unknown_action_rejected() {
        let gate = PermissionGate::new(vec![
            PluginPermission::ReadNodes,
            PluginPermission::WriteNodes,
            PluginPermission::Search,
        ]);
        assert!(gate.check("mv_delete_everything").is_err());
    }
}
