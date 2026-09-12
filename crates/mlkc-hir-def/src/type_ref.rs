use mlkc_la_arena::Idx;

use crate::path::PathId;

pub type TypeRefId = Idx<TypeRef>;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypeRef {
    Path(PathId),
}
