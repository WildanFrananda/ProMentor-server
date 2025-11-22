package migrations

import (
	"database/sql"

	"github.com/pressly/goose/v3"
)

func init() {
	goose.AddMigration(upCreateUserDeviceTokens, downCreateUserDeviceTokens)
}

func upCreateUserDeviceTokens(tx *sql.Tx) error {
	_, err := tx.Exec(`
		CREATE TABLE IF NOT EXISTS user_device_tokens (
			id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
			user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
			device_token TEXT NOT NULL UNIQUE,
			created_at TIMESTAMP WITH TIME ZONE DEFAULT now()
		);

		CREATE INDEX IF NOT EXISTS idx_user_device_tokens_user_id ON user_device_tokens(user_id);
	`)
	return err
}

func downCreateUserDeviceTokens(tx *sql.Tx) error {
	_, err := tx.Exec(`DROP TABLE IF EXISTS user_device_tokens;`)
	return err
}
