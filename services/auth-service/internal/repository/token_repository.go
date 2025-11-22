package repository

import (
	"auth-service/internal/model"
	"context"

	"github.com/jmoiron/sqlx"
)

type TokenRepository interface {
	Create(ctx context.Context, token *model.RefreshToken) error
	FindByTokenHash(ctx context.Context, tokenHash string) (*model.RefreshToken, error)
	Delete(ctx context.Context, tokenHash string) error
}

type postgresTokenRepository struct {
	db *sqlx.DB
}

func NewPostgresTokenRepository(db *sqlx.DB) TokenRepository {
	return &postgresTokenRepository{db: db}
}

func (r *postgresTokenRepository) Create(ctx context.Context, token *model.RefreshToken) error {
	query := `INSERT INTO refresh_tokens (user_id, token_hash, expires_at) VALUES ($1, $2, $3)`
	_, err := r.db.ExecContext(ctx, query, token.UserID, token.TokenHash, token.ExpiresAt)
	return err
}

func (r *postgresTokenRepository) FindByTokenHash(ctx context.Context, tokenHash string) (*model.RefreshToken, error) {
	var token model.RefreshToken
	query := `SELECT * FROM refresh_tokens WHERE token_hash = $1`
	err := r.db.GetContext(ctx, &token, query, tokenHash)
	return &token, err
}

func (r *postgresTokenRepository) Delete(ctx context.Context, tokenHash string) error {
	query := `DELETE FROM refresh_tokens WHERE token_hash = $1`
	_, err := r.db.ExecContext(ctx, query, tokenHash)
	return err
}
