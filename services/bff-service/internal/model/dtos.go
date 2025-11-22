package model

import (
	"time"

	"github.com/google/uuid"
)

type SessionData struct {
	ID          uuid.UUID  `json:"id"`
	CoachID     uuid.UUID  `json:"coach_id"`
	Title       string     `json:"title"`
	Description string     `json:"description"`
	StartAt     time.Time  `json:"start_at"`
	EndAt       *time.Time `json:"end_at"`
	Capacity    int        `json:"capacity"`
	CreatedAt   time.Time  `json:"created_at"`
}

type CoachProfileData struct {
	ID        uuid.UUID `json:"id"`
	Name      *string   `json:"name"`
	AvatarURL *string   `json:"avatar_url"`
}

type SessionDetailsResponse struct {
	ID          uuid.UUID        `json:"id"`
	Title       string           `json:"title"`
	Description string           `json:"description"`
	StartAt     time.Time        `json:"start_at"`
	EndAt       *time.Time       `json:"end_at,omitempty"`
	Capacity    int              `json:"capacity"`
	Coach       CoachProfileData `json:"coach"`
	CreatedAt   time.Time        `json:"created_at"`
}
