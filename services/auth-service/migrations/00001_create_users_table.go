package migrations

import (
	"context"
	"database/sql"

	"github.com/pressly/goose/v3"
)

func init() {
	goose.AddMigrationContext(upCreateUsersTable, downCreateUsersTable)
}

func upCreateUsersTable(ctx context.Context, tx *sql.Tx) error {
	query := `
	CREATE TABLE users (
	  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
	  email TEXT UNIQUE NOT NULL,
	  password_hash TEXT NOT NULL,
	  name TEXT,
	  created_at TIMESTAMP WITH TIME ZONE DEFAULT now(),
	  updated_at TIMESTAMP WITH TIME ZONE DEFAULT now(),
	  last_login TIMESTAMP WITH TIME ZONE
	);
	`

	_, err := tx.ExecContext(ctx, query)

	if err != nil {
		return err
	}

	return nil
}

func downCreateUsersTable(ctx context.Context, tx *sql.Tx) error {
	query := `DROP TABLE IF EXISTS users;`
	_, err := tx.ExecContext(ctx, query)
	if err != nil {
		return err
	}
	return nil
}
