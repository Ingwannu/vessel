//! Typed host-level skill administration; unavailable while `MCP_PARITY` is pending.
use crate::NodeClient;
use vessel_proto::skills::{
    GetSkillsStatus, ImportSkillsSelection, ReconcileSkills, SaveSkillsSelection, SkillsImported,
    SkillsSaved, SkillsSnapshot, UninstallSkills,
};
use vessel_rpc::CallError;

impl NodeClient {
    /// Host skills `status`; Admin only, direct node connection.
    ///
    /// # Errors
    /// Unsupported peers, admission, filesystem or recovery failures are returned.
    pub async fn skills_status(&self) -> Result<SkillsSnapshot, CallError> {
        self.call(GetSkillsStatus {}).await
    }
    /// Host skills `reconcile`; Admin only, direct node connection.
    ///
    /// # Errors
    /// Unsupported peers, admission, filesystem or recovery failures are returned.
    pub async fn reconcile_skills(&self) -> Result<SkillsSnapshot, CallError> {
        self.call(ReconcileSkills {}).await
    }
    /// Host skills `uninstall`; Admin only, direct node connection.
    ///
    /// # Errors
    /// Unsupported peers, admission, filesystem or recovery failures are returned.
    pub async fn uninstall_skills(&self) -> Result<SkillsSnapshot, CallError> {
        self.call(UninstallSkills {}).await
    }
    /// Host skills `save_selection`; Admin only, direct node connection.
    ///
    /// # Errors
    /// Unsupported peers, admission, filesystem or recovery failures are returned.
    pub async fn save_skills_selection(
        &self,
        request: SaveSkillsSelection,
    ) -> Result<SkillsSaved, CallError> {
        self.call(request).await
    }
    /// Host skills `import_selection`; Admin only, direct node connection.
    ///
    /// # Errors
    /// Unsupported peers, admission, filesystem or recovery failures are returned.
    pub async fn import_skills_selection(
        &self,
        request: ImportSkillsSelection,
    ) -> Result<SkillsImported, CallError> {
        self.call(request).await
    }
}
