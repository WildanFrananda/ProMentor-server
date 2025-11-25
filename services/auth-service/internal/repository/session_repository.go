package repository

import (
	"auth-service/internal/model"
	"context"
	"database/sql"
	"fmt"

	"github.com/google/uuid"
	"github.com/jmoiron/sqlx"
)

type PaginationMeta struct {
	CurrentPage int `json:"current_page"`
	TotalPages  int `json:"total_pages"`
	TotalItems  int `json:"total_items"`
	PerPage     int `json:"per_page"`
}

type PaginatedSessions struct {
	Data []model.SessionDetails `json:"data"`
	Meta PaginationMeta         `json:"meta"`
}

type SessionRepository interface {
	Create(ctx context.Context, session *model.Session) (*model.Session, error)
	FindByID(ctx context.Context, sessionID uuid.UUID) (*model.Session, error)
	AddParticipant(ctx context.Context, sessionID, userID uuid.UUID, role string) error
	CountParticipants(ctx context.Context, sessionID uuid.UUID) (int, error)
	ListUpcoming(ctx context.Context, categoryID string, page int, limit int) (*PaginatedSessions, error)
	ListHistoryByUserID(ctx context.Context, userID uuid.UUID) ([]model.SessionDetails, error)
	GetCategories(ctx context.Context) ([]model.Category, error)
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

func (r *postgresSessionRepository) ListUpcoming(ctx context.Context, categoryID string, page int, limit int) (*PaginatedSessions, error) {
	offset := (page - 1) * limit

	baseQuery := `
		SELECT 
			s.id, 
			COALESCE(s.title, 'Untitled Session') as title, -- Safety check
			COALESCE(s.description, '') as description,
			s.start_at, 
			COALESCE(s.capacity, 0) as capacity,
			s.coach_id, 
			COALESCE(u.name, 'Unknown Coach') as coach_name,
            s.category_id
		FROM sessions s
		LEFT JOIN users u ON s.coach_id = u.id
		WHERE s.start_at > NOW()
	`

	args := []interface{}{}
	argId := 1
	if categoryID != "" {
		baseQuery += fmt.Sprintf(" AND s.category_id = $%d", argId)
		args = append(args, categoryID)
		argId++
	}

	countQuery := "SELECT COUNT(*) FROM (" + baseQuery + ") as count_query"
	var totalItems int
	err := r.db.GetContext(ctx, &totalItems, countQuery, args...)
	if err != nil {
		return nil, err
	}

	baseQuery += fmt.Sprintf(" ORDER BY s.start_at ASC LIMIT $%d OFFSET $%d", argId, argId+1)
	args = append(args, limit, offset)

	var sessions []model.SessionDetails
	err = r.db.SelectContext(ctx, &sessions, baseQuery, args...)
	if err != nil {
		return nil, err
	}

	if sessions == nil {
		sessions = []model.SessionDetails{}
	}

	totalPages := (totalItems + limit - 1) / limit

	return &PaginatedSessions{
		Data: sessions,
		Meta: PaginationMeta{
			CurrentPage: page,
			TotalPages:  totalPages,
			TotalItems:  totalItems,
			PerPage:     limit,
		},
	}, nil
}

func (r *postgresSessionRepository) GetCategories(ctx context.Context) ([]model.Category, error) {
	var categories []model.Category
	err := r.db.SelectContext(ctx, &categories, "SELECT id, name, icon FROM categories")
	return categories, err
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
