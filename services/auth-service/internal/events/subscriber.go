package events

import (
	"auth-service/internal/repository"
	"context"
	"encoding/json"
	"log"
	"time"

	"github.com/google/uuid"
	"github.com/nats-io/nats.go"
)

const (
	maxRetries    = 3
	retryDelaySec = 2
	dlqSubject    = "chat.message.failed"
)

type ChatMessageReceivedEvent struct {
	EventType string    `json:"event_type"`
	SessionID uuid.UUID `json:"session_id"`
	UserID    uuid.UUID `json:"user_id"`
	Content   string    `json:"content"`
}

type ChatSubscriber struct {
	natsConn *nats.Conn
	chatRepo repository.ChatRepository
}

func NewChatSubscriber(natsURL string, chatRepo repository.ChatRepository) (*ChatSubscriber, error) {
	nc, err := nats.Connect(natsURL)
	if err != nil {
		return nil, err
	}
	log.Println("✅ Chat subscriber connected to NATS.")

	subscriber := &ChatSubscriber{
		natsConn: nc,
		chatRepo: chatRepo,
	}

	subscriber.subscribeToChatMessages()

	return subscriber, nil
}

func (s *ChatSubscriber) subscribeToChatMessages() {
	_, err := s.natsConn.Subscribe("chat.message.received.*", func(msg *nats.Msg) {
		var event ChatMessageReceivedEvent
		if err := json.Unmarshal(msg.Data, &event); err != nil {
			log.Printf("❌ Failed to unmarshal chat message event: %v", err)
			return
		}

		log.Printf("📨 Chat event received: User %s in session %s", event.UserID, event.SessionID)

		chatMsg := &repository.ChatMessage{
			SessionID: event.SessionID,
			UserID:    event.UserID,
			Content:   event.Content,
		}

		var saveErr error
		for attempt := 1; attempt <= maxRetries; attempt++ {
			saveErr = s.chatRepo.SaveMessage(context.Background(), chatMsg)
			if saveErr == nil {
				log.Printf("Chat from user %s saved successfully to DB (Retry %d)", event.UserID, attempt)
				return
			}

			log.Printf("Failed saving chat to DB (Retry %d): %v. Retrying in %d seconds...", attempt, saveErr, retryDelaySec)
			time.Sleep(time.Second * retryDelaySec)
		}

		log.Printf("FAILED COMPLETELY to save chat message to DB after %d attempts. Message may be lost. User: %s, Session: %s. Last error: %v", maxRetries, event.UserID, event.SessionID, saveErr)

		err := s.natsConn.Publish(dlqSubject, msg.Data)

		if err != nil {
			log.Printf("Failed to publish to DLQ '%s': %v", dlqSubject, err)
		} else {
			log.Printf("Published failed chat message to DLQ '%s'", dlqSubject)
		}
	})
	if err != nil {
		log.Printf("Failed to subscribe to chat event: %v", err)
	} else {
		log.Println("👂 Chat subscriber listening to event chat.message.received.*")
	}
}
