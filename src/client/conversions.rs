use crate::client::module::ModuleKindV1;
use crate::daemon::api::ApiKind;

impl From<&ModuleKindV1> for ApiKind {
    fn from(kind: &ModuleKindV1) -> ApiKind {
        match kind {
            ModuleKindV1::Service => ApiKind::Service,
            ModuleKindV1::Task => ApiKind::Task,
            ModuleKindV1::Check => ApiKind::Task,
        }
    }
}
