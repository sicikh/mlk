//! The parse of a file: the tree it is, and the diagnostics it reported.

use std::sync::Arc;

use mlkc_diagnostics::Diagnostic;
use mlkc_parser_core::{AnyParse, diagnostic::ParseDiagnostic};
use mlkc_rowan::{AstNode, NodeCache};
use mlkc_syntax::{ModuleRoot, SyntaxNode};
use mlkc_vfs::FileId;

use super::{Driver, ParseSlot, Pass};

/// The value of the parse slot: what the parser returned, whole.
///
/// The tree is immutable and shared, so whoever holds it
/// holds the text it was parsed from, whatever the file holds now.
pub struct Parse {
    parse: AnyParse,
}

impl Parse {
    /// Runs the pass.
    ///
    /// This is the only place where the driver touches the parser:
    /// the value is stored as the parser produced it,
    /// which is what makes the slot's key the input of the pass.
    ///
    /// The tree is built through `cache`, which is the table of the file this parse is of:
    /// what the parse before it wrote is what this one shares.
    fn of(source: &str, cache: &mut NodeCache) -> Self {
        Self {
            parse: mlkc_parser::parse_with_cache(source, cache),
        }
    }

    /// The concrete syntax tree: lossless, and never absent, however broken the input.
    pub fn syntax(&self) -> SyntaxNode {
        self.parse.syntax()
    }

    /// The typed view over the tree,
    /// or `None` when the parse did not find a module root — a tree is not a module by itself.
    pub fn module_root(&self) -> Option<ModuleRoot> {
        ModuleRoot::cast(self.syntax())
    }

    /// The diagnostics the parse produced, in the shape the parser knows them.
    pub fn diagnostics(&self) -> &[ParseDiagnostic] {
        self.parse.diagnostics()
    }

    /// Whether the parse reported an error.
    pub fn has_errors(&self) -> bool {
        self.parse.has_errors()
    }
}

impl std::fmt::Debug for Parse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Parse")
            .field("diagnostics", &self.parse.diagnostics())
            .finish_non_exhaustive()
    }
}

impl Driver {
    // Pulls: they may compute, and they never answer from an invalid slot.

    /// The parse of `file`, computed when the slot is missing or stale.
    ///
    /// `None` means the driver has no text for the file:
    /// it was never pushed, it is gone, or it is not text at all.
    /// [`Driver::file_state`] tells which of those it is.
    pub fn parse(&mut self, file: FileId) -> Option<Arc<Parse>> {
        let version = self.file_version(file);
        let held = self.parses.get(&file);

        // There is nothing to back-date here: the parse is a function of the text,
        // and the tree of different text is a different tree.
        // The stages where a recomputation can end up equal to the retained value —
        // the item tree, the interface — are the ones that follow.
        if let Some(slot) = held
            && slot.version == version
        {
            self.stats.consulted(Pass::Parse, true, true);

            return Some(slot.value.clone());
        }

        self.stats.consulted(Pass::Parse, held.is_some(), false);

        let Some(text) = self.file_text(file) else {
            // There is no input left to describe, so the slot goes, and the green nodes of
            // this file go with it: nothing is going to be parsed the way it was.
            self.stats
                .dropped(Pass::Parse, self.parses.remove(&file).is_some() as usize);

            return None;
        };

        // The table this parse is built through is the one the parse before it left: the
        // tokens that did not move are what the two revisions of this file share.
        let mut cache = self
            .parses
            .remove(&file)
            .map_or_else(NodeCache::default, |slot| slot.cache);
        let value = Arc::new(Parse::of(&text, &mut cache));

        self.parses.insert(file, ParseSlot {
            version,
            value: value.clone(),
            cache,
        });

        Some(value)
    }

    /// The diagnostics of the parse of `file`, in the shape a host renders.
    ///
    /// The rendering is a function of the parse, so it is keyed by the text of the file like
    /// the parse itself: what a parser reported is rendered once per text it read.
    pub(super) fn parse_diagnostics(&mut self, file: FileId) -> Option<Arc<[Diagnostic]>> {
        let parse = self.parse(file)?;
        let version = self.file_version(file);

        Self::text_derived(
            &mut self.stats,
            Pass::ParseDiagnostics,
            &mut self.parse_diagnostics,
            file,
            version,
            || {
                let rendered = parse
                    .diagnostics()
                    .iter()
                    .map(|diagnostic| diagnostic.to_diagnostic(file))
                    .collect::<Vec<_>>();

                Some(Arc::from(rendered))
            },
        )
    }
}
