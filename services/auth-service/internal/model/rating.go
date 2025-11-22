package model

import (
	"time"

	"github.com/google/uuid"
)

type Rating struct {
	ID        uuid.UUID `db:"id"`
	SessionID uuid.UUID `db:"session_id"`
	UserID    uuid.UUID `db:"user_id"`
	Rating    int       `db:"rating"`
	Comment   *string   `db:"comment"`
	CreatedAt time.Time `db:"created_at"`
}
