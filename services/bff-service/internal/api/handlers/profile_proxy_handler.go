package handlers

import (
	"bff-service/internal/utils"
	"bytes"
	"encoding/json"
	"fmt"
	"log"
	"net/http"
	"os"
	"time"

	"github.com/gofiber/fiber/v2"
	"github.com/google/uuid"
)

type UserProfileResponse struct {
	ID        uuid.UUID `json:"id"`
	Name      *string   `json:"name,omitempty"`
	Email     string    `json:"email"`
	Role      string    `json:"role"`
	AvatarURL string    `json:"avatar_url"`
}

// Use a shared client for performance and connection pooling
var proxyClient = &http.Client{
	Timeout: 10 * time.Second,
}

func HandleGetMyProfile(userURL string) fiber.Handler {
	internalSecret := os.Getenv("INTERNAL_SHARED_SECRET")

	return func(c *fiber.Ctx) error {
		targetURL := fmt.Sprintf("%s/v1/users/me", userURL)
		req, err := http.NewRequest("GET", targetURL, nil)
		if err != nil {
			log.Printf("Error creating request for %s: %v", targetURL, err)
			return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "Failed to create request"})
		}

		req.Header.Set("Authorization", c.Get("Authorization"))

		if internalSecret != "" {
			req.Header.Add("X-Internal-Secret", internalSecret)
		}

		resp, err := proxyClient.Do(req)
		if err != nil {
			log.Printf("Error calling service at %s: %v", targetURL, err)
			return c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{"error": "Service unavailable"})
		}
		defer resp.Body.Close()

		if resp.StatusCode != http.StatusOK {
			return c.Status(resp.StatusCode).SendStream(resp.Body)
		}

		var profile UserProfileResponse
		if err := json.NewDecoder(resp.Body).Decode(&profile); err != nil {
			return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "Failed to parse profile data"})
		}

		profile.AvatarURL = utils.RewriteAvatarURL(profile.AvatarURL, c.BaseURL())

		return c.Status(fiber.StatusOK).JSON(profile)
	}
}

func HandleUpdateMyProfile(userURL string) fiber.Handler {
	internalSecret := os.Getenv("INTERNAL_SHARED_SECRET")

	return func(c *fiber.Ctx) error {
		targetURL := fmt.Sprintf("%s/v1/users/me", userURL)

		// Read the body into a buffer to avoid streaming deadlocks
		bodyBytes := c.Body()
		req, err := http.NewRequest("PUT", targetURL, bytes.NewReader(bodyBytes))
		if err != nil {
			log.Printf("Error creating request for %s: %v", targetURL, err)
			return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "Failed to create request"})
		}
		
		// Forward necessary headers
		req.Header.Set("Content-Type", c.Get("Content-Type"))
		req.Header.Set("Authorization", c.Get("Authorization"))
		req.ContentLength = int64(len(bodyBytes)) // Set content length since we're using a buffer
		if internalSecret != "" {
			req.Header.Add("X-Internal-Secret", internalSecret)
		}

		resp, err := proxyClient.Do(req)
		if err != nil {
			log.Printf("Error calling service at %s: %v", targetURL, err)
			return c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{"error": "Service unavailable"})
		}
		defer resp.Body.Close()

		if resp.StatusCode != http.StatusOK {
			return c.Status(resp.StatusCode).SendStream(resp.Body)
		}

		var profile UserProfileResponse
		if err := json.NewDecoder(resp.Body).Decode(&profile); err != nil {
			return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "Failed to parse profile data"})
		}

		profile.AvatarURL = utils.RewriteAvatarURL(profile.AvatarURL, c.BaseURL())

		return c.Status(fiber.StatusOK).JSON(profile)
	}
}
