package handlers

import (
	"bff-service/internal/utils"
	"encoding/json"
	"fmt"
	"log"
	"net/http"
	"os"

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

func HandleGetMyProfile(userURL string) fiber.Handler {
	internalSecret := os.Getenv("INTERNAL_SHARED_SECRET")

	return func(c *fiber.Ctx) error {
		targetURL := fmt.Sprintf("%s/v1/users/me", userURL)
		req, _ := http.NewRequest("GET", targetURL, nil)

		req.Header.Set("Authorization", c.Get("Authorization"))

		if internalSecret != "" {
			req.Header.Add("X-Internal-Secret", internalSecret)
		}

		client := &http.Client{}
		resp, err := client.Do(req)

		if err != nil {
			log.Printf("Error calling user service at %s: %v", targetURL, err)
			return c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{"error": "User service unavailable"})
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

		req, _ := http.NewRequest("PUT", targetURL, c.Context().RequestBodyStream())
		req.Header.Set("Content-Type", "application/json")
		req.Header.Set("Authorization", c.Get("Authorization"))
		if internalSecret != "" {
			req.Header.Add("X-Internal-Secret", internalSecret)
		}

		client := &http.Client{}
		resp, err := client.Do(req)
		if err != nil {
			log.Printf("Error calling user service at %s: %v", targetURL, err)
			return c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{"error": "User service unavailable"})
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
