package migrations

import (
	"database/sql"

	"github.com/pressly/goose/v3"
)

func init() {
	goose.AddMigration(upCreateRatingsTable, downCreateRatingsTable)
}

func upCreateRatingsTable(tx *sql.Tx) error {
	_, err := tx.Exec(`
		CREATE TABLE IF NOT EXISTS ratings (
			id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
			session_id UUID NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
			user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
			rating INT NOT NULL CHECK (rating >= 1 AND rating <= 5),
			comment TEXT,
			created_at TIMESTAMP WITH TIME ZONE DEFAULT now(),
			-- Ensure a user can only rate once per session
			UNIQUE (session_id, user_id)
		);

		CREATE INDEX IF NOT EXISTS idx_ratings_session_id ON ratings(session_id);
		CREATE INDEX IF NOT EXISTS idx_ratings_user_id ON ratings(user_id);
	`)
	return err
}

func downCreateRatingsTable(tx *sql.Tx) error {
	_, err := tx.Exec(`DROP TABLE IF EXISTS ratings;`)
	return err
}
