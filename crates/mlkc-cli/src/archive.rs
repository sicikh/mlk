//! The ZIP archive a build is handed over in, read as the files it holds ([ZIP]).
//!
//! An archive is a way to carry a directory: a download cannot make a folder, so the editor
//! writes the modules and the manifest of a build under their paths inside one file. What is
//! read here is what the editor writes --- the method the specification calls "store", a
//! local header per file, and the directory at the end --- because the archive is the editor's
//! download rather than a format of the compiler: a build that came from anywhere else is a
//! directory of files. An entry that is compressed is refused with what to do about it.
//!
//! Nothing is unpacked to disk: a file of the archive is looked up by the name its path stands
//! for and handed over as the bytes it is, so a path a person should not write is a path
//! nobody does.
//!
//! [zip]: https://en.wikipedia.org/wiki/ZIP_(file_format)

use std::collections::BTreeMap;

use anyhow::{Context as _, bail};

/// The signature of a local header, which every file of an archive begins with.
const LOCAL: u32 = 0x0403_4B50;

/// The signature of a line of the directory at the end of an archive.
const CENTRAL: u32 = 0x0201_4B50;

/// The signature of the end of a directory: the last thing an archive holds.
const END: [u8; 4] = [b'P', b'K', 0x05, 0x06];

/// The files of a ZIP archive, held in memory.
pub(crate) struct Archive {
    /// The archive itself: what a file of it is a piece of.
    bytes: Vec<u8>,

    /// Where every file of the archive stands, by the path it is written under.
    files: BTreeMap<String, Entry>,
}

/// One file of an archive: where its header stands, and how long its bytes are.
struct Entry {
    /// The offset of the local header of the file.
    at: usize,

    /// How many bytes the contents of the file are.
    size: usize,
}

impl Archive {
    /// Reads the archive `bytes` are.
    pub(crate) fn read(bytes: Vec<u8>) -> anyhow::Result<Archive> {
        let end = end_of(&bytes)?;
        let count = u16_at(&bytes, end + 10)?;
        let mut files = BTreeMap::new();
        let mut at = u32_at(&bytes, end + 16)? as usize;

        for index in 0..count {
            if u32_at(&bytes, at)? != CENTRAL {
                bail!("entry {index} of the archive is not a header");
            }

            let method = u16_at(&bytes, at + 10)?;
            let size = u32_at(&bytes, at + 24)? as usize;
            let name = u16_at(&bytes, at + 28)? as usize;
            let extra = u16_at(&bytes, at + 30)? as usize;
            let comment = u16_at(&bytes, at + 32)? as usize;
            let local = u32_at(&bytes, at + 42)? as usize;
            let raw = bytes
                .get(at + 46..at + 46 + name)
                .context("the archive ends in the middle of the name of a file")?;
            let path = String::from_utf8(raw.to_vec())
                .context("the name of a file of the archive is not text")?;

            if method != 0 {
                bail!(
                    "the file `{path}` of the archive is compressed, and a build is stored as it \
                     is: unpack the archive, or write it again without compression",
                );
            }

            files.insert(path, Entry { at: local, size });
            at += 46 + name + extra + comment;
        }

        Ok(Archive { bytes, files })
    }

    /// The bytes of the file `name` of the archive.
    pub(crate) fn file(&self, name: &str) -> anyhow::Result<&[u8]> {
        let entry = self
            .files
            .get(name)
            .with_context(|| format!("the archive holds no `{name}`"))?;

        if u32_at(&self.bytes, entry.at)? != LOCAL {
            bail!("the file `{name}` of the archive is not a header");
        }

        let held = u16_at(&self.bytes, entry.at + 26)? as usize;
        let extra = u16_at(&self.bytes, entry.at + 28)? as usize;
        let at = entry.at + 30 + held + extra;
        let data = self
            .bytes
            .get(at..at + entry.size)
            .with_context(|| format!("the archive ends in the middle of `{name}`"))?;

        Ok(data)
    }
}

/// Where the directory of the archive ends, read from the end of it: a comment may stand after
/// it, and a signature inside a file of the archive is not one.
fn end_of(bytes: &[u8]) -> anyhow::Result<usize> {
    let last = bytes
        .len()
        .checked_sub(22)
        .context("the archive is shorter than its end")?;
    let from = bytes.len().saturating_sub(22 + 0xFFFF);

    (from..=last)
        .rev()
        .find(|at| bytes[*at..*at + 4] == END)
        .context("the archive has no directory at its end")
}

/// The number at `at` of an archive, which is two bytes wide and written little end first.
fn u16_at(bytes: &[u8], at: usize) -> anyhow::Result<u16> {
    let raw = bytes
        .get(at..at + 2)
        .context("the archive ends in the middle of it")?;

    Ok(u16::from_le_bytes([raw[0], raw[1]]))
}

/// The number at `at` of an archive, which is four bytes wide and written little end first.
fn u32_at(bytes: &[u8], at: usize) -> anyhow::Result<u32> {
    let raw = bytes
        .get(at..at + 4)
        .context("the archive ends in the middle of it")?;

    Ok(u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]))
}
