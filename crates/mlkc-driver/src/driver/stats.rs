//! The counters of the driver: what it reused, what it read again, and what it dropped.
//!
//! The driver does not read a clock --- a value is a function of its inputs, and how long
//! something took is not one of them --- so what it counts is the work itself. Every
//! consultation of a slot is a hit, a miss, or a stale read; the pass a stale read runs may
//! still come out equal to the value the driver held, which is the value it keeps, and which
//! is what a keep counts ([ADR-0008]); and a value that goes with the input it was built from
//! is a drop.
//!
//! A host that wants milliseconds wraps the pull it made: the time of a pull is the host's to
//! measure, and the counters say what the time was spent on. [`Driver::take_stats`] hands the
//! counters over and starts counting again, so what a host reads is what happened since it last
//! asked --- a lookup, a table, a log line, or a budget is the host's to keep.
//!
//! [`Driver::take_stats`]: super::Driver::take_stats
//!
//! [ADR-0008]: ../../../docs/adr/0008-compiler-driver.md

use std::fmt;

/// One pass of the pipeline, as the driver counts what it does with it.
///
/// Every pass owns one slot per unit it is a value of: a file owns its parse and its line
/// index, a module owns its HIR, its interface, its resolution and its rendered diagnostics,
/// a project owns its index and its def map, and a body owns its check, its MIR and its SSA
/// form. What a host reads of the counters is which of those had to be read again.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Pass {
    /// The parse of a file: the tree, and what the parser reported.
    Parse,
    /// The HIR of a file: the surface of its module, and the bodies of it.
    Lower,
    /// Where the lines of a file start and end.
    LineIndex,
    /// What a module shows to the modules that name it.
    Interface,
    /// Which path of a project names which module.
    ModuleIndex,
    /// What the names of a module denote.
    Resolution,
    /// The diagnostics of the resolution of a module, rendered.
    ResolutionDiagnostics,
    /// The scopes of the modules of a project.
    DefMap,
    /// The types the declarations of a module write, resolved.
    Signatures,
    /// The checks of the bodies of a module, rendered as diagnostics.
    TypeDiagnostics,
    /// What the parser reported about a file, rendered.
    ParseDiagnostics,
    /// The types of the nodes of one body, checked.
    Check,
    /// The MIR of one body, in the CFG form.
    Mir,
    /// The MIR of one body, in the SSA form.
    Ssa,
}

impl Pass {
    /// Every pass, in the order a module is read in.
    pub const ALL: [Pass; 14] = [
        Pass::Parse,
        Pass::Lower,
        Pass::LineIndex,
        Pass::Interface,
        Pass::ModuleIndex,
        Pass::Resolution,
        Pass::ResolutionDiagnostics,
        Pass::DefMap,
        Pass::Signatures,
        Pass::TypeDiagnostics,
        Pass::ParseDiagnostics,
        Pass::Check,
        Pass::Mir,
        Pass::Ssa,
    ];

    /// The name a host knows the pass by: the one a table is read by, and one word.
    pub const fn name(self) -> &'static str {
        match self {
            Pass::Parse => "parse",
            Pass::Lower => "lower",
            Pass::LineIndex => "line_index",
            Pass::Interface => "interface",
            Pass::ModuleIndex => "module_index",
            Pass::Resolution => "resolution",
            Pass::ResolutionDiagnostics => "resolution_diagnostics",
            Pass::DefMap => "def_map",
            Pass::Signatures => "signatures",
            Pass::TypeDiagnostics => "type_diagnostics",
            Pass::ParseDiagnostics => "parse_diagnostics",
            Pass::Check => "check",
            Pass::Mir => "mir",
            Pass::Ssa => "ssa",
        }
    }
}

impl fmt::Display for Pass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// What one pass did since a host last took the counters.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Tally {
    /// The slot was keyed by what it was built from: the value was handed over, and the pass
    /// did not run.
    pub hits: u32,
    /// The pass ran, and the driver held no value for it: a first read, or one whose value had
    /// been dropped.
    pub misses: u32,
    /// The pass ran although a value was held, because what the value was built from had
    /// changed.
    pub stales: u32,
    /// The pass ran, and what it computed read the same as the value the driver held, which is
    /// the value it kept.
    pub kept: u32,
    /// The value went, with the input it was built from: the next read of it is a miss.
    pub dropped: u32,
}

impl Tally {
    /// Whether the pass did nothing at all.
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

/// What the driver did since a host last took the counters.
///
/// A counter is of consultations rather than of values: one pull that asks for the same slot
/// twice is two lookups, one that may be a miss and one that may be a hit. What the counters
/// say is what the driver did, not how many slots it holds.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Stats {
    /// The counters of every pass, by [`Pass::ALL`].
    tally: [Tally; Pass::ALL.len()],
}

impl Stats {
    /// The counters of one pass.
    pub fn of(&self, pass: Pass) -> Tally {
        self.tally[pass as usize]
    }

    /// Every pass and what it did, in the order a module is read in.
    pub fn iter(&self) -> impl Iterator<Item = (Pass, Tally)> + '_ {
        Pass::ALL.into_iter().map(|pass| (pass, self.of(pass)))
    }

    /// Whether the driver did nothing at all.
    pub fn is_empty(&self) -> bool {
        self.tally.iter().all(Tally::is_empty)
    }

    /// Counts one consultation of a slot: a hit, or a pass that had to run.
    ///
    /// `held` is whether the driver held a value for the slot when it was consulted, which is
    /// what tells a first read from one whose input had changed.
    pub(super) fn consulted(&mut self, pass: Pass, held: bool, hit: bool) {
        let tally = &mut self.tally[pass as usize];

        if hit {
            tally.hits += 1;
        } else if held {
            tally.stales += 1;
        } else {
            tally.misses += 1;
        }
    }

    /// Counts a pass that ran and read the same as the value the driver held.
    pub(super) fn kept(&mut self, pass: Pass) {
        self.tally[pass as usize].kept += 1;
    }

    /// Counts values that went, with the input they were built from.
    pub(super) fn dropped(&mut self, pass: Pass, count: usize) {
        self.tally[pass as usize].dropped += count as u32;
    }
}

#[cfg(test)]
mod tests {
    use super::{Pass, Stats, Tally};

    /// The list is the enum, and the enum indexes the counters: a pass added to one is a pass
    /// added to the other, and the compiler is what says so.
    #[test]
    fn every_pass_is_counted() {
        for (index, pass) in Pass::ALL.into_iter().enumerate() {
            assert_eq!(pass as usize, index);

            // An exhaustive match: a variant that is not named here does not compile.
            match pass {
                Pass::Parse
                | Pass::Lower
                | Pass::LineIndex
                | Pass::Interface
                | Pass::ModuleIndex
                | Pass::Resolution
                | Pass::ResolutionDiagnostics
                | Pass::DefMap
                | Pass::Signatures
                | Pass::TypeDiagnostics
                | Pass::ParseDiagnostics
                | Pass::Check
                | Pass::Mir
                | Pass::Ssa => (),
            }
        }

        assert_eq!(Pass::ALL.len(), 14);
    }

    /// A name is what a table and a log read a pass by, and two passes share none.
    #[test]
    fn every_pass_has_its_own_name() {
        let mut names: Vec<&str> = Pass::ALL.iter().map(|pass| pass.name()).collect();

        names.sort_unstable();
        names.dedup();

        assert_eq!(names.len(), Pass::ALL.len());
    }

    /// One tally does not spill into another, and a clean read is a read that counted nothing.
    #[test]
    fn counters_are_per_pass() {
        let mut stats = Stats::default();

        assert!(stats.is_empty());

        stats.consulted(Pass::Parse, false, false);
        stats.consulted(Pass::Parse, true, false);
        stats.consulted(Pass::Parse, true, true);
        stats.kept(Pass::Parse);
        stats.dropped(Pass::Parse, 2);

        assert_eq!(stats.of(Pass::Parse), Tally {
            hits: 1,
            misses: 1,
            stales: 1,
            kept: 1,
            dropped: 2,
        });
        assert_eq!(stats.of(Pass::Check), Tally::default());
        assert!(!stats.is_empty());
        assert_eq!(
            stats.iter().filter(|(_, tally)| !tally.is_empty()).count(),
            1,
        );
    }
}
