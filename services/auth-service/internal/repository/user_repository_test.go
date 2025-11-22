package repository_test

import (
	"context"
	"database/sql"
	"regexp"
	"testing"

	"auth-service/internal/model"
	repo "auth-service/internal/repository"

	"github.com/DATA-DOG/go-sqlmock"
	"github.com/google/uuid"
	"github.com/jmoiron/sqlx"
	"github.com/stretchr/testify/require"
)

func TestPostgresUserRepository_Create(t *testing.T) {
	db, mock, err := sqlmock.New()
	require.NoError(t, err)
	defer db.Close()

	sqlxDB := sqlx.NewDb(db, "sqlmock")
	r := repo.NewPostgresUserRepository(sqlxDB)

	// expect query with RETURNING id
	id := uuid.New()
	mock.ExpectQuery(regexp.QuoteMeta(`INSERT INTO users (email, password_hash, name) VALUES ($1, $2, $3) RETURNING id`)).
		WithArgs("a@b.com", "hash", "Name").
		WillReturnRows(sqlmock.NewRows([]string{"id"}).AddRow(id))

	nid, err := r.Create(context.Background(), &model.User{Email: "a@b.com", PasswordHash: "hash", Name: "Name"})
	require.NoError(t, err)
	require.Equal(t, id, nid)
	require.NoError(t, mock.ExpectationsWereMet())
}

func TestPostgresUserRepository_FindByEmail_Success(t *testing.T) {
	db, mock, err := sqlmock.New()
	require.NoError(t, err)
	defer db.Close()

	sqlxDB := sqlx.NewDb(db, "sqlmock")
	r := repo.NewPostgresUserRepository(sqlxDB)

	id := uuid.New()
	rows := sqlmock.NewRows([]string{"id", "email", "password_hash", "name"}).AddRow(id, "a@b.com", "hash", "Name")
	mock.ExpectQuery(regexp.QuoteMeta(`SELECT id, email, password_hash, name FROM users WHERE email = $1`)).
		WithArgs("a@b.com").WillReturnRows(rows)

	u, err := r.FindByEmail(context.Background(), "a@b.com")
	require.NoError(t, err)
	require.Equal(t, "a@b.com", u.Email)
	require.NoError(t, mock.ExpectationsWereMet())
}

func TestPostgresUserRepository_FindByID_Error(t *testing.T) {
	db, mock, err := sqlmock.New()
	require.NoError(t, err)
	defer db.Close()

	sqlxDB := sqlx.NewDb(db, "sqlmock")
	r := repo.NewPostgresUserRepository(sqlxDB)

	mock.ExpectQuery(regexp.QuoteMeta(`SELECT id, email, name, created_at, updated_at FROM users WHERE id = $1`)).WithArgs(sqlmock.AnyArg()).WillReturnError(sql.ErrNoRows)

	_, err = r.FindByID(context.Background(), uuid.New())
	require.Error(t, err)
	require.NoError(t, mock.ExpectationsWereMet())
}
