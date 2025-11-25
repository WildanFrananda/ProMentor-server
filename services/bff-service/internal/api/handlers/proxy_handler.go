package handlers

import (
	"bytes"
	"io"
	"log"
	"net/http"
	"os"
	"strings"

	"github.com/gofiber/fiber/v2"
	"go.opentelemetry.io/contrib/instrumentation/net/http/otelhttp"
)

func ProxyTo(baseURL string, targetPath string) fiber.Handler {
	internalSecret := os.Getenv("INTERNAL_SHARED_SECRET")
	otelClient := http.Client{
		Transport: otelhttp.NewTransport(http.DefaultTransport),
	}

	return func(c *fiber.Ctx) error {
		targetURL := baseURL + c.Path()

		for key, value := range c.AllParams() {
			targetURL = strings.Replace(targetURL, ":"+key, value, 1)
		}

		log.Printf("BFF redirect: %s -> %s", c.Path(), targetURL)

		body := c.Body()
		req, err := http.NewRequestWithContext(c.UserContext(), c.Method(), targetURL, bytes.NewReader(body))
		if err != nil {
			return err
		}

		for key, value := range c.Request().Header.All() {
			req.Header.Set(string(key), string(value))
		}

		req.ContentLength = int64(len(body))

		req.Host = baseURL

		if internalSecret != "" {
			req.Header.Set("X-Internal-Secret", internalSecret)
		} else {
			panic("Internal secret is not set in BFF service!")
		}

		resp, err := otelClient.Do(req)

		if err != nil {
			log.Printf("BFF Error calling %s: %v", targetURL, err)
			return c.Status(fiber.StatusServiceUnavailable).JSON(fiber.Map{
				"error": "Internal service unavailable",
			})
		}

		defer resp.Body.Close()

		c.Status(resp.StatusCode)
		for key, values := range resp.Header {
			for _, value := range values {
				c.Set(key, value)
			}
		}

		bodyBytes, _ := io.ReadAll(resp.Body)
		c.Context().SetBody(bodyBytes)

		return nil
	}
}
