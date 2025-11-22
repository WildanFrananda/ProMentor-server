package main

import (
	"bff-service/internal/api"
	"log"
	"os"
	"strconv"
	"time"

	"github.com/gofiber/fiber/v2"
	"github.com/gofiber/fiber/v2/middleware/limiter"
	"github.com/joho/godotenv"
)

func main() {
	godotenv.Load(".env.dev")
	app := fiber.New()

	maxRequest, _ := strconv.Atoi(os.Getenv("RATE_LIMIT_MAX"))
	if maxRequest == 0 {
		maxRequest = 100
	}
	expirationSec, _ := strconv.Atoi(os.Getenv("RATE_LIMIT_EXPIRATION"))
	if expirationSec == 0 {
		expirationSec = 60
	}

	app.Use(limiter.New(limiter.Config{
		Max:        maxRequest,
		Expiration: time.Duration(expirationSec) * time.Second,
		KeyGenerator: func(c *fiber.Ctx) string {
			return c.IP()
		},
		LimitReached: func(c *fiber.Ctx) error {
			return c.Status(fiber.StatusTooManyRequests).JSON(fiber.Map{
				"error": "Too many request, please try again later.",
			})
		},
	}))

	authURL := os.Getenv("AUTH_SERVICE_URL")
	userURL := os.Getenv("USER_SERVICE_URL")

	if authURL == "" || userURL == "" {
		log.Fatal("Service URLs are not properly set in environment variables")
	}

	api.SetupRoutes(app, authURL, userURL)

	port := os.Getenv("APP_PORT")
	if port == "" {
		port = "8000"
	}

	log.Printf("Starting BFF service on port %s", port)
	log.Fatal(app.Listen(":" + port))
}
