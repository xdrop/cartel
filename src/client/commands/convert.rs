use crate::{
    client::{
        commands::deployer::ModuleToDeploy,
        module::{ModuleDefinition, ModuleMarker},
    },
    dependency::DependencyNode,
};

impl<'a> From<&'a DependencyNode<&ModuleDefinition, ModuleMarker>>
    for ModuleToDeploy<'a>
{
    fn from(
        dep_node: &'a DependencyNode<&ModuleDefinition, ModuleMarker>,
    ) -> Self {
        ModuleToDeploy {
            definition: dep_node.value,
            marker: dep_node.marker,
        }
    }
}

impl<'a> From<&'a ModuleDefinition> for ModuleToDeploy<'a> {
    fn from(definition: &'a ModuleDefinition) -> Self {
        ModuleToDeploy {
            definition,
            marker: Some(ModuleMarker::Instant),
        }
    }
}
