package repository

import (
	"auth-service/internal/model"
	"context"
	"database/sql"

	"github.com/google/uuid"
	"github.com/jmoiron/sqlx"
)

type RatingRepository interface {
	Create(ctx context.Context, rating *model.Rating) error
	CheckIfUserJoinedSession(ctx context.Context, userID, sessionID uuid.UUID) (bool, error)
}

type postgresRatingRepository struct {
	db *sqlx.DB
}

func NewPostgresRatingRepository(db *sqlx.DB) RatingRepository {
	return &postgresRatingRepository{db: db}
}

func (r *postgresRatingRepository) Create(ctx context.Context, rating *model.Rating) error {
	query := `
		INSERT INTO ratings (session_id, user_id, rating, comment)
		VALUES ($1, $2, $3, $4)
	`
	_, err := r.db.ExecContext(ctx, query, rating.SessionID, rating.UserID, rating.Rating, rating.Comment)
	return err
}

func (r *postgresRatingRepository) CheckIfUserJoinedSession(ctx context.Context, userID, sessionID uuid.UUID) (bool, error) {
	var exists bool
	query := `SELECT EXISTS(SELECT 1 FROM session_participants WHERE user_id = $1 AND session_id = $2)`
	err := r.db.GetContext(ctx, &exists, query, userID, sessionID)
	if err != nil && err != sql.ErrNoRows {
		return false, err
	}
	return exists, nil
}
