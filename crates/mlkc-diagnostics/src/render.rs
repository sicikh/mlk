// use std::io::{self, Write};
//
// use mlkc_span::TextSize;
// use mlkc_vfs::Vfs;
//
// use crate::diagnostic::{Diagnostic, Label};
//
// pub fn render(diag: &Diagnostic, vfs: &Vfs, out: &mut impl Write) -> io::Result<()> {
//     writeln!(
//         out,
//         "{}[{}]: {}",
//         diag.level.as_str(),
//         diag.code,
//         diag.message
//     )?;
//
//     let mut labels: Vec<&Label> = diag.labels.iter().collect();
//     labels.sort_by_key(|l| (l.span.file, l.span.range.start()));
//
//     for label in &labels {
//         render_label(label, vfs, out)?;
//     }
//
//     for note in &diag.notes {
//         writeln!(out, "  = note: {}", note)?;
//     }
//
//     writeln!(out)?;
//     Ok(())
// }
//
// fn render_label(label: &Label, vfs: &Vfs, out: &mut impl Write) -> io::Result<()> {
//     let file = vfs.file(label.span.file);
//     let (line, col) = vfs.lookup_line_col(label.span);
//     let (line_text, line_start) = vfs.line_text(label.span.file, line);
//
//     writeln!(out, "  --> {}:{}:{}", file.name.display(), line, col)?;
//     writeln!(out, "   |")?;
//
//     let line_no = line.to_string();
//     let gutter = line_no.len();
//     writeln!(out, " {} | {}", line_no, line_text)?;
//
//     let prefix = label.span.range.start() - line_start;
//     let width = (label.span.range.end() - label.span.range.start()).max(TextSize::from(1));
//     let marker = if label.primary { '^' } else { '-' };
//
//     let pad = " ".repeat(prefix.into());
//     let marks = marker.to_string().repeat(width.into());
//     let spacer = " ".repeat(gutter + 1);
//
//     if label.message.is_empty() {
//         writeln!(out, "{spacer}| {pad}{marks}")?;
//     } else {
//         writeln!(out, "{spacer}| {pad}{marks} {}", label.message)?;
//     }
//
//     Ok(())
// }
