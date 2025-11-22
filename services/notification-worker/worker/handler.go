package worker

import (
	"context"
	"encoding/json"
	"fmt"
	"log"
	"notification-worker/db"
	"os"

	"github.com/google/uuid"
	"github.com/nats-io/nats.go"
	"github.com/sideshow/apns2"
	"github.com/sideshow/apns2/token"
)

type SessionJoinedEvent struct {
	EventType string    `json:"event_type"`
	SessionID uuid.UUID `json:"session_id"`
	UserID    uuid.UUID `json:"user_id"`
}

type Worker struct {
	natsConn   *nats.Conn
	apnsClient *apns2.Client
	repo       db.Repository
}

func (w *Worker) handleSessionJoined(msg *nats.Msg) {
	var event SessionJoinedEvent
	if err := json.Unmarshal(msg.Data, &event); err != nil {
		log.Printf("Error unmarshalling event: %v", err)
		return
	}

	log.Printf(
		"📬 Event Received: User %s joined to session %s.",
		event.UserID,
		event.SessionID,
	)

	tokens, err := w.repo.GetUserDeviceTokens(context.Background(), event.UserID)
	if err != nil {
		log.Printf("Failed to retrieve device tokens for user %s: %v", event.UserID, err)
		return
	}

	if len(tokens) == 0 {
		log.Printf("No device tokens found for user %s. No notifications sent.", event.UserID)
		return
	}

	log.Printf("Found %d device token(s) for user %s. Sending notifications...", len(tokens), event.UserID)
	payload := fmt.Sprintf(`{"aps":{"alert":"You have joined a new session!","sound":"default"}}`)

	for _, token := range tokens {
		notification := &apns2.Notification{
			DeviceToken: token,
			Topic:       os.Getenv("APNS_TOPIC"),
			Payload:     []byte(payload),
		}

		if w.apnsClient == nil {
			log.Printf("✅ SUCCESS (mock): Push notification sent to device %s", token)
		} else {
			// Send real notification
			res, err := w.apnsClient.Push(notification)
			if err != nil {
				log.Printf("❌ FAILED to send notification: %v", err)
			} else if res.Sent() {
				log.Printf("✅ SUCCESS: Notification sent with APNS ID: %s", res.ApnsID)
			} else {
				log.Printf("❌ FAILED: Notification not sent. Reason: %s", res.Reason)
			}
		}
	}
}

func Start(natsURL string) error {
	authKeyPath := os.Getenv("APNS_AUTH_KEY_PATH")
	keyID := os.Getenv("APNS_KEY_ID")
	teamID := os.Getenv("APNS_TEAM_ID")

	var apnsClient *apns2.Client
	if authKeyPath != "" && authKeyPath[0] != '#' && keyID != "" && teamID != "" {
		log.Println("APNs credentials found, initializing APNs client...")
		authKey, err := token.AuthKeyFromFile(authKeyPath)
		if err != nil {
			return fmt.Errorf("Failed to read auth key APNs: %w", err)
		}

		authToken := &token.Token{
			AuthKey: authKey,
			KeyID:   keyID,
			TeamID:  teamID,
		}

		if os.Getenv("APNS_MODE") == "production" {
			apnsClient = apns2.NewTokenClient(authToken).Production()
		} else {
			apnsClient = apns2.NewTokenClient(authToken).Development()
		}
	} else {
		log.Println("APNs credentials not found or invalid. Worker will run in MOCK mode.")
	}

	repo, err := db.NewRepository()
	if err != nil {
		return err
	}
	// defer repo.Close()

	nc, err := nats.Connect(natsURL)
	if err != nil {
		return err
	}

	worker := &Worker{
		natsConn:   nc,
		apnsClient: apnsClient,
		repo:       repo,
	}

	// Subscribe ke subject "session.joined"
	_, err = nc.Subscribe("session.joined", worker.handleSessionJoined)
	if err != nil {
		return err
	}

	return nil
}
