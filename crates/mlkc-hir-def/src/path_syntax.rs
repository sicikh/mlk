use std::{fmt, iter};

use itertools::Itertools as _;
use mlkc_intern::{Interned, impl_internable};

use crate::name::Name;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PathSyntax {
    Plain(PlainPathId),
    // Normal(Box<NormalPath>),
}

impl PathSyntax {
    pub fn plain_path(&self) -> &PlainPath {
        match self {
            PathSyntax::Plain(path) => path,
        }
    }

    pub fn qualifier(&self) -> Option<PathSyntax> {
        match self {
            PathSyntax::Plain(plain_path) => {
                if plain_path.is_ident() {
                    return None;
                }

                Some(PathSyntax::Plain(Interned::new(PlainPath::from_segments(
                    plain_path
                        .segments()
                        .iter()
                        .take(plain_path.len() - 1)
                        .cloned(),
                ))))
            },
        }
    }

    pub fn segments(&self) -> PathSegments<'_> {
        match self {
            PathSyntax::Plain(plain_path) => {
                PathSegments {
                    segments: plain_path.segments(),
                }
            },
        }
    }
}

pub type PlainPathId = Interned<PlainPath>;

/// Path as plain list of identifiers.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PlainPath {
    segments: Vec<Name>,
}

impl PlainPath {
    pub fn from_segments(segments: impl IntoIterator<Item = Name>) -> Self {
        let mut segments: Vec<_> = segments.into_iter().collect();
        segments.shrink_to_fit();
        PlainPath { segments }
    }

    pub fn segments(&self) -> &[Name] {
        &self.segments
    }

    pub fn push_segment(&mut self, segment: Name) {
        self.segments.push(segment);
    }

    pub fn pop_segment(&mut self) -> Option<Name> {
        self.segments.pop()
    }

    #[expect(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.segments.len()
    }

    pub fn text_len(&self) -> usize {
        self.segments.iter().map(|s| s.as_str().len()).sum()
    }

    pub fn as_ident(&self) -> Option<&Name> {
        match self.segments.as_slice() {
            [name] => Some(name),
            _ => None,
        }
    }

    pub fn is_ident(&self) -> bool {
        self.as_ident().is_some()
    }
}

impl Extend<Name> for PlainPath {
    fn extend<T: IntoIterator<Item = Name>>(&mut self, iter: T) {
        self.segments.extend(iter);
    }
}

impl fmt::Display for PlainPath {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        #[expect(unstable_name_collisions)]
        self.segments
            .iter()
            .map(|s| s.as_str())
            .intersperse(".")
            .try_for_each(|s| write!(f, "{}", s))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PathSegment<'a> {
    pub name: &'a Name,
}

impl PathSegment<'_> {
    pub const MISSING: PathSegment<'static> = PathSegment {
        name: &Name::missing(),
    };
}

#[derive(Debug, Clone, Copy)]
pub struct PathSegments<'a> {
    segments: &'a [Name],
}

impl<'a> PathSegments<'a> {
    pub const EMPTY: PathSegments<'static> = PathSegments { segments: &[] };
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub fn len(&self) -> usize {
        self.segments.len()
    }
    pub fn first(&self) -> Option<PathSegment<'a>> {
        self.get(0)
    }
    pub fn last(&self) -> Option<PathSegment<'a>> {
        self.get(self.len().checked_sub(1)?)
    }

    pub fn get(&self, idx: usize) -> Option<PathSegment<'a>> {
        let res = PathSegment {
            name: self.segments.get(idx)?,
        };
        Some(res)
    }

    pub fn skip(&self, len: usize) -> PathSegments<'a> {
        PathSegments {
            segments: self.segments.get(len..).unwrap_or(&[]),
        }
    }

    pub fn take(&self, len: usize) -> PathSegments<'a> {
        PathSegments {
            segments: self.segments.get(..len).unwrap_or(self.segments),
        }
    }

    pub fn strip_last(&self) -> PathSegments<'a> {
        PathSegments {
            segments: self.segments.split_last().map_or(&[], |it| it.1),
        }
    }

    pub fn strip_last_two(&self) -> PathSegments<'a> {
        PathSegments {
            segments: self
                .segments
                .get(..self.segments.len().saturating_sub(2))
                .unwrap_or(&[]),
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = PathSegment<'a>> {
        self.segments.iter().map(|name| PathSegment { name })
    }
}

impl From<Name> for PathSyntax {
    fn from(name: Name) -> Self {
        PathSyntax::Plain(Interned::new(PlainPath::from_segments(iter::once(name))))
    }
}

impl_internable!(PlainPath);
