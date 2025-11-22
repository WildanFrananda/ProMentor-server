package model

import (
	"time"

	"github.com/google/uuid"
)

type Session struct {
	ID          uuid.UUID  `db:"id" json:"id"`
	CoachID     uuid.UUID  `db:"coach_id" json:"coach_id"`
	Title       string     `db:"title" json:"title"`
	Description string     `db:"description" json:"description"`
	StartAt     time.Time  `db:"start_at" json:"start_at"`
	EndAt       *time.Time `db:"end_at" json:"end_at,omitempty"`
	Capacity    int        `db:"capacity" json:"capacity"`
	CreatedAt   time.Time  `db:"created_at" json:"created_at"`
}

type SessionDetails struct {
	ID          uuid.UUID  `db:"id" json:"id"`
	Title       string     `db:"title" json:"title"`
	Description string     `db:"description" json:"description"`
	StartAt     time.Time  `db:"start_at" json:"start_at"`
	EndAt       *time.Time `db:"end_at" json:"end_at,omitempty"`
	Capacity    int        `db:"capacity" json:"capacity"`
	CoachID     uuid.UUID  `db:"coach_id" json:"coach_id"`
	CoachName   string     `db:"coach_name" json:"coach_name"`
}
