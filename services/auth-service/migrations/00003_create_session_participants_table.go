package migrations

import (
	"context"
	"database/sql"

	"github.com/pressly/goose/v3"
)

func init() {
	goose.AddMigrationContext(upCreateSessionParticipantsTable, downCreateSessionParticipantsTable)
}

func upCreateSessionParticipantsTable(ctx context.Context, tx *sql.Tx) error {
	query := `
		CREATE TABLE session_participants (
			id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
			session_id UUID NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
			user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
			joined_at TIMESTAMP WITH TIME ZONE DEFAULT now(),
			role TEXT NOT NULL,
			UNIQUE(session_id, user_id)
		);
	`

	_, err := tx.ExecContext(ctx, query)

	if err != nil {
		return err
	}

	return nil
}

func downCreateSessionParticipantsTable(ctx context.Context, tx *sql.Tx) error {
	query := `DROP TABLE IF EXISTS session_participants;`
	_, err := tx.ExecContext(ctx, query)
	if err != nil {
		return err
	}
	return nil
}
