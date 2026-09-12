use mlkc_la_arena::ArenaMap;
use rustc_hash::FxHashMap;

use crate::{
    id::{DefKey, LocalUseId, ModuleId, ModuleUseId},
    item_data::Visibility,
    name::Name,
    path_syntax::PlainPathId,
    project_graph::{ProjectGraph, ProjectId},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UseResolution {
    Resolved(DefKey),
    Unresolved,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct ModuleUseMap {
    pub resolutions: ArenaMap<ModuleUseId, UseResolution>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct BodyUseMap {
    pub uses: ArenaMap<LocalUseId, UseResolution>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ModuleLocator {
    /// Path that refers to real module.
    ///
    /// ```mlk
    /// use project::module
    /// fun foo() = module::id(42)
    /// ```
    Module(ModuleId),
    /// Path that refers to virtual module path.
    ///
    /// ```mlk
    /// use project::data
    /// fun foo() = data::submodule::id(42)
    /// ```
    Virtual {
        project_id: ProjectId,
        /// Full path with project name
        /// (to allow interning)
        path: PlainPathId,
    },
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct PerNs {
    pub ty: Option<(DefKey, Visibility)>,
    pub value: Option<(DefKey, Visibility)>,
    pub module: Option<(ModuleLocator, Visibility)>,
}

impl PerNs {
    pub fn is_empty(&self) -> bool {
        self.ty.is_none() && self.value.is_none()
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ModuleScope {
    pub entries: FxHashMap<Name, PerNs>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ProjectDefMap {
    pub modules: FxHashMap<ModuleId, ModuleScope>,
}

pub struct ResolveCtx<'a> {
    pub current_module: ModuleId,
    pub project_graph: &'a ProjectGraph,
    pub project_def_map: &'a ProjectDefMap,
}

fn resolve_plain_path(_path: PlainPathId, _ctx: &ResolveCtx) -> PerNs {
    todo!()
}
