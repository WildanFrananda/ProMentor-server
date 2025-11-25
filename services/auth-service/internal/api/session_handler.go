package api

import (
	"auth-service/internal/model"
	"auth-service/internal/service"
	"database/sql"
	"errors"
	"log/slog"
	"strconv"
	"time"

	"github.com/go-playground/validator/v10"
	"github.com/gofiber/fiber/v2"
	"github.com/google/uuid"
)

type SessionHandler struct {
	sessionService service.SessionService
	validate       *validator.Validate
}

func NewSessionHandler(sessionService service.SessionService) *SessionHandler {
	return &SessionHandler{
		sessionService: sessionService,
		validate:       validator.New(),
	}
}

type CreateSessionRequest struct {
	Title       string    `json:"title" validate:"required,min=5,max=100"`
	Description string    `json:"description,omitempty" validate:"max=500"`
	StartAt     time.Time `json:"start_at" validate:"required"`
	Capacity    int       `json:"capacity" validate:"required,min=1"`
}

func (h *SessionHandler) CreateSession(c *fiber.Ctx) error {
	userID, err := GetUserIDFromClaims(c)

	if err != nil {
		slog.ErrorContext(c.UserContext(), "Error getting user ID from claims", slog.String("error", err.Error()))
		return c.Status(fiber.StatusUnauthorized).JSON(fiber.Map{"error": "Invalid user claims"})
	}

	coachID := userID

	var request CreateSessionRequest

	if err := c.BodyParser(&request); err != nil {
		return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "Cannot parse JSON"})
	}

	if err := h.validate.Struct(&request); err != nil {
		return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "Invalid input", "details": err.Error()})
	}

	session := &model.Session{
		CoachID:     coachID,
		Title:       request.Title,
		Description: request.Description,
		StartAt:     request.StartAt,
		Capacity:    request.Capacity,
	}

	createdSession, err := h.sessionService.CreateSession(c.Context(), session)

	if err != nil {
		return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "Could not create session"})
	}

	return c.Status(fiber.StatusCreated).JSON(createdSession)
}

func (h *SessionHandler) JoinSession(c *fiber.Ctx) error {
	sessionIDStr := c.Params("id")
	sessionID, err := uuid.Parse(sessionIDStr)

	if err != nil {
		return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "Invalid session ID format"})
	}

	userID, err := GetUserIDFromClaims(c)

	if err != nil {
		return c.Status(fiber.StatusUnauthorized).JSON(fiber.Map{"error": "Invalid user claims"})
	}

	err = h.sessionService.JoinSession(c.Context(), sessionID, userID)

	if err != nil {
		switch err {
		case service.ErrSessionNotFound:
			return c.Status(fiber.StatusNotFound).JSON(fiber.Map{"error": err.Error()})
		case service.ErrAlreadyJoined:
			return c.Status(fiber.StatusConflict).JSON(fiber.Map{"error": err.Error()})
		case service.ErrSessionFull:
			return c.Status(fiber.StatusConflict).JSON(fiber.Map{"error": err.Error()})
		default:
			return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": err.Error()})
		}
	}

	return c.Status(fiber.StatusOK).JSON(fiber.Map{"message": "Joined session successfully"})
}

func (h *SessionHandler) ListUpcomingSessions(c *fiber.Ctx) error {
	page, _ := strconv.Atoi(c.Query("page", "1"))
	limit, _ := strconv.Atoi(c.Query("limit", "10"))
	query := c.Query("query")

	sessions, err := h.sessionService.ListUpcomingSessions(c.Context(), page, limit, query)
	if err != nil {
		return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "Could not fetch upcoming sessions"})
	}

	return c.Status(fiber.StatusOK).JSON(sessions)
}

func (h *SessionHandler) ListHistory(c *fiber.Ctx) error {
	userID, err := GetUserIDFromClaims(c)

	if err != nil {
		return c.Status(fiber.StatusUnauthorized).JSON(fiber.Map{"error": "Invalid user claims"})
	}

	history, err := h.sessionService.ListUserHistory(c.Context(), userID)
	if err != nil {
		return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "Could not fetch session history"})
	}

	return c.Status(fiber.StatusOK).JSON(history)
}

func (h *SessionHandler) GetSessionDetails(c *fiber.Ctx) error {
	sessionIDStr := c.Params("id")
	sessionID, err := uuid.Parse(sessionIDStr)
	if err != nil {
		return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "Invalid session ID format"})
	}

	session, err := h.sessionService.GetSessionDetails(c.Context(), sessionID)
	if err != nil {
		if errors.Is(err, service.ErrSessionNotFound) || errors.Is(err, sql.ErrNoRows) {
			return c.Status(fiber.StatusNotFound).JSON(fiber.Map{"error": "Session not found"})
		}
		slog.ErrorContext(c.UserContext(), "Error getting session details", slog.String("error", err.Error()))
		return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "Could not fetch session details"})
	}

	return c.Status(fiber.StatusOK).JSON(session)
}
