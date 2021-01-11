use crate::client::module::{Healthcheck, ModuleKind, TermSignal};
use crate::daemon::api::{
    ApiExeHealthcheck, ApiHealthcheck, ApiKind, ApiTermSignal,
};

impl From<&ModuleKind> for ApiKind {
    fn from(kind: &ModuleKind) -> ApiKind {
        match kind {
            ModuleKind::Service => ApiKind::Service,
            ModuleKind::Task => ApiKind::Task,
            ModuleKind::Check => ApiKind::Task,
            ModuleKind::Group => ApiKind::Task,
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

impl From<&Healthcheck> for ApiHealthcheck {
    fn from(healthcheck: &Healthcheck) -> ApiHealthcheck {
        match healthcheck {
            Healthcheck::Exec(exec) => {
                ApiHealthcheck::Executable(ApiExeHealthcheck {
                    command: exec.command.clone(),
                    working_dir: exec.working_dir.clone(),
                })
            }
        }
    }
}
