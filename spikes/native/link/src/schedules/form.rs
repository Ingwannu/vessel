//! Existing unary calls through the immutable selected-connection fence. The
//! containing Hub owns the shared permit/deadline/event delivery, not this file.
use super::ScheduleError;
use crate::connections::SelectedConnection;
use vessel_app_view::schedules::editor::{
    definition::Save,
    io::{Operation, Targets, Value},
};

pub(super) async fn run(
    client: &SelectedConnection,
    operation: &Operation,
) -> Result<Value, ScheduleError> {
    match operation {
        Operation::Targets => {
            let info = client.call(vessel_proto::GetNode {}).await?;
            let harnesses = client.call(vessel_proto::ListHarnesses {}).await?;
            let agents = client.call(vessel_proto::agents::ListAgents {}).await?;
            let projects = client
                .call(vessel_proto::workspace::ListProjects {})
                .await?
                .projects;
            Ok(Value::Targets(Box::new(Targets {
                projects,
                agents,
                harnesses,
                timezone: info.timezone,
            })))
        }
        Operation::Options(request) => Ok(Value::Options(client.call(request.clone()).await?)),
        Operation::Preview(request) => Ok(Value::Preview(client.call(request.clone()).await?)),
        Operation::Save(save) => {
            let info = match save.as_ref() {
                Save::Create(request) => client.call(request.clone()).await?,
                Save::Update(request) => client.call(request.clone()).await?,
            };
            Ok(Value::Saved(Box::new(info)))
        }
    }
}
