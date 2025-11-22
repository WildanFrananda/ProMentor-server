package migrations

import (
	"database/sql"

	"github.com/pressly/goose/v3"
)

func init() {
	goose.AddMigration(upCreateRefreshTokensTable, downCreateRefreshTokensTable)
}

func upCreateRefreshTokensTable(tx *sql.Tx) error {
	_, err := tx.Exec(`
		CREATE TABLE IF NOT EXISTS refresh_tokens (
			id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
			user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
			token_hash TEXT NOT NULL UNIQUE,
			expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
			created_at TIMESTAMP WITH TIME ZONE DEFAULT now()
		);
	`)
	return err
}

func downCreateRefreshTokensTable(tx *sql.Tx) error {
	_, err := tx.Exec(`DROP TABLE IF EXISTS refresh_tokens;`)
	return err
}
