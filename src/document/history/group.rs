//! The flat group of [`History`] (ADR 0007 §D12, LCV-142): arm, apply into,
//! seal. Split out of `history.rs` for the LOC cap (LCV-198).
//!
//! MUST NOT import `egui`, `eframe`, or `rfd`.

use super::{Group, History};
use crate::document::{Command, CompositeCommand, Document};

impl History {
    /// Arm an empty flat group labelled `label` (ADR 0007 §D12). A group
    /// already open is sealed first, so groups never nest.
    pub fn begin_group(&mut self, label: &str) {
        self.end_group();
        self.group = Some(Group {
            label: label.to_owned(),
            commands: Vec::new(),
        });
    }

    /// Run `cmd` against `doc` **now** and remember it in the open group.
    ///
    /// Observably the same as [`History::commit`] for the document and the
    /// revision — `do_` runs at once, redo is cleared, `revision` bumps by
    /// one — only *where the command is remembered* differs. With no group
    /// armed it is exactly `commit`.
    pub fn commit_grouped(&mut self, mut cmd: Box<dyn Command>, doc: &mut Document) {
        let Some(group) = self.group.as_mut() else {
            self.commit(cmd, doc);
            return;
        };
        cmd.do_(doc);
        group.commands.push(cmd);
        self.redo_stack.clear();
        self.revision += 1;
    }

    /// Seal the open group onto the undo stack and disarm it.
    ///
    /// Zero commands push nothing, one is pushed bare, two or more become one
    /// [`CompositeCommand`] under the group's label; the depth cap runs once.
    /// Nothing is re-run and the revision does not move. Returns `None` when
    /// no group was armed (idempotent: a second call is a no-op) and
    /// `Some(n)` with the number of commands it sealed otherwise.
    pub fn end_group(&mut self) -> Option<usize> {
        let Group {
            label,
            mut commands,
        } = self.group.take()?;
        let sealed = commands.len();
        match sealed {
            0 => {}
            1 => self.undo_stack.extend(commands.pop()),
            _ => self
                .undo_stack
                .push_back(Box::new(CompositeCommand::new(commands, label))),
        }
        self.enforce_depth_cap();
        Some(sealed)
    }

    /// Is a group armed? The fence's second witness (ADR 0007 §D14): every
    /// seal and every document replacement leaves this `false`.
    pub fn group_open(&self) -> bool {
        self.group.is_some()
    }

    /// Does the open group hold any command? It counts as one undo entry.
    pub(super) fn open_entries(&self) -> usize {
        self.group
            .as_ref()
            .map_or(0, |g| usize::from(!g.commands.is_empty()))
    }
}
