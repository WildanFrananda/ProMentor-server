package handlers

import (
	"encoding/base64"
	"encoding/json"
	"fmt"
	"log"
	"net/http"
	"os"

	"github.com/gofiber/fiber/v2"
)

type UploadURLResponse struct {
	UploadURL     string `json:"upload_url"`
	FinalImageURL string `json:"final_image_url"`
}

func GetAvatarUploadURLWithProxy(userURL string) fiber.Handler {
	internalSecret := os.Getenv("INTERNAL_SHARED_SECRET")

	return func(c *fiber.Ctx) error {
		targetURL := fmt.Sprintf("%s/v1/users/me/avatar/upload-url", userURL)

		req, _ := http.NewRequest("POST", targetURL, nil)

		req.Header.Set("Authorization", c.Get("Authorization"))
		if internalSecret != "" {
			req.Header.Set("X-Internal-Secret", internalSecret)
		}

		client := &http.Client{}
		resp, err := client.Do(req)

		if err != nil {
			return c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{"error": "User service unavailable"})
		}

		defer resp.Body.Close()

		if resp.StatusCode != http.StatusOK {
			return c.Status(resp.StatusCode).SendStream(resp.Body)
		}

		var originalResp UploadURLResponse

		if err := json.NewDecoder(resp.Body).Decode(&originalResp); err != nil {
			return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "Failed to parse upstream response"})
		}

		targetBase64 := base64.StdEncoding.EncodeToString([]byte(originalResp.UploadURL))
		proxyURL := fmt.Sprintf("%s/v1/proxy/upload?target=%s", c.BaseURL(), targetBase64)

		log.Printf("Generated Proxy URL for Client: %s", proxyURL)

		return c.Status(fiber.StatusOK).JSON(fiber.Map{
			"upload_url":      proxyURL,
			"final_image_url": originalResp.FinalImageURL,
		})
	}
}
