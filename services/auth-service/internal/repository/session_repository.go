package repository

import (
	"auth-service/internal/model"
	"context"
	"database/sql"
	"strconv"

	"github.com/google/uuid"
	"github.com/jmoiron/sqlx"
)

type SessionRepository interface {
	Create(ctx context.Context, session *model.Session) (*model.Session, error)
	FindByID(ctx context.Context, sessionID uuid.UUID) (*model.Session, error)
	AddParticipant(ctx context.Context, sessionID, userID uuid.UUID, role string) error
	CountParticipants(ctx context.Context, sessionID uuid.UUID) (int, error)
	ListUpcoming(ctx context.Context, limit int, offset int, query string) ([]model.SessionDetails, error)
	ListHistoryByUserID(ctx context.Context, userID uuid.UUID) ([]model.SessionDetails, error)
}

type postgresSessionRepository struct {
	db *sqlx.DB
}

func NewPostgresSessionRepository(db *sqlx.DB) SessionRepository {
	return &postgresSessionRepository{db: db}
}

func (r *postgresSessionRepository) Create(ctx context.Context, session *model.Session) (*model.Session, error) {
	query := `
		INSERT INTO sessions (coach_id, title, description, start_at, capacity)
		VALUES ($1, $2, $3, $4, $5)
		RETURNING id, created_at
	`

	row := r.db.QueryRowxContext(ctx, query, session.CoachID, session.Title, session.Description, session.StartAt, session.Capacity)
	err := row.Scan(&session.ID, &session.CreatedAt)

	if err != nil {
		return nil, err
	}

	return session, nil
}

func (r *postgresSessionRepository) FindByID(ctx context.Context, sessionID uuid.UUID) (*model.Session, error) {
	var session model.Session
	query := `SELECT * FROM sessions WHERE id = $1`
	err := r.db.GetContext(ctx, &session, query, sessionID)

	if err != nil {
		if err == sql.ErrNoRows {
			return nil, nil
		}

		return nil, err
	}

	return &session, nil
}

func (r *postgresSessionRepository) AddParticipant(ctx context.Context, sessionID, userID uuid.UUID, role string) error {
	query := `
		INSERT INTO session_participants (session_id, user_id, role)
		VALUES ($1, $2, $3)
	`
	_, err := r.db.ExecContext(ctx, query, sessionID, userID, role)
	return err
}

func (r *postgresSessionRepository) CountParticipants(ctx context.Context, sessionID uuid.UUID) (int, error) {
	var count int
	query := `SELECT COUNT(*) FROM session_participants WHERE session_id = $1`
	err := r.db.GetContext(ctx, &count, query, sessionID)

	if err != nil {
		return 0, err
	}

	return count, nil
}

func (r *postgresSessionRepository) ListUpcoming(ctx context.Context, limit int, offset int, query string) ([]model.SessionDetails, error) {
	var sessions []model.SessionDetails
	sqlQuery := `
		SELECT s.id, s.title, s.description, s.start_at, s.capacity, s.coach_id, u.name as coach_name
		FROM sessions s
		LEFT JOIN users u ON s.coach_id = u.id
		WHERE s.start_at > NOW()`

	var args []interface{}
	argCount := 1

	if query != "" {
		sqlQuery += " AND (s.title ILIKE $" + strconv.Itoa(argCount) + " OR s.description ILIKE $" + strconv.Itoa(argCount) + ")"
		args = append(args, "%"+query+"%")
		argCount++
	}

	sqlQuery += " ORDER BY s.start_at ASC LIMIT $" + strconv.Itoa(argCount) + " OFFSET $" + strconv.Itoa(argCount+1)
	args = append(args, limit, offset)

	err := r.db.SelectContext(ctx, &sessions, sqlQuery, args...)
	return sessions, err
}

func (r *postgresSessionRepository) ListHistoryByUserID(ctx context.Context, userID uuid.UUID) ([]model.SessionDetails, error) {
	var sessions []model.SessionDetails
	query := `
		SELECT s.id, s.title, s.description, s.start_at, s.capacity, s.coach_id, u.name as coach_name
		FROM sessions s
		LEFT JOIN users u ON s.coach_id = u.id
		JOIN session_participants sp ON s.id = sp.session_id
		WHERE sp.user_id = $1
		ORDER BY s.start_at DESC;
	`
	err := r.db.SelectContext(ctx, &sessions, query, userID)
	return sessions, err
}
