package migrations

import (
	"database/sql"

	"github.com/pressly/goose/v3"
)

func init() {
	goose.AddMigration(upAddRolesAndCategories, downAddRolesAndCategories)
}

func upAddRolesAndCategories(tx *sql.Tx) error {
	_, err := tx.Exec(`
		ALTER TABLE users ADD COLUMN role TEXT NOT NULL DEFAULT 'attendee';
		ALTER TABLE users ADD CONSTRAINT check_role CHECK (role IN ('coach', 'attendee', 'admin'));
	`)
	if err != nil {
		return err
	}

	_, err = tx.Exec(`
		CREATE TABLE IF NOT EXISTS categories (
			id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
			name TEXT NOT NULL UNIQUE,
			icon TEXT NOT NULL
		);
		
		-- Seed data categories
		INSERT INTO categories (name, icon) VALUES 
		('Technology', 'laptop'),
		('Health', 'heart'),
		('Business', 'briefcase'),
		('Design', 'palette');
	`)
	if err != nil {
		return err
	}

	_, err = tx.Exec(`
		ALTER TABLE sessions ADD COLUMN category_id UUID REFERENCES categories(id);
	`)
	return err
}

func downAddRolesAndCategories(tx *sql.Tx) error {
	_, err := tx.Exec(`
		ALTER TABLE sessions DROP COLUMN category_id;
		DROP TABLE categories;
		ALTER TABLE users DROP COLUMN role;
	`)
	return err
}
