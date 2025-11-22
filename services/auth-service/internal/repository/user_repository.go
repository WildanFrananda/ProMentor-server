package repository

import (
	"context"
	"fmt"

	"github.com/google/uuid"
	"github.com/jmoiron/sqlx"

	"auth-service/internal/model"
)

type UserRepository interface {
	Create(ctx context.Context, user *model.User) (uuid.UUID, error)
	FindByEmail(ctx context.Context, email string) (*model.User, error)
	FindByID(ctx context.Context, id uuid.UUID) (*model.User, error)
}

type postgresUserRepository struct {
	db *sqlx.DB
}

func NewPostgresUserRepository(db *sqlx.DB) UserRepository {
	return &postgresUserRepository{db: db}
}

func (r *postgresUserRepository) Create(ctx context.Context, user *model.User) (uuid.UUID, error) {
	query := `INSERT INTO users (email, password_hash, name) VALUES ($1, $2, $3) RETURNING id`
	var newID uuid.UUID
	err := r.db.QueryRowxContext(ctx, query, user.Email, user.PasswordHash, user.Name).Scan(&newID)

	if err != nil {
		return uuid.Nil, err
	}

	return newID, nil
}

func (r *postgresUserRepository) FindByEmail(ctx context.Context, email string) (*model.User, error) {
	var user model.User
	query := `SELECT id, email, password_hash, name, avatar_url, created_at, updated_at FROM users WHERE email = $1`
	err := r.db.GetContext(ctx, &user, query, email)

	if err != nil {
		return nil, err
	}

	return &user, nil
}

func (r *postgresUserRepository) FindByID(ctx context.Context, id uuid.UUID) (*model.User, error) {
	var user model.User
	query := `SELECT id, email, name, avatar_url, created_at, updated_at FROM users WHERE id = $1`
	err := r.db.GetContext(ctx, &user, query, id)

	fmt.Printf("DEBUG FindByID: %+v\n", user)

	if err != nil {
		return nil, err
	}

	return &user, nil
}
