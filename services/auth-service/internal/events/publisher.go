package events

import (
	"auth-service/internal/model"
	"encoding/json"
	"log"
	"time"

	"github.com/google/uuid"
	"github.com/nats-io/nats.go"
)

type EventPublisher interface {
	PublishSessionCreated(session *model.Session) error
	PublishSessionJoined(sessionID, userID uuid.UUID) error
}

type NatsPublisher struct {
	conn *nats.Conn
}

func NewNatsPublisher(natsURL string) (EventPublisher, error) {
	nc, err := nats.Connect(natsURL)

	if err != nil {
		return nil, err
	}

	return &NatsPublisher{conn: nc}, nil
}

type SessionCreatedEvent struct {
	EventType string    `json:"event_type"`
	SessionID uuid.UUID `json:"session_id"`
	CoachID   uuid.UUID `json:"coach_id"`
	Title     string    `json:"title"`
	StartAt   time.Time `json:"start_at"`
}

type SessionJoinedEvent struct {
	EventType string    `json:"event_type"`
	SessionID uuid.UUID `json:"session_id"`
	UserID    uuid.UUID `json:"user_id"`
	JoinedAt  time.Time `json:"joined_at"`
}

func (p *NatsPublisher) PublishSessionCreated(session *model.Session) error {
	event := SessionCreatedEvent{
		EventType: "session.created",
		SessionID: session.ID,
		CoachID:   session.CoachID,
		Title:     session.Title,
		StartAt:   session.StartAt,
	}

	eventJSON, err := json.Marshal(event)

	if err != nil {
		log.Printf("Error marshalling event JSON: %v", err)
		return err
	}

	subject := "session.created"
	err = p.conn.Publish(subject, eventJSON)

	if err != nil {
		log.Printf("Error publishing to NATS: %v", err)
		return err
	}

	log.Printf("Published event to NATS on subject '%s'", subject)

	return nil
}

func (p *NatsPublisher) PublishSessionJoined(sessionID, userID uuid.UUID) error {
	event := SessionJoinedEvent{
		EventType: "session.joined",
		SessionID: sessionID,
		UserID:    userID,
		JoinedAt:  time.Now(),
	}

	eventJSON, err := json.Marshal(event)

	if err != nil {
		return err
	}

	subject := "session.joined"
	err = p.conn.Publish(subject, eventJSON)

	if err != nil {
		log.Printf("Error publishing to NATS: %v", err)

		return err
	}

	log.Printf("Published event to NATS on subject '%s' for user '%s'", subject, userID)

	return nil
}
