use crate::{name::Name, path_syntax::PlainPathId};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Visibility {
    Public,
    Private,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UseData {
    pub path: PlainPathId,
    pub alias: Option<Name>,
    pub visibility: Visibility,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FunctionData {
    pub name: Name,
    pub visibility: Visibility,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ValueData {
    pub name: Name,
    pub visibility: Visibility,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ImplData {
    pub name: Option<Name>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ClassData {
    pub name: Name,
    pub visibility: Visibility,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConstData {
    pub name: Name,
    pub visibility: Visibility,
}
