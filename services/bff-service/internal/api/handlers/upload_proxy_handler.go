package handlers

import (
	"encoding/base64"
	"io"
	"log"
	"net/http"

	"github.com/gofiber/fiber/v2"
)

func HandleUploadProxy(c *fiber.Ctx) error {
	targetEncoded := c.Query("target")

	if targetEncoded == "" {
		return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "Missing target parameter"})
	}

	targetURLBytes, err := base64.StdEncoding.DecodeString(targetEncoded)

	if err != nil {
		return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "Invalid target parameter"})
	}

	targetURL := string(targetURLBytes)

	log.Printf("🔄 BFF Proxy Uploading to: %s", targetURL)

	proxyReq, err := http.NewRequest(c.Method(), targetURL, c.Context().RequestBodyStream())

	if err != nil {
		return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "Failed to create proxy request"})
	}

	proxyReq.ContentLength = int64(c.Request().Header.ContentLength())
	proxyReq.Header.Set("Content-Type", c.Get("Content-Type"))

	client := &http.Client{}
	resp, err := client.Do(proxyReq)

	if err != nil {
		log.Printf("Error proxying upload to MinIO: %v", err)
		return c.Status(fiber.StatusBadGateway).JSON(fiber.Map{"error": "Upstream storage unavailable"})
	}

	defer resp.Body.Close()

	if resp.StatusCode >= 400 {
		body, _ := io.ReadAll(resp.Body)
		log.Printf("MinIO Error (%d): %s", resp.StatusCode, string(body))
	}

	return c.SendStatus(resp.StatusCode)
}
