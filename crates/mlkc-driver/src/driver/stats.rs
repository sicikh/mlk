//! The counters of the driver: what it reused, what it read again, what it dropped, and how
//! long the passes it ran took.
//!
//! The driver reads no clock of its own --- a browser has none inside wasm, and a host that
//! measures is one that knows what it measures --- so a host that wants milliseconds gives the
//! driver a [`Clock`], and what a pass takes is read off it. A driver with no clock counts the
//! same work and reports no time.
//!
//! Every consultation of a slot is a hit, a miss, or a stale read; the pass a stale read runs
//! may still come out equal to the value the driver held, which is the value it keeps, and
//! which is what a keep counts ([ADR-0008]); and a value that goes with the input it was built
//! from is a drop.
//!
//! [`Driver::take_stats`] hands the counters over and starts counting again, so what a host
//! reads is what happened since it last asked --- a lookup, a table, a log line, or a budget is
//! the host's to keep. What it reads them by is the pass and the unit: the parse of a file, the
//! check of a body, the index of a project.
//!
//! [`Driver::take_stats`]: super::Driver::take_stats
//!
//! [ADR-0008]: ../../../docs/adr/0008-compiler-driver.md

use std::{
    fmt,
    time::{Duration, Instant},
};

use mlkc_hir_def::{BodyEntityLoc, ModuleId, ProjectId};
use mlkc_vfs::FileId;

/// A source of time, told to a driver by its host.
///
/// The driver reads no clock of its own, so a host that wants passes timed hands it one: what
/// a clock answers is monotonic milliseconds, the same epoch for every call, and what the
/// driver makes of it is the difference between two readings around a pass.
pub type Clock = fn() -> f64;

/// The clock of a host that has one: the time of the machine, in milliseconds since the first
/// reading of it by this process.
///
/// A browser has no time of its own inside wasm, so the wasm shim gives its driver the clock of
/// the page instead; a host that has no clock at all sets none, and reads the work of the
/// passes rather than the time they took.
pub fn system_clock() -> Clock {
    /// The first reading, which every reading after it is counted from.
    static START: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();

    fn now() -> f64 {
        START.get_or_init(Instant::now).elapsed().as_secs_f64() * 1000.0
    }

    now
}

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
    /// The module of one module: its functions, and the functions they call ([ADR-0021]).
    ///
    /// [adr-0021]: ../../docs/adr/0021-translation-units.md
    MirModule,
    /// The compiled modules of one project, ordered for a host to link.
    Link,
}

impl Pass {
    /// Every pass, in the order a module is read in.
    pub const ALL: [Pass; 16] = [
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
        Pass::MirModule,
        Pass::Link,
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
            Pass::MirModule => "mir_module",
            Pass::Link => "link",
        }
    }
}

impl fmt::Display for Pass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Which value a pass was asked for: the unit the slot it read belongs to.
///
/// A file owns its parse, its HIR, its line index and the rendered diagnostics of its parse; a
/// module owns its interface, its resolution, its type surface and the diagnostics of them; a
/// project owns its module index and its def map; a body owns its check, its MIR and its SSA
/// form. What a host reads of the counters is which of those had to be read again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unit {
    /// A file of the host's, by the id it was pushed under.
    File(FileId),

    /// A module: a file the compiler reads as one.
    Module(ModuleId),

    /// A project: the modules of a host, and what they are read under.
    Project(ProjectId),

    /// A body: the body of one entity of a module.
    Body(BodyEntityLoc),
}

/// What one pass did for one unit since a host last took the counters.
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
    /// The time the pass spent running for this unit, as the host's clock read it: zero when
    /// the driver was given no clock, and zero for a pass that never ran.
    pub took: Duration,
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
/// twice is two lookups, one that may be a miss and one that may be a hit. The entries are per
/// pass and per unit --- the check of one body is one entry, the check of another is another
/// --- and they read in the order a module is read in.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Stats {
    /// The counters of every pass and unit that was asked for, by pass and by the order they
    /// were first asked for.
    units: Vec<(Pass, Unit, Tally)>,
}

impl Stats {
    /// The counters of one pass for one unit, or nothing when it was never asked for.
    pub fn of(&self, pass: Pass, unit: &Unit) -> Option<Tally> {
        self.units
            .iter()
            .find(|(it, held, _)| *it == pass && held == unit)
            .map(|(_, _, tally)| *tally)
    }

    /// Every pass and unit that was asked for, in the order a module is read in.
    pub fn iter(&self) -> impl Iterator<Item = (Pass, &Unit, Tally)> + '_ {
        self.units
            .iter()
            .map(|(pass, unit, tally)| (*pass, unit, *tally))
    }

    /// What one pass did over every unit it was asked for.
    pub fn total(&self, pass: Pass) -> Tally {
        self.units.iter().filter(|(it, ..)| *it == pass).fold(
            Tally::default(),
            |all, (_, _, tally)| {
                Tally {
                    hits: all.hits + tally.hits,
                    misses: all.misses + tally.misses,
                    stales: all.stales + tally.stales,
                    kept: all.kept + tally.kept,
                    dropped: all.dropped + tally.dropped,
                    took: all.took + tally.took,
                }
            },
        )
    }

    /// Whether the driver did nothing at all.
    pub fn is_empty(&self) -> bool {
        self.units.iter().all(|(_, _, tally)| tally.is_empty())
    }

    /// The counters a host is taking, with the clock left where it was.
    pub(super) fn take(&mut self) -> Self {
        Self {
            units: std::mem::take(&mut self.units),
        }
    }

    /// Counts one consultation of a slot: a hit, or a pass that has to run.
    ///
    /// `held` is whether the driver held a value for the slot when it was consulted, which is
    /// what tells a first read from one whose input had changed.
    pub(super) fn consulted(&mut self, pass: Pass, unit: &Unit, held: bool, hit: bool) {
        let tally = self.tally(pass, unit);

        if hit {
            tally.hits += 1;
        } else if held {
            tally.stales += 1;
        } else {
            tally.misses += 1;
        }
    }

    /// Counts a pass that ran, and the time between two readings of the host's clock.
    ///
    /// `started` is the reading taken before the pass ran, and `None` is what a host that gave
    /// the driver no clock leaves behind: the work is counted either way, and nothing is timed.
    pub(super) fn ran(
        &mut self,
        pass: Pass,
        unit: &Unit,
        clock: Option<Clock>,
        started: Option<f64>,
    ) {
        let took = match (clock, started) {
            (Some(clock), Some(started)) => {
                Duration::from_secs_f64((clock() - started).max(0.0) / 1000.0)
            },
            _ => Duration::ZERO,
        };

        self.tally(pass, unit).took += took;
    }

    /// Counts a pass that ran and read the same as the value the driver held.
    pub(super) fn kept(&mut self, pass: Pass, unit: &Unit) {
        self.tally(pass, unit).kept += 1;
    }

    /// Counts values that went, with the input they were built from.
    pub(super) fn dropped(&mut self, pass: Pass, unit: &Unit, count: usize) {
        self.tally(pass, unit).dropped += count as u32;
    }

    /// The counters of one pass and unit, made to exist when they are not there yet.
    ///
    /// An entry is put where it reads in the order a module is read in: after the entries of
    /// the passes before it, and after the entries of its own pass.
    fn tally(&mut self, pass: Pass, unit: &Unit) -> &mut Tally {
        if let Some(at) = self
            .units
            .iter()
            .position(|(it, held, _)| *it == pass && held == unit)
        {
            return &mut self.units[at].2;
        }

        let at = self
            .units
            .iter()
            .rposition(|(it, ..)| *it <= pass)
            .map_or(0, |at| at + 1);

        self.units
            .insert(at, (pass, unit.clone(), Tally::default()));

        &mut self.units[at].2
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use mlkc_hir_def::ModuleId;
    use mlkc_vfs::FileId;

    use super::{Pass, Stats, Tally, Unit};

    /// The list is the enum, and the enum indexes the passes: a pass added to one is a pass
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
                | Pass::Ssa
                | Pass::MirModule
                | Pass::Link => (),
            }
        }

        assert_eq!(Pass::ALL.len(), 16);
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
    fn counters_are_per_pass_and_unit() {
        let file = Unit::File(FileId::from_raw(1));
        let module = Unit::Module(ModuleId(FileId::from_raw(1)));
        let mut stats = Stats::default();

        assert!(stats.is_empty());

        stats.consulted(Pass::Parse, &file, false, false);
        stats.consulted(Pass::Parse, &file, true, false);
        stats.consulted(Pass::Parse, &file, true, true);
        stats.kept(Pass::Parse, &file);
        stats.dropped(Pass::Parse, &file, 2);

        assert_eq!(
            stats.of(Pass::Parse, &file),
            Some(Tally {
                hits: 1,
                misses: 1,
                stales: 1,
                kept: 1,
                dropped: 2,
                took: Duration::ZERO,
            })
        );
        assert_eq!(stats.of(Pass::Parse, &module), None);
        assert_eq!(stats.of(Pass::Interface, &file), None);
        assert!(!stats.is_empty());
        assert_eq!(stats.iter().count(), 1);
    }

    /// The entries read in the order a module is read in, and in the order they were made
    /// within one pass.
    #[test]
    fn the_entries_read_in_the_order_of_the_passes() {
        let file = Unit::File(FileId::from_raw(1));
        let other = Unit::File(FileId::from_raw(2));
        let mut stats = Stats::default();

        stats.consulted(Pass::Check, &file, false, false);
        stats.consulted(Pass::Parse, &other, false, false);
        stats.consulted(Pass::Parse, &file, false, false);
        stats.consulted(Pass::Check, &file, true, true);

        let read: Vec<(Pass, &Unit)> = stats.iter().map(|(pass, unit, _)| (pass, unit)).collect();

        assert_eq!(read, vec![
            (Pass::Parse, &other),
            (Pass::Parse, &file),
            (Pass::Check, &file),
        ]);
    }

    /// What a host takes is the counters, and the clock stays with the driver.
    #[test]
    fn taking_the_counters_leaves_the_clock_behind() {
        let unit = Unit::File(FileId::from_raw(1));
        let mut stats = Stats::default();

        stats.consulted(Pass::Parse, &unit, false, false);

        let taken = stats.take();

        assert_eq!(taken.iter().count(), 1);
        assert_eq!(stats.iter().count(), 0);
    }
}
