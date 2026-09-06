//! Configured command bindings use the Commands availability and execution owner.
use super::{BoardApp, QueryEditor};
use crate::{
    application::Effect,
    ports::environment::{Clock, IdGenerator},
};

impl BoardApp {
    pub(in crate::ui::app) fn execute_bound_command(
        &mut self,
        action: crate::ui::ShortcutActionId,
        ids: &mut impl IdGenerator,
        clock: &impl Clock,
    ) -> Vec<Effect> {
        if self.editor_snapshot().is_some() {
            self.capture_palette_selection_handoff();
        }
        let mut effects = match self.flush_edit_boundary(ids, clock) {
            crate::ui::app::pending_types::EditFlush::Complete(effects) => effects,
            crate::ui::app::pending_types::EditFlush::Blocked(effects) => return effects,
        };
        effects.extend(self.execute_flushed_bound_command(action, ids, clock));
        effects
    }

    fn execute_flushed_bound_command(
        &mut self,
        action: crate::ui::ShortcutActionId,
        ids: &mut impl IdGenerator,
        clock: &impl Clock,
    ) -> Vec<Effect> {
        use crate::ui::ShortcutActionId as Shortcut;
        match action {
            Shortcut::OpenCommands => {
                if self.editor_snapshot().is_some() {
                    self.capture_palette_selection_handoff();
                }
                self.open_palette();
                return Vec::new();
            }
            Shortcut::OpenSearch => {
                self.open_search();
                return Vec::new();
            }
            _ => {}
        }
        if self
            .settings
            .shortcuts
            .descriptor(action)
            .and_then(|descriptor| descriptor.commands)
            .is_none()
        {
            return Vec::new();
        }
        // Reuse Commands availability, handoff capture and execution without
        // rendering or dispatching a synthetic key sequence.
        if self.palette.is_none() {
            if self.editor_snapshot().is_some() {
                self.capture_palette_selection_handoff();
            }
            self.open_palette();
        }
        if let Some(palette) = &mut self.palette {
            palette.query = QueryEditor::default();
        }
        let index = self.palette.as_ref().and_then(|palette| {
            palette
                .matches()
                .iter()
                .position(|(candidate, _, _)| *candidate == action)
        });
        if let Some(index) = index {
            self.execute_palette_index(index, ids, clock)
        } else {
            self.palette = None;
            self.set_warning("command is unavailable in the current state");
            Vec::new()
        }
    }
}
