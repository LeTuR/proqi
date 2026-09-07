//! Browser confirmation shares rendered keymap geometry and semantic outcomes.

use super::{BrowserAction, BrowserAvailability, SessionBrowser, SessionBrowserItem};
use crate::{
    domain::Timestamp,
    ports::store::SessionHit,
    ui::{
        KeyStroke, KeymapDocument, LogicalKey, PointerButton, PointerInput, PointerKind,
        ShortcutPlatform, Theme, ThemePreference, UiInput, render_browser,
    },
};
use ratatui_core::{backend::TestBackend, terminal::Terminal};

fn browser(name: &str, remapped: bool) -> SessionBrowser {
    let entry = SessionBrowserItem {
        hit: SessionHit {
            id: "ses_068q40p201o010000000000000"
                .parse()
                .expect("session ID"),
            name: Some(name.to_owned()),
            origin_cwd: "synthetic".into(),
            last_opened_cwd: "synthetic".into(),
            last_opened_at: Timestamp::from_millis(10),
            last_active_at: Timestamp::from_millis(10),
            thought_count: 0,
            excerpt: String::new(),
            previews: Vec::new(),
            search_content: String::new(),
            integration_context: None,
            trashed: false,
        },
        availability: BrowserAvailability::Resumable,
    };
    let mut browser = SessionBrowser::new(vec![entry], Timestamp::from_millis(20));
    if remapped {
        browser.shortcut_registry = toml::from_str::<KeymapDocument>(
            "schema_version=1\n[bindings.browser_rename]\n\"context.confirm\"=[{key=\"F35\",modifiers=[\"Control\",\"Alt\",\"Shift\"]}]",
        )
        .expect("keymap document")
        .resolve(ShortcutPlatform::Portable)
        .expect("resolved keymap");
    }
    browser
}

fn key(key: LogicalKey) -> UiInput {
    UiInput::KeyStroke(KeyStroke::press(key))
}

fn click(column: u16, row: u16) -> UiInput {
    UiInput::Pointer(PointerInput {
        column,
        row,
        kind: PointerKind::Down(PointerButton::Left),
        extend_selection: false,
    })
}

fn draw(browser: &mut SessionBrowser, width: u16, height: u16) -> Terminal<TestBackend> {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("terminal");
    terminal
        .draw(|frame| {
            let layout = browser.prepare_frame(frame.area());
            render_browser(
                frame,
                browser,
                &layout,
                &Theme::resolve(ThemePreference::Auto, true),
            );
        })
        .expect("draw");
    terminal
}

fn footer(terminal: &Terminal<TestBackend>) -> String {
    let buffer = terminal.backend().buffer();
    (0..buffer.area.width)
        .map(|x| buffer[(x, buffer.area.height - 1)].symbol())
        .collect()
}

fn label_column(line: &str, label: &str) -> u16 {
    let byte = line.find(label).expect("visible label");
    u16::try_from(crate::ports::text_layout::terminal_cell_width(
        &line[..byte],
    ))
    .expect("terminal column")
}

#[test]
fn rename_save_mouse_matches_keyboard_trim_and_clear() {
    for name in ["  Grüße 界 e\u{301}  ", "   "] {
        let mut keyboard = browser(name, false);
        keyboard.handle(key(LogicalKey::Function(2)));
        let expected = keyboard.handle(key(LogicalKey::Enter));
        assert!(
            matches!(&expected, BrowserAction::Rename { name: actual, .. }
            if actual.as_deref() == (!name.trim().is_empty()).then_some(name.trim()))
        );
        for (width, height) in [(80, 24), (24, 7), (12, 3)] {
            let mut mouse = browser(name, false);
            mouse.handle(key(LogicalKey::Function(2)));
            let terminal = draw(&mut mouse, width, height);
            let column = label_column(&footer(&terminal), "Save");
            assert_eq!(mouse.handle(click(column, height - 1)), expected);
            assert!(mouse.rename_value().is_none());
        }
    }
}

#[test]
fn remapped_save_uses_current_visible_geometry_and_cancel_survives() {
    let mut browser = browser("new name", true);
    browser.handle(key(LogicalKey::Function(2)));
    let terminal = draw(&mut browser, 80, 7);
    let label = footer(&terminal);
    assert!(label.contains("Ctrl+Alt+Shift+F35 Save"), "{label}");
    let column = label_column(&label, "Save");
    assert!(matches!(
        browser.handle(click(column, 6)),
        BrowserAction::Rename { .. }
    ));

    browser.handle(key(LogicalKey::Function(2)));
    let narrow = draw(&mut browser, 18, 7);
    let label = footer(&narrow);
    assert!(!label.contains("Save"));
    let cancel = label_column(&label, "Cancel");
    assert_eq!(browser.handle(click(17, 6)), BrowserAction::Continue);
    assert!(browser.rename_value().is_some());
    assert_eq!(browser.handle(click(cancel, 6)), BrowserAction::Continue);
    assert!(browser.rename_value().is_none());
}

#[test]
fn rename_without_visible_footer_cannot_save() {
    for (width, height) in [(0, 7), (80, 0), (80, 2), (8, 7)] {
        let mut browser = browser("unchanged", false);
        browser.handle(key(LogicalKey::Function(2)));
        draw(&mut browser, width, height);
        assert_eq!(
            browser.handle(click(7, height.saturating_sub(1))),
            BrowserAction::Continue
        );
        assert_eq!(browser.rename_value(), Some("unchanged"));
    }
}

#[test]
fn browser_open_mouse_matches_keyboard_and_availability() {
    for available in [true, false] {
        let mut browser = browser("session", false);
        if !available {
            browser.items[0].availability = BrowserAvailability::Trashed;
        }
        let expected = browser.handle(key(LogicalKey::Enter));
        browser.status = None;
        let terminal = draw(&mut browser, 80, 7);
        let label = footer(&terminal);
        let column = label_column(&label, "Open");
        assert_eq!(browser.handle(click(column, 6)), expected);
    }
}

#[test]
fn rename_transition_and_status_frame_have_no_stale_confirm_target() {
    let mut browser = browser("unchanged", false);
    draw(&mut browser, 80, 7);
    browser.handle(key(LogicalKey::Function(2)));
    assert_eq!(browser.handle(click(7, 6)), BrowserAction::Continue);
    assert!(browser.rename_value().is_some());
    draw(&mut browser, 80, 7);
    browser.handle(key(LogicalKey::Escape));
    assert_eq!(browser.handle(click(7, 6)), BrowserAction::Continue);
    assert!(browser.rename_value().is_none());

    browser.items[0].availability = BrowserAvailability::Trashed;
    browser.handle(key(LogicalKey::Enter));
    let terminal = draw(&mut browser, 80, 7);
    assert!(footer(&terminal).contains("Restore this session"));
    assert_eq!(browser.handle(click(3, 6)), BrowserAction::Continue);
    assert!(
        browser.rename_value().is_none(),
        "hidden Rename cannot be clicked"
    );
}
