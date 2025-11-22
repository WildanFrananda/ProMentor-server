package events_test

import (
	"encoding/json"
	"testing"
	"time"

	"auth-service/internal/events"
	"auth-service/internal/model"

	"github.com/google/uuid"
	"github.com/stretchr/testify/require"
)

func TestSessionCreatedEvent_Marshal(t *testing.T) {
	s := &model.Session{ID: uuid.New(), CoachID: uuid.New(), Title: "C", StartAt: time.Now()}
	ev := events.SessionCreatedEvent{
		EventType: "session.created",
		SessionID: s.ID,
		CoachID:   s.CoachID,
		Title:     s.Title,
		StartAt:   s.StartAt,
	}

	b, err := json.Marshal(ev)
	require.NoError(t, err)
	var decoded map[string]interface{}
	require.NoError(t, json.Unmarshal(b, &decoded))
	require.Equal(t, "session.created", decoded["event_type"])
}

func TestSessionJoinedEvent_Marshal(t *testing.T) {
	sid := uuid.New()
	uid := uuid.New()
	ev := events.SessionJoinedEvent{
		EventType: "session.joined",
		SessionID: sid,
		UserID:    uid,
		JoinedAt:  time.Now(),
	}

	b, err := json.Marshal(ev)
	require.NoError(t, err)
	var decoded map[string]interface{}
	require.NoError(t, json.Unmarshal(b, &decoded))
	require.Equal(t, "session.joined", decoded["event_type"])
}
