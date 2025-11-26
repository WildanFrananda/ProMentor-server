package api

import (
	"auth-service/internal/jwt"
	"errors"
	"fmt"
	"os"
	"strings"
	"time"

	"github.com/gofiber/fiber/v2"
	jwtv5 "github.com/golang-jwt/jwt/v5"
	"github.com/google/uuid"
	"github.com/prometheus/client_golang/prometheus"
	"github.com/prometheus/client_golang/prometheus/promauto"
)

var (
	httpRequestTotal = promauto.NewCounterVec(
		prometheus.CounterOpts{
			Name: "http_requests_total",
			Help: "Total number of HTTP requests",
		},
		[]string{"method", "path", "status_code"},
	)
	httpRequestDuration = promauto.NewHistogramVec(
		prometheus.HistogramOpts{
			Name:    "http_request_duration_seconds",
			Help:    "Duration of http request",
			Buckets: prometheus.DefBuckets,
		},
		[]string{"method", "path", "status_code"},
	)
)

func AuthMiddleware() fiber.Handler {
	return func(c *fiber.Ctx) error {
		authHeader := c.Get("Authorization")
		if authHeader == "" {
			return c.Status(fiber.StatusUnauthorized).JSON(fiber.Map{"error": "Missing authorization header"})
		}

		parts := strings.Split(authHeader, " ")
		if len(parts) != 2 || parts[0] != "Bearer" {
			return c.Status(fiber.StatusUnauthorized).JSON(fiber.Map{"error": "Invalid authorization header format"})
		}
		tokenString := parts[1]

		claims, err := jwt.ValidateToken(tokenString)
		if err != nil {
			if errors.Is(err, jwtv5.ErrTokenExpired) {
				return c.Status(fiber.StatusUnauthorized).JSON(fiber.Map{"error": "Token has expired"})
			}
			return c.Status(fiber.StatusUnauthorized).JSON(fiber.Map{"error": "Invalid token"})
		}

		userIDStr, ok := claims["sub"].(string)
		if !ok {
			return c.Status(fiber.StatusUnauthorized).JSON(fiber.Map{"error": "User ID not found in token claims"})
		}

		_, err = uuid.Parse(userIDStr)
		if err != nil {
			return c.Status(fiber.StatusUnauthorized).JSON(fiber.Map{"error": "Invalid user ID format in token"})
		}

		c.Locals("userClaims", claims)

		return c.Next()
	}
}

func GetUserIDFromClaims(c *fiber.Ctx) (uuid.UUID, error) {
	claims, ok := c.Locals("userClaims").(jwtv5.MapClaims)
	if !ok {
		return uuid.Nil, errors.New("claims not found in context")
	}

	userIDStr, ok := claims["sub"].(string)
	if !ok {
		return uuid.Nil, errors.New("userID not found in claims")
	}

	userID, err := uuid.Parse(userIDStr)
	if err != nil {
		return uuid.Nil, fmt.Errorf("invalid userID format in claims: %w", err)
	}

	return userID, nil
}

func InternalAuthMiddleware() fiber.Handler {
	expectedSecret := os.Getenv("INTERNAL_SHARED_SECRET")
	if expectedSecret == "" {
		panic("INTERNAL_SHARED_SECRET environment variable is not set")
	}

	return func(c *fiber.Ctx) error {
		secret := c.Get("X-Internal-Secret")

		if secret != expectedSecret {
			return c.Status(fiber.StatusForbidden).JSON(fiber.Map{"error": "Internal access denied!"})
		}

		return c.Next()
	}
}

func PrometheusMiddleware() fiber.Handler {
	return func(c *fiber.Ctx) error {
		start := time.Now()
		err := c.Next()
		duration := time.Since(start).Seconds()
		statusCode := c.Response().StatusCode()

		if err != nil {
			var e *fiber.Error

			if errors.As(err, &e) {
				statusCode = e.Code
			} else {
				statusCode = fiber.StatusInternalServerError
			}
		}

		method := c.Method()
		path := c.Path()
		statusStr := fmt.Sprintf("%d", statusCode)

		httpRequestTotal.WithLabelValues(method, path, statusStr).Inc()
		httpRequestDuration.WithLabelValues(method, path, statusStr).Observe(duration)

		return err
	}
}
