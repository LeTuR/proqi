//! Terminal-independent searchable session browser state and geometry.

#[cfg(test)]
mod confirmation_tests;
mod geometry;
mod input;
mod management;

use ratatui_core::layout::Rect;

use crate::{
    domain::{SessionId, Timestamp},
    ports::{runtime::InstanceInfo, store::SessionHit},
};

/// Runtime availability shown beside one durable session.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BrowserAvailability {
    /// Another verified process currently owns the session.
    Active(InstanceInfo),
    /// Session can be leased and resumed normally.
    Resumable,
    /// Stale crash metadata was recovered during this browser scan.
    Recovered,
    /// Session is recoverably deleted and cannot be opened.
    Trashed,
}

impl BrowserAvailability {
    /// Stable user-facing state label.
    #[must_use]
    pub const fn label(&self) -> &'static str {
        match self {
            Self::Active(_) => "active",
            Self::Resumable => "resumable",
            Self::Recovered => "recovered",
            Self::Trashed => "trashed",
        }
    }
}

/// One search result paired with verified runtime availability.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionBrowserItem {
    /// Durable search projection.
    pub hit: SessionHit,
    /// Runtime and trash state observed before the browser opened.
    pub availability: BrowserAvailability,
}

/// Relative recency section rendered in the result list.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecencyGroup {
    /// Active within the latest 24 hours.
    Today,
    /// Active between 24 and 48 hours ago.
    Yesterday,
    /// Active between two and seven days ago.
    PreviousWeek,
    /// Older than seven days.
    Older,
}

impl RecencyGroup {
    /// Stable heading text.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Today => "Today",
            Self::Yesterday => "Yesterday",
            Self::PreviousWeek => "Previous 7 days",
            Self::Older => "Older",
        }
    }
}

/// Geometry for one visible result and its optional narrow detail.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BrowserEntryLayout {
    /// Index into the browser's complete item collection.
    pub item_index: usize,
    /// Optional recency heading immediately above the result.
    pub group: Option<(RecencyGroup, Rect)>,
    /// Clickable result row.
    pub row: Rect,
    /// Inline detail shown for the selected result in a narrow pane.
    pub inline_detail: Option<Rect>,
}

/// Complete browser geometry shared by rendering and mouse handling.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BrowserLayout {
    /// Complete frame.
    pub area: Rect,
    /// Search header.
    pub header: Rect,
    /// Scrollable result region.
    pub results: Rect,
    /// Wide-screen detail pane.
    pub detail: Option<Rect>,
    /// Visible result geometry.
    pub entries: Vec<BrowserEntryLayout>,
    /// Quiet non-interactive cue when earlier results exist.
    pub overflow_above: Option<Rect>,
    /// Quiet non-interactive cue when later results exist.
    pub overflow_below: Option<Rect>,
    /// Clickable cancellation footer.
    pub footer: Rect,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum BrowserHit {
    Item(usize),
    Rename,
    Trash,
    Confirm,
    Cancel,
    None,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct BrowserFooterControl {
    pub(super) hit: BrowserHit,
    pub(super) key: String,
    pub(super) label: &'static str,
    pub(super) area: Rect,
}

pub(super) fn browser_footer_controls(
    area: Rect,
    registry: &crate::ui::ShortcutRegistry,
    context: crate::ui::ShortcutContext,
) -> Vec<BrowserFooterControl> {
    if area.width == 0 || area.height == 0 {
        return Vec::new();
    }
    let mut x = area.x.saturating_add(1);
    crate::ui::shortcut_registry::presentation::browser_footer_projection(
        registry, area.width, context,
    )
    .into_iter()
    .filter_map(|projection| {
        let width = crate::ports::text_layout::terminal_cell_width(&projection.key)
            .saturating_add(1)
            .saturating_add(crate::ports::text_layout::terminal_cell_width(
                projection.label,
            ));
        let width = u16::try_from(width).unwrap_or(u16::MAX);
        if x.saturating_add(width) > area.right() {
            return None;
        }
        let control = BrowserFooterControl {
            hit: browser_hit(projection.actions),
            key: projection.key,
            label: projection.label,
            area: Rect::new(x, area.y, width, 1),
        };
        x = x.saturating_add(width).saturating_add(2);
        Some(control)
    })
    .collect()
}

fn browser_hit(actions: &[crate::ui::ShortcutActionId]) -> BrowserHit {
    if actions.contains(&crate::ui::ShortcutActionId::RenameSession) {
        BrowserHit::Rename
    } else if actions.contains(&crate::ui::ShortcutActionId::BrowserTrash) {
        BrowserHit::Trash
    } else if actions.contains(&crate::ui::ShortcutActionId::Close) {
        BrowserHit::Cancel
    } else if actions.contains(&crate::ui::ShortcutActionId::Confirm) {
        BrowserHit::Confirm
    } else {
        BrowserHit::None
    }
}

/// Result of handling one browser input.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BrowserAction {
    /// Continue browsing.
    Continue,
    /// Open this typed session after the browser restores the terminal.
    Open(SessionId),
    /// Persist a new optional name and reopen the refreshed browser.
    Rename {
        /// Session to rename.
        session_id: SessionId,
        /// Empty input clears the optional name.
        name: Option<String>,
    },
    /// Move this session into recoverable trash.
    Trash(SessionId),
    /// Leave without opening a session.
    Cancel,
}

/// Searchable, responsive session picker.
pub struct SessionBrowser {
    items: Vec<SessionBrowserItem>,
    filtered: Vec<usize>,
    query: String,
    selected: usize,
    first_visible: usize,
    now: Timestamp,
    layout: Option<BrowserLayout>,
    pub(super) footer_controls: Vec<BrowserFooterControl>,
    rename: Option<management::RenameState>,
    pub(super) shortcut_registry: crate::ui::ShortcutRegistry,
    /// Visible explanation for blocked or ambiguous actions.
    pub status: Option<String>,
}

impl SessionBrowser {
    /// Construct a browser in current ranking order.
    #[must_use]
    pub fn new(items: Vec<SessionBrowserItem>, now: Timestamp) -> Self {
        let filtered = (0..items.len()).collect();
        Self {
            items,
            filtered,
            query: String::new(),
            selected: 0,
            first_visible: 0,
            now,
            layout: None,
            footer_controls: Vec::new(),
            rename: None,
            shortcut_registry: crate::ui::ShortcutRegistry::default(),
            status: None,
        }
    }

    pub(crate) fn with_shortcut_registry(
        items: Vec<SessionBrowserItem>,
        now: Timestamp,
        shortcut_registry: crate::ui::ShortcutRegistry,
    ) -> Self {
        let mut browser = Self::new(items, now);
        browser.shortcut_registry = shortcut_registry;
        browser
    }

    /// Current case-insensitive search text.
    #[must_use]
    pub fn query(&self) -> &str {
        &self.query
    }

    /// Active rename input, when the browser is editing a session name.
    #[must_use]
    pub fn rename_value(&self) -> Option<&str> {
        self.rename.as_ref().map(|rename| rename.value.as_str())
    }

    /// Search results in their storage-defined ranking order.
    pub fn visible_items(&self) -> impl Iterator<Item = (usize, &SessionBrowserItem)> {
        self.filtered
            .iter()
            .copied()
            .map(|index| (index, &self.items[index]))
    }

    /// Currently selected item, when the search has results.
    #[must_use]
    pub fn selected_item(&self) -> Option<(usize, &SessionBrowserItem)> {
        self.filtered
            .get(self.selected)
            .copied()
            .map(|index| (index, &self.items[index]))
    }

    /// Relative recency group for one item.
    #[must_use]
    pub fn group_for(&self, item: &SessionBrowserItem) -> RecencyGroup {
        let age = self
            .now
            .as_millis()
            .saturating_sub(item.hit.last_active_at.as_millis());
        if age <= 86_400_000 {
            RecencyGroup::Today
        } else if age <= 172_800_000 {
            RecencyGroup::Yesterday
        } else if age <= 604_800_000 {
            RecencyGroup::PreviousWeek
        } else {
            RecencyGroup::Older
        }
    }

    /// Compact last-activity label for result rows.
    #[must_use]
    pub fn activity_label(&self, item: &SessionBrowserItem) -> String {
        let age = self
            .now
            .as_millis()
            .saturating_sub(item.hit.last_active_at.as_millis())
            .max(0);
        if age < 60_000 {
            "now".to_owned()
        } else if age < 3_600_000 {
            format!("{}m ago", age / 60_000)
        } else if age < 86_400_000 {
            format!("{}h ago", age / 3_600_000)
        } else {
            format!("{}d ago", age / 86_400_000)
        }
    }

    /// Recompute authoritative frame and hit-test geometry.
    pub fn prepare_frame(&mut self, area: Rect) -> BrowserLayout {
        if self.selected < self.first_visible {
            self.first_visible = self.selected;
        }
        let mut layout = self.compute_layout(area);
        let selected_index = self.filtered.get(self.selected).copied();
        if selected_index
            .is_some_and(|index| !layout.entries.iter().any(|entry| entry.item_index == index))
        {
            self.first_visible = self.selected;
            layout = self.compute_layout(area);
        }
        self.layout = Some(layout.clone());
        self.footer_controls = if self.status.is_none() {
            browser_footer_controls(
                layout.footer,
                &self.shortcut_registry,
                self.shortcut_context(),
            )
        } else {
            Vec::new()
        };
        layout
    }
}
