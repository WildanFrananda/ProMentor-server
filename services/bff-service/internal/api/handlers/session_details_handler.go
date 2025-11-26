package handlers

import (
	"bff-service/internal/model"
	"bff-service/internal/utils"
	"encoding/json"
	"fmt"
	"log"
	"net/http"
	"os"
	"sync"

	"github.com/gofiber/fiber/v2"
	"github.com/google/uuid"
)

func GetSessionDetails(authURL string, userURL string) fiber.Handler {
	internalSecret := os.Getenv("INTERNAL_SHARED_SECRET")

	return func(c *fiber.Ctx) error {
		sessionID := c.Params("id")
		if _, err := uuid.Parse(sessionID); err != nil {
			return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "Invalid session ID format"})
		}

		sessionChan := make(chan *model.SessionData)
		coachChan := make(chan *model.CoachProfileData)
		errChan := make(chan error, 2)

		var wg sync.WaitGroup
		wg.Add(1)

		go func() {
			defer wg.Done()
			sessionURL := fmt.Sprintf("%s/v1/sessions/%s", authURL, sessionID)
			client := &http.Client{}
			req, _ := http.NewRequest("GET", sessionURL, nil)
			if internalSecret != "" {
				req.Header.Add("X-Internal-Secret", internalSecret)
			}
			resp, err := client.Do(req)
			if err != nil {
				log.Printf("Error calling auth-service: %v", err)
				errChan <- fmt.Errorf("auth-service unavailable")
				sessionChan <- nil
				return
			}
			defer resp.Body.Close()

			if resp.StatusCode == http.StatusNotFound {
				errChan <- fmt.Errorf("session not found")
				sessionChan <- nil
				return
			}
			if resp.StatusCode != http.StatusOK {
				errChan <- fmt.Errorf("auth-service error: status %d", resp.StatusCode)
				sessionChan <- nil
				return
			}

			var sessionData model.SessionData
			if err := json.NewDecoder(resp.Body).Decode(&sessionData); err != nil {
				log.Printf("Error decoding session data: %v", err)
				errChan <- fmt.Errorf("failed to decode session data")
				sessionChan <- nil
				return
			}
			sessionChan <- &sessionData
		}()

		sessionData := <-sessionChan

		if sessionData == nil {
			err := <-errChan
			if err.Error() == "session not found" {
				return c.Status(fiber.StatusNotFound).JSON(fiber.Map{"error": err.Error()})
			}
			return c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{"error": err.Error()})
		}

		wg.Add(1)
		go func(coachID uuid.UUID) {
			defer wg.Done()
			coachURL := fmt.Sprintf("%s/v1/users/%s", userURL, coachID.String())
			client := &http.Client{}
			req, _ := http.NewRequest("GET", coachURL, nil)
			if internalSecret != "" {
				req.Header.Add("X-Internal-Secret", internalSecret)
			}
			resp, err := client.Do(req)
			if err != nil {
				log.Printf("Error calling user-service: %v", err)
				errChan <- fmt.Errorf("user-service unavailable")
				coachChan <- nil
				return
			}
			defer resp.Body.Close()

			if resp.StatusCode == http.StatusNotFound {
				coachChan <- &model.CoachProfileData{ID: coachID}
				return
			}
			if resp.StatusCode != http.StatusOK {
				errChan <- fmt.Errorf("user-service error: status %d", resp.StatusCode)
				coachChan <- nil
				return
			}

			var coachData model.CoachProfileData
			if err := json.NewDecoder(resp.Body).Decode(&coachData); err != nil {
				log.Printf("Error decoding coach data: %v", err)
				errChan <- fmt.Errorf("failed to decode coach data")
				coachChan <- nil
				return
			}
			coachChan <- &coachData
		}(sessionData.CoachID)

		coachData := <-coachChan

		select {
		case err := <-errChan:
			if coachData == nil {
				return c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{"error": err.Error()})
			}
			log.Printf("Non-critical error during coach fetch: %v", err)
		default:
		}

		if coachData == nil {
			coachData = &model.CoachProfileData{ID: sessionData.CoachID}
		}

		if coachData.AvatarURL != nil {
			newURL := utils.RewriteAvatarURL(*coachData.AvatarURL, c.BaseURL())
			coachData.AvatarURL = &newURL
		}

		response := model.SessionDetailsResponse{
			ID:          sessionData.ID,
			Title:       sessionData.Title,
			Description: sessionData.Description,
			StartAt:     sessionData.StartAt,
			Capacity:    sessionData.Capacity,
			Coach:       *coachData,
			CreatedAt:   sessionData.CreatedAt,
		}

		return c.Status(fiber.StatusOK).JSON(response)
	}
}
