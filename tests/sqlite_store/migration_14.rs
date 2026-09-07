use super::*;

const DOWNGRADE_TO_13: &str = r"
PRAGMA foreign_keys = OFF;
BEGIN IMMEDIATE;
DROP INDEX submission_attempt_items_active_thought;
ALTER TABLE submission_attempt_items RENAME TO submission_attempt_items_v14;
ALTER TABLE submission_attempts RENAME TO submission_attempts_v14;
CREATE TABLE submission_attempts (
    id BLOB PRIMARY KEY CHECK (length(id) = 16),
    session_id BLOB NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    thought_id BLOB NOT NULL REFERENCES thoughts(id) ON DELETE CASCADE,
    source_digest BLOB NOT NULL CHECK (length(source_digest) = 32),
    source_sequence INTEGER NOT NULL CHECK (source_sequence >= 0),
    disposition TEXT NOT NULL CHECK (disposition IN ('keep', 'remove_after_success')),
    route_version INTEGER NOT NULL CHECK (route_version IN (0, 1)),
    route_kind TEXT NOT NULL CHECK (
        route_kind IN ('adjacent_pane', 'herdr_agent')
        AND (route_version = 1 OR route_kind = 'adjacent_pane')
    ),
    direction TEXT CHECK (
        (route_kind = 'adjacent_pane' AND direction IN ('up', 'right', 'down', 'left'))
        OR (route_kind = 'herdr_agent' AND direction IS NULL)
    ),
    provider TEXT NOT NULL,
    protocol INTEGER NOT NULL CHECK (protocol >= 0),
    target_fingerprint BLOB NOT NULL CHECK (length(target_fingerprint) = 32),
    pre_state TEXT NOT NULL,
    post_state TEXT,
    error_code TEXT,
    deletion_operation_id BLOB CHECK (
        deletion_operation_id IS NULL OR length(deletion_operation_id) = 16
    ),
    state TEXT NOT NULL CHECK (
        state IN ('prepared', 'sending', 'accepted', 'failed', 'cancelled', 'outcome_unknown')
    ),
    prepared_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
) STRICT;
INSERT INTO submission_attempts(
    id, session_id, thought_id, source_digest, source_sequence, disposition,
    route_version, route_kind, direction, provider, protocol, target_fingerprint,
    pre_state, post_state, error_code, deletion_operation_id, state, prepared_at, updated_at
)
SELECT id, session_id, thought_id, source_digest, source_sequence, disposition,
       route_version, route_kind, direction, provider, protocol, target_fingerprint,
       pre_state, post_state, error_code, deletion_operation_id, state, prepared_at, updated_at
FROM submission_attempts_v14;
CREATE TABLE submission_attempt_items (
    submission_id BLOB NOT NULL REFERENCES submission_attempts(id) ON DELETE CASCADE,
    thought_id BLOB NOT NULL REFERENCES thoughts(id) ON DELETE CASCADE,
    ordinal INTEGER NOT NULL CHECK (ordinal >= 0),
    source_digest BLOB NOT NULL CHECK (length(source_digest) = 32),
    active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1)),
    PRIMARY KEY(submission_id, ordinal),
    UNIQUE(submission_id, thought_id)
) STRICT;
INSERT INTO submission_attempt_items(submission_id, thought_id, ordinal, source_digest, active)
SELECT submission_id, thought_id, ordinal, source_digest, active
FROM submission_attempt_items_v14;
DROP TABLE submission_attempt_items_v14;
DROP TABLE submission_attempts_v14;
CREATE UNIQUE INDEX submission_attempt_items_active_thought
ON submission_attempt_items(thought_id)
WHERE active = 1;
DELETE FROM migration_history WHERE version = 14;
UPDATE schema_meta SET schema_version = 13, storage_protocol = 12;
COMMIT;
PRAGMA foreign_keys = ON;
";

fn attempt(
    ids: &mut FakeIdGenerator,
    state: &AppState,
    thought_id: ThoughtId,
    route: proqi::ports::store::SubmissionJournalRoute,
    provider: &str,
    marker: u8,
) -> SubmissionAttempt {
    SubmissionAttempt {
        id: ids.submission_id(),
        session_id: state.board.session.id,
        sources: vec![SubmissionSource {
            thought_id,
            source_digest: [marker; 32],
        }],
        payload_digest: [marker; 32],
        source_sequence: state.board.session.last_durable_sequence,
        disposition: SubmissionDisposition::Keep,
        route,
        provider: provider.to_owned(),
        protocol: 45,
        target_fingerprint: [marker.saturating_add(1); 32],
        pre_state: AgentState::Idle,
        prepared_at: Timestamp::from_millis(40),
    }
}

#[test]
fn physical_v13_migration_widens_the_route_contract_without_touching_herdr_rows() {
    let (fixture, state, thurbox_thought) = physical_v13_database();
    let mut refused = fixture.config.clone();
    refused.migration_mode = MigrationMode::Refuse;
    assert!(matches!(
        SqliteStore::open(&refused),
        Err(StoreError::MigrationRequired {
            found: 13,
            supported: 14
        })
    ));

    let mut migrated = fixture.open();
    migrated.quick_check().expect("migrated integrity");
    let connection = Connection::open(&fixture.config.database_path).expect("migrated database");
    assert_eq!(
        route_rows(&connection),
        vec![("herdr_agent".to_owned(), None, vec![52_u8; 32])],
        "the existing Herdr journal row survives the widened contract untouched"
    );
    drop(connection);

    let mut ids = FakeIdGenerator::new(14_200);
    migrated
        .prepare_submission(&attempt(
            &mut ids,
            &state,
            thurbox_thought,
            proqi::ports::store::SubmissionJournalRoute::thurbox_session(),
            "thurbox",
            61,
        ))
        .expect("thurbox attempt is journaled after the migration");
    drop(migrated);

    let connection = Connection::open(&fixture.config.database_path).expect("journal");
    assert_eq!(
        route_rows(&connection),
        vec![
            ("herdr_agent".to_owned(), None, vec![52_u8; 32]),
            ("thurbox_session".to_owned(), None, vec![62_u8; 32]),
        ],
        "a global route never records a direction"
    );
    assert!(
        connection
            .execute(
                "UPDATE submission_attempts SET route_kind = 'invented_route'",
                [],
            )
            .is_err(),
        "the durable route vocabulary stays closed"
    );
    assert_eq!(
        std::fs::read_dir(&fixture.config.backup_dir)
            .expect("migration backup")
            .count(),
        1
    );
}

fn physical_v13_database() -> (DatabaseFixture, AppState, ThoughtId) {
    let fixture = DatabaseFixture::new();
    let mut store = fixture.open();
    let mut ids = FakeIdGenerator::new(14_100);
    let mut state = session_state(&mut ids, &test_path("migration-14"));
    store
        .commit(&OperationBatch::CreateSession(state.board.session.clone()))
        .expect("create session");
    let herdr_thought = create_thought(&mut store, &mut state, &mut ids, "herdr", 2);
    let thurbox_thought = create_thought(&mut store, &mut state, &mut ids, "thurbox", 3);
    store
        .prepare_submission(&attempt(
            &mut ids,
            &state,
            herdr_thought,
            proqi::ports::store::SubmissionJournalRoute::herdr_agent(),
            "herdr",
            51,
        ))
        .expect("herdr attempt");
    drop(store);

    let connection = Connection::open(&fixture.config.database_path).expect("current database");
    connection
        .execute_batch(DOWNGRADE_TO_13)
        .expect("physical v13 database");
    drop(connection);
    (fixture, state, thurbox_thought)
}

fn route_rows(connection: &Connection) -> Vec<(String, Option<String>, Vec<u8>)> {
    connection
        .prepare(
            "SELECT route_kind, direction, target_fingerprint
             FROM submission_attempts ORDER BY prepared_at, id",
        )
        .expect("route query")
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Vec<u8>>(2)?,
            ))
        })
        .expect("route rows")
        .collect::<Result<Vec<_>, _>>()
        .expect("collect route rows")
}
