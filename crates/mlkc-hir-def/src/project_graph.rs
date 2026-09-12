use mlkc_la_arena::{ArenaMap, Idx};
use rustc_hash::FxHashMap;

use crate::{id::ModuleId, name::Name};

pub type ProjectId = Idx<ProjectData>;

#[derive(Debug, Clone)]
pub struct ProjectData {
    pub root_module: ModuleId,
    pub dependencies: FxHashMap<Name, ProjectId>,
}

pub struct ProjectGraph {
    pub projects: ArenaMap<ProjectId, ProjectData>,
    pub module_to_projects: FxHashMap<ModuleId, ProjectId>,
}
