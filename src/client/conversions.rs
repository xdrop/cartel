use crate::client::module::{ModuleKindV1, TermSignal};
use crate::daemon::api::{ApiKind, ApiTermSignal};

impl From<&ModuleKindV1> for ApiKind {
    fn from(kind: &ModuleKindV1) -> ApiKind {
        match kind {
            ModuleKindV1::Service => ApiKind::Service,
            ModuleKindV1::Task => ApiKind::Task,
            ModuleKindV1::Check => ApiKind::Task,
        }
    }
}

impl From<&TermSignal> for ApiTermSignal {
    fn from(signal: &TermSignal) -> ApiTermSignal {
        match signal {
            TermSignal::TERM => ApiTermSignal::TERM,
            TermSignal::KILL => ApiTermSignal::KILL,
            TermSignal::INT => ApiTermSignal::INT,
        }
    }
}
