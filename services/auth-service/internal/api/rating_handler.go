package api

import (
	"auth-service/internal/service"
	"log"

	"github.com/go-playground/validator/v10"
	"github.com/gofiber/fiber/v2"
	"github.com/google/uuid"
)

type RatingHandler struct {
	ratingService service.RatingService
	validate      *validator.Validate
}

func NewRatingHandler(ratingService service.RatingService) *RatingHandler {
	return &RatingHandler{
		ratingService: ratingService,
		validate:      validator.New(),
	}
}

type RateSessionRequest struct {
	Rating  int     `json:"rating" validate:"required,min=1,max=5"`
	Comment *string `json:"comment,omitempty"`
}

func (h *RatingHandler) RateSession(c *fiber.Ctx) error {
	sessionIDStr := c.Params("id")
	sessionID, err := uuid.Parse(sessionIDStr)
	if err != nil {
		return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "Invalid session ID format"})
	}

	userID, err := GetUserIDFromClaims(c)

	if err != nil {
		return c.Status(fiber.StatusUnauthorized).JSON(fiber.Map{"error": "Invalid user claims"})
	}

	var req RateSessionRequest
	if err := c.BodyParser(&req); err != nil {
		return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "Cannot parse JSON"})
	}
	if err := h.validate.Struct(&req); err != nil {
		return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "Invalid input", "details": err.Error()})
	}

	err = h.ratingService.RateSession(c.Context(), sessionID, userID, req.Rating, req.Comment)
	if err != nil {
		switch err {
		case service.ErrUserNotInSession:
			return c.Status(fiber.StatusForbidden).JSON(fiber.Map{"error": err.Error()})
		case service.ErrAlreadyRated:
			return c.Status(fiber.StatusConflict).JSON(fiber.Map{"error": err.Error()})
		case service.ErrSessionNotReadyForRating:
			return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": err.Error()})
		default:
			log.Printf("Error rating session: %v", err)
			return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "Could not rate session"})
		}
	}

	return c.Status(fiber.StatusCreated).JSON(fiber.Map{"message": "Session rated successfully"})
}
