package handlers

import (
	"fmt"
	"io"
	"log"
	"net/http"

	"github.com/gofiber/fiber/v2"
)

func HandleDownloadProxy(c *fiber.Ctx) error {
	objectKey := c.Query("key")

	if objectKey == "" {
		return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "Missing key parameter"})
	}

	internalMinioURL := fmt.Sprintf("http://minio:9000/" + objectKey)

	log.Printf("📥 BFF Proxy Downloading from: %s", internalMinioURL)

	resp, err := http.Get(internalMinioURL)

	if err != nil {
		log.Printf("Error fetching from MinIO: %v", err)
		return c.Status(fiber.StatusBadGateway).JSON(fiber.Map{"error": "Storage service unavailable"})
	}

	defer resp.Body.Close()

	if resp.StatusCode == http.StatusNotFound {
		return c.Status(fiber.StatusNotFound).JSON(fiber.Map{"error": "Image not found"})
	}
	if resp.StatusCode != http.StatusOK {
		return c.Status(fiber.StatusBadGateway).JSON(fiber.Map{"error": "Failed to retrieve image"})
	}

	c.Set("Content-Type", resp.Header.Get("Content-Type"))
	c.Set("Content-Length", resp.Header.Get("Content-Length"))
	c.Status(http.StatusOK)

	if _, err := io.Copy(c.Response().BodyWriter(), resp.Body); err != nil {
		log.Printf("Error streaming response: %v", err)
	}

	return nil
}
