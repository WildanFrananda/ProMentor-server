package db

import (
	"context"
	"fmt"
	"log"
	"os"

	"github.com/google/uuid"
	_ "github.com/jackc/pgx/v5/stdlib"
	"github.com/jmoiron/sqlx"
)

type Repository interface {
	GetUserDeviceTokens(ctx context.Context, userID uuid.UUID) ([]string, error)
	Close()
}

type postgresRepository struct {
	db *sqlx.DB
}

func NewRepository() (Repository, error) {
	dbUser := os.Getenv("DB_USER")
	dbPassword := os.Getenv("DB_PASSWORD")
	dbHost := os.Getenv("DB_HOST")
	dbPort := os.Getenv("DB_PORT")
	dbName := os.Getenv("DB_NAME")

	dbURL := fmt.Sprintf("postgres://%s:%s@%s:%s/%s?sslmode=disable",
		dbUser, dbPassword, dbHost, dbPort, dbName,
	)

	db, err := sqlx.Connect("pgx", dbURL)
	if err != nil {
		return nil, fmt.Errorf("failed to connect to database: %w", err)
	}
	log.Println("Notification worker connected to the database.")
	return &postgresRepository{db: db}, nil
}

func (r *postgresRepository) Close() {
	r.db.Close()
}

func (r *postgresRepository) GetUserDeviceTokens(ctx context.Context, userID uuid.UUID) ([]string, error) {
	var tokens []string
	query := `SELECT device_token FROM user_device_tokens WHERE user_id = $1`
	err := r.db.SelectContext(ctx, &tokens, query, userID)
	return tokens, err
}
