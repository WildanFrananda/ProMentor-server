package model

import (
	"time"

	"github.com/google/uuid"
)

type DeviceToken struct {
	ID          uuid.UUID `db:"id"`
	UserID      uuid.UUID `db:"user_id"`
	DeviceToken string    `db:"device_token"`
	CreatedAt   time.Time `db:"created_at"`
}
