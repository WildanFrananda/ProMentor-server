package migrations

import (
	"database/sql"

	"github.com/pressly/goose/v3"
)

func init() {
	goose.AddMigration(upAddAvatarUrlToUsers, downAddAvatarUrlToUsers)
}

func upAddAvatarUrlToUsers(tx *sql.Tx) error {
	_, err := tx.Exec(`ALTER TABLE users ADD COLUMN avatar_url TEXT;`)
	return err
}

func downAddAvatarUrlToUsers(tx *sql.Tx) error {
	_, err := tx.Exec(`ALTER TABLE users DROP COLUMN avatar_url;`)
	return err
}
