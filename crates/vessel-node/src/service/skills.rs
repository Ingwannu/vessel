//! Admin-only, local host skills. Guard and method table reject agents/hops.
use super::{Connection, NodeService};
use crate::skills::SkillsError;
use vessel_proto::skills::{
    GetSkillsStatus, ImportSkillsSelection, ReconcileSkills, SaveSkillsSelection, SkillsImported,
    SkillsSaved, SkillsSnapshot, UninstallSkills,
};
use vessel_proto::{ErrorCode, WireError};

impl NodeService {
    pub(crate) async fn skills_status(
        &self,
        _: &Connection,
        _: GetSkillsStatus,
    ) -> Result<SkillsSnapshot, WireError> {
        self.skills.status().await.map_err(wire)
    }
    pub(crate) async fn reconcile_skills(
        &self,
        _: &Connection,
        _: ReconcileSkills,
    ) -> Result<SkillsSnapshot, WireError> {
        self.skills.reconcile().await.map_err(wire)
    }
    pub(crate) async fn uninstall_skills(
        &self,
        _: &Connection,
        _: UninstallSkills,
    ) -> Result<SkillsSnapshot, WireError> {
        self.skills.uninstall().await.map_err(wire)
    }
    pub(crate) async fn save_skills_selection(
        &self,
        _: &Connection,
        request: SaveSkillsSelection,
    ) -> Result<SkillsSaved, WireError> {
        self.skills.save(request).await.map_err(wire)
    }
    pub(crate) async fn import_skills_selection(
        &self,
        _: &Connection,
        request: ImportSkillsSelection,
    ) -> Result<SkillsImported, WireError> {
        self.skills.import(request).await.map_err(wire)
    }
}
fn wire(error: SkillsError) -> WireError {
    let code = match &error {
        SkillsError::Busy | SkillsError::Stopping => ErrorCode::Conflict,
        SkillsError::Skill(vessel_fleet::orchestration::SkillError::UnknownSkill) => {
            ErrorCode::InvalidRequest
        }
        SkillsError::Skill(_)
        | SkillsError::Selection(_)
        | SkillsError::Store(_)
        | SkillsError::Poisoned
        | SkillsError::Lost(_) => ErrorCode::Internal,
    };
    tracing::warn!(%error, "host skills command failed");
    WireError::new(
        code,
        "host skills command failed; the node retained recovery evidence",
    )
}
