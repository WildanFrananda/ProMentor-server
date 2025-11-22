package repository

import (
	"context"
	"time"

	"github.com/google/uuid"
	"github.com/jmoiron/sqlx"
)

type ChatMessage struct {
	SessionID uuid.UUID `db:"session_id"`
	UserID    uuid.UUID `db:"user_id"`
	Content   string    `db:"content"`
	CreatedAt time.Time `db:"created_at"`
}

type ChatRepository interface {
	SaveMessage(ctx context.Context, msg *ChatMessage) error
}

type postgresChatRepository struct {
	db *sqlx.DB
}

func NewPostgresChatRepository(db *sqlx.DB) ChatRepository {
	return &postgresChatRepository{db: db}
}

func (r *postgresChatRepository) SaveMessage(ctx context.Context, msg *ChatMessage) error {
	query := `INSERT INTO chat_messages (session_id, user_id, content) VALUES ($1, $2, $3)`
	_, err := r.db.ExecContext(ctx, query, msg.SessionID, msg.UserID, msg.Content)
	return err
}
