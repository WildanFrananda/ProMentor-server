package repository_test

import (
	"context"
	"database/sql"
	"regexp"
	"testing"
	"time"

	"auth-service/internal/model"
	repo "auth-service/internal/repository"

	"github.com/DATA-DOG/go-sqlmock"
	"github.com/google/uuid"
	"github.com/jmoiron/sqlx"
	"github.com/stretchr/testify/require"
)

func TestPostgresSessionRepository_Create(t *testing.T) {
	db, mock, err := sqlmock.New()
	require.NoError(t, err)
	defer db.Close()

	sqlxDB := sqlx.NewDb(db, "sqlmock")
	r := repo.NewPostgresSessionRepository(sqlxDB)

	// expect insert returning id and created_at
	id := uuid.New()
	now := time.Now()
	mock.ExpectQuery(regexp.QuoteMeta(`
		INSERT INTO sessions (coach_id, title, description, start_at, capacity)
		VALUES ($1, $2, $3, $4, $5)
		RETURNING id, created_at
	`)).WithArgs(sqlmock.AnyArg(), "T", sqlmock.AnyArg(), sqlmock.AnyArg(), 2).
		WillReturnRows(sqlmock.NewRows([]string{"id", "created_at"}).AddRow(id, now))

	sess := &model.Session{CoachID: uuid.New(), Title: "T", StartAt: time.Now(), Capacity: 2}
	created, err := r.Create(context.Background(), sess)
	require.NoError(t, err)
	require.Equal(t, id, created.ID)
	require.NoError(t, mock.ExpectationsWereMet())
}

func TestPostgresSessionRepository_FindByID_NoRows(t *testing.T) {
	db, mock, err := sqlmock.New()
	require.NoError(t, err)
	defer db.Close()

	sqlxDB := sqlx.NewDb(db, "sqlmock")
	r := repo.NewPostgresSessionRepository(sqlxDB)

	mock.ExpectQuery(regexp.QuoteMeta(`SELECT * FROM sessions WHERE id = $1`)).WithArgs(sqlmock.AnyArg()).WillReturnError(sql.ErrNoRows)

	s, err := r.FindByID(context.Background(), uuid.New())
	require.NoError(t, err)
	require.Nil(t, s)
	require.NoError(t, mock.ExpectationsWereMet())
}

func TestPostgresSessionRepository_AddParticipantAndCount(t *testing.T) {
	db, mock, err := sqlmock.New()
	require.NoError(t, err)
	defer db.Close()

	sqlxDB := sqlx.NewDb(db, "sqlmock")
	r := repo.NewPostgresSessionRepository(sqlxDB)

	mock.ExpectExec(regexp.QuoteMeta(`
		INSERT INTO session_participants (session_id, user_id, role)
		VALUES ($1, $2, $3)
	`)).WithArgs(sqlmock.AnyArg(), sqlmock.AnyArg(), "attendee").WillReturnResult(sqlmock.NewResult(1, 1))

	mock.ExpectQuery(regexp.QuoteMeta(`SELECT COUNT(*) FROM session_participants WHERE session_id = $1`)).WithArgs(sqlmock.AnyArg()).WillReturnRows(sqlmock.NewRows([]string{"count"}).AddRow(1))

	err = r.AddParticipant(context.Background(), uuid.New(), uuid.New(), "attendee")
	require.NoError(t, err)

	cnt, err := r.CountParticipants(context.Background(), uuid.New())
	require.NoError(t, err)
	require.Equal(t, 1, cnt)

	require.NoError(t, mock.ExpectationsWereMet())
}
