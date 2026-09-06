//! Canonical first-run practice-board policy, copy, and semantic shortcut emphasis.

use crate::{
    domain::{DomainError, Session, SessionBoard, Timestamp},
    ports::{
        environment::IdGenerator,
        store::{FirstRunBoard, OnboardingVersion},
    },
};

use super::{
    AppState, ApplicationError, ApplicationResult, Effect,
    instructional_text::{InstructionalText, InstructionalTextBuilder},
    reduce,
};

/// Cheap, truthful local environment distinction used only to select practice copy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FirstRunEnvironment {
    /// The current Proqi pane carries Herdr's managed-pane signal.
    HerdrManaged,
    /// The current Proqi pane is not managed by Herdr.
    Standalone,
}

/// Build the current practice board from ordinary application-created thoughts.
///
/// # Errors
///
/// Returns an application error if the session, generated ordering, or reviewed
/// instructional annotations are invalid.
pub fn first_run_board(
    session: Session,
    ids: &mut impl IdGenerator,
    environment: FirstRunEnvironment,
) -> ApplicationResult<FirstRunBoard> {
    let now = session.created_at;
    let empty = SessionBoard::new(session, Vec::new())?;
    let mut state = AppState::new(empty);
    let instructions = instructions(environment)?;
    for (insertion_index, instruction) in instructions.into_iter().enumerate() {
        add_instruction(&mut state, ids, instruction, insertion_index, now)?;
    }
    Ok(FirstRunBoard::new(
        OnboardingVersion::PRACTICE_BOARD,
        state.board,
    ))
}

fn add_instruction(
    state: &mut AppState,
    ids: &mut impl IdGenerator,
    instruction: InstructionalText,
    insertion_index: usize,
    at: Timestamp,
) -> ApplicationResult<()> {
    let effects = reduce(
        state,
        instruction.create_action(
            ids.thought_id(),
            ids.operation_id(),
            Some(insertion_index),
            at,
        ),
    )?;
    if !matches!(effects.as_slice(), [Effect::CommitBoardOperation(_)]) {
        return Err(ApplicationError::InvalidState);
    }
    Ok(())
}

fn instructions(environment: FirstRunEnvironment) -> Result<[InstructionalText; 6], DomainError> {
    let welcome = InstructionalTextBuilder::new()
        .text("Welcome to Proqi!\n\nProqi is a prompt composer designed to replace common agent input methods. Capture, refine, organize, and submit prompts here.")
        .finish()?;
    let editing = InstructionalTextBuilder::new()
        .text("Edit the focused thought using its current shortcut in Help. Press ")
        .shortcut("Esc")?
        .text(" to return to board mode.\n\n- Use Newline to continue this unordered list. Use Delete logical line to remove this line. Use Delete sentence to remove this sentence.")
        .finish()?;
    let creation = InstructionalTextBuilder::new()
        .text("Choose New in the footer to create a thought, or paste in board mode to create one from the pasted text. Choose Copy to copy the focused thought.\n\nIn edit mode, type $name, /name, or supported @name to complete discovered local invocations.")
        .finish()?;
    let navigation = InstructionalTextBuilder::new()
        .text("Move focus between thoughts, select, delete, and undo using your current bindings. Open Shortcuts in the footer to see the resolved keys for this context. Commands provides additional actions.")
        .finish()?;
    let integration = InstructionalTextBuilder::new()
        .text(match environment {
            FirstRunEnvironment::HerdrManaged => "/plan starts a planning prompt when a compatible adjacent Codex or Claude Code agent is verified.\n\nHerdr is detected. With a compatible adjacent agent verified, use Submit to submit and remove, or Submit & keep to retain the thought. Current shortcuts appear in the footer and Help. Learn more at https://herdr.dev",
            FirstRunEnvironment::Standalone => "Proqi works on its own. Herdr adds verified adjacent submission. Use Submit to submit and remove, or Submit & keep to retain the thought. Current shortcuts appear in the footer and Help. Learn more at https://herdr.dev",
        })
        .finish()?;
    let deletion = InstructionalTextBuilder::new()
        .text("Use Select all, then Delete in board mode to delete this entire practice board. The footer and Help show your current bindings, including custom aliases.")
        .finish()?;
    Ok([
        welcome,
        editing,
        creation,
        navigation,
        integration,
        deletion,
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        adapters::memory::FakeIdGenerator,
        domain::{AnnotationBehavior, InlineStyleKind},
    };

    #[test]
    fn variants_have_six_exact_ordered_platform_specific_thoughts() {
        for environment in [
            FirstRunEnvironment::HerdrManaged,
            FirstRunEnvironment::Standalone,
        ] {
            let board = board(environment);
            assert_eq!(contents(&board), expected_contents(environment));
            assert!(
                contents(&board)
                    .iter()
                    .all(|content| !content.contains("Primary+"))
            );
        }
    }

    #[test]
    fn reviewed_shortcut_literals_are_the_only_semantic_emphasis() {
        let expected = expected_shortcuts();
        for environment in [
            FirstRunEnvironment::HerdrManaged,
            FirstRunEnvironment::Standalone,
        ] {
            assert_eq!(shortcut_literals(&board(environment)), expected);
        }
    }

    fn board(environment: FirstRunEnvironment) -> SessionBoard {
        let mut ids = FakeIdGenerator::new(1_725_200_000_000);
        let session = Session::new(
            ids.session_id(),
            std::env::temp_dir().join("proqi-onboarding-copy"),
            Timestamp::from_millis(1),
        )
        .expect("session");
        first_run_board(session, &mut ids, environment)
            .expect("practice board")
            .board()
            .clone()
    }

    fn expected_contents(environment: FirstRunEnvironment) -> Vec<String> {
        let integration = match environment {
            FirstRunEnvironment::HerdrManaged => {
                "/plan starts a planning prompt when a compatible adjacent Codex or Claude Code agent is verified.\n\nHerdr is detected. With a compatible adjacent agent verified, use Submit to submit and remove, or Submit & keep to retain the thought. Current shortcuts appear in the footer and Help. Learn more at https://herdr.dev"
            }
            FirstRunEnvironment::Standalone => {
                "Proqi works on its own. Herdr adds verified adjacent submission. Use Submit to submit and remove, or Submit & keep to retain the thought. Current shortcuts appear in the footer and Help. Learn more at https://herdr.dev"
            }
        };
        vec![
            "Welcome to Proqi!\n\nProqi is a prompt composer designed to replace common agent input methods. Capture, refine, organize, and submit prompts here.".to_owned(),
            "Edit the focused thought using its current shortcut in Help. Press Esc to return to board mode.\n\n- Use Newline to continue this unordered list. Use Delete logical line to remove this line. Use Delete sentence to remove this sentence.".to_owned(),
            "Choose New in the footer to create a thought, or paste in board mode to create one from the pasted text. Choose Copy to copy the focused thought.\n\nIn edit mode, type $name, /name, or supported @name to complete discovered local invocations.".to_owned(),
            "Move focus between thoughts, select, delete, and undo using your current bindings. Open Shortcuts in the footer to see the resolved keys for this context. Commands provides additional actions.".to_owned(),
            integration.to_owned(),
            "Use Select all, then Delete in board mode to delete this entire practice board. The footer and Help show your current bindings, including custom aliases.".to_owned(),
        ]
    }

    fn expected_shortcuts() -> Vec<String> {
        vec!["Esc".to_owned()]
    }

    fn contents(board: &SessionBoard) -> Vec<&str> {
        board
            .live_thoughts()
            .iter()
            .map(|thought| thought.content.as_str())
            .collect()
    }

    fn shortcut_literals(board: &SessionBoard) -> Vec<&str> {
        board
            .live_thoughts()
            .iter()
            .flat_map(|thought| {
                thought.annotations.iter().map(|annotation| {
                    assert_eq!(
                        annotation.kind.behavior(),
                        AnnotationBehavior::InlineStyle(InlineStyleKind::ShortcutEmphasis)
                    );
                    &thought.content[annotation.start..annotation.end]
                })
            })
            .collect()
    }
}
