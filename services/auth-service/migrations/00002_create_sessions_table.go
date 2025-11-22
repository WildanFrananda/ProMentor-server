package migrations

import (
	"context"
	"database/sql"

	"github.com/pressly/goose/v3"
)

func init() {
	goose.AddMigrationContext(upCreateSessionsTable, downCreateSessionsTable)
}

func upCreateSessionsTable(ctx context.Context, tx *sql.Tx) error {
	query := `
		CREATE TABLE sessions (
			id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
			coach_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
			title TEXT NOT NULL,
			description TEXT,
			start_at TIMESTAMP WITH TIME ZONE NOT NULL,
			end_at TIMESTAMP WITH TIME ZONE,
			capacity INT DEFAULT 1,
			created_at TIMESTAMP WITH TIME ZONE DEFAULT now()
		);
	`

	_, err := tx.ExecContext(ctx, query)

	if err != nil {
		return err
	}

	return nil
}

func downCreateSessionsTable(ctx context.Context, tx *sql.Tx) error {
	query := `DROP TABLE IF EXISTS sessions;`
	_, err := tx.ExecContext(ctx, query)

	if err != nil {
		return err
	}

	return nil
}
