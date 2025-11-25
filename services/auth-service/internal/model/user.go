package model

import (
	"time"

	"github.com/google/uuid"
)

type User struct {
	ID           uuid.UUID `db:"id"`
	Email        string    `db:"email"`
	PasswordHash string    `db:"password_hash"`
	Name         string    `db:"name"`
	AvatarURL    *string   `db:"avatar_url"`
	Role         string    `db:"role"`
	CreatedAt    time.Time `db:"created_at"`
	UpdatedAt    time.Time `db:"updated_at"`
}
