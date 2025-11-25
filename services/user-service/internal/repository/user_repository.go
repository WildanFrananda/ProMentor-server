package repository

import (
	"context"
	"fmt"
	"strings"
	"user-service/internal/model"

	"github.com/google/uuid"
	"github.com/jmoiron/sqlx"
)

type UserRepository interface {
	FindByID(ctx context.Context, id uuid.UUID) (*model.User, error)
	Update(ctx context.Context, user *model.User) error
	RegisterDeviceToken(ctx context.Context, userID uuid.UUID, token string) error
}

type postgresUserRepository struct {
	db *sqlx.DB
}

func NewPostgresUserRepository(db *sqlx.DB) UserRepository {
	return &postgresUserRepository{db: db}
}

func (r *postgresUserRepository) FindByID(ctx context.Context, id uuid.UUID) (*model.User, error) {
	var user model.User
	query := `SELECT id, name, email, role, avatar_url FROM users WHERE id = $1`
	err := r.db.GetContext(ctx, &user, query, id)

	return &user, err
}

func (r *postgresUserRepository) Update(ctx context.Context, user *model.User) error {
	var setClauses []string
	var args []interface{}
	argId := 1

	if user.Name != nil {
		setClauses = append(setClauses, fmt.Sprintf("name = $%d", argId))
		args = append(args, *user.Name)
		argId++
	}
	if user.AvatarURL != nil {
		setClauses = append(setClauses, fmt.Sprintf("avatar_url = $%d", argId))
		args = append(args, *user.AvatarURL)
		argId++
	}

	if len(setClauses) == 0 {
		return nil
	}

	query := fmt.Sprintf("UPDATE users SET %s WHERE id = $%d", strings.Join(setClauses, ", "), argId)
	args = append(args, user.ID)

	_, err := r.db.ExecContext(ctx, query, args...)
	return err
}

func (r *postgresUserRepository) RegisterDeviceToken(ctx context.Context, userID uuid.UUID, token string) error {
	query := `
		INSERT INTO user_device_tokens (user_id, device_token)
		VALUES ($1, $2)
		ON CONFLICT (device_token) DO UPDATE SET user_id = $1
	`
	_, err := r.db.ExecContext(ctx, query, userID, token)
	return err
}
