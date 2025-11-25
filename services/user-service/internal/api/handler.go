package api

import (
	"log"
	"net/url"
	"os"
	"user-service/internal/s3"
	"user-service/internal/service"

	"github.com/go-playground/validator/v10"
	"github.com/gofiber/fiber/v2"
	"github.com/google/uuid"
)

type UserHandler struct {
	userService   service.UserService
	validate      *validator.Validate
	filePresigner *s3.FilePresigner
}

func NewUserHandler(userService service.UserService, presigner *s3.FilePresigner) *UserHandler {
	return &UserHandler{
		userService:   userService,
		validate:      validator.New(),
		filePresigner: presigner,
	}
}

type UpdateUserProfileRequest struct {
	Name      *string `json:"name,omitempty" validate:"omitempty,min=2"`
	AvatarURL *string `json:"avatar_url,omitempty" validate:"omitempty,url"`
}

type RegisterTokenRequest struct {
	DeviceToken string `json:"device_token" validate:"required"`
}

type UserProfileResponse struct {
	ID        uuid.UUID `json:"id"`
	Name      *string   `json:"name,omitempty"`
	Email     string    `json:"email"`
	Role      string    `json:"role"`
	AvatarURL string    `json:"avatar_url"`
}

func (h *UserHandler) UpdateUserProfile(c *fiber.Ctx) error {
	userIDStr := c.Locals("userID").(string)
	userID, err := uuid.Parse(userIDStr)
	if err != nil {
		return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "Invalid user ID format"})
	}

	var req UpdateUserProfileRequest
	if err := c.BodyParser(&req); err != nil {
		return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "Cannot parse JSON"})
	}

	if err := h.validate.Struct(&req); err != nil {
		return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "Invalid input", "details": err.Error()})
	}

	updatedUser, err := h.userService.UpdateUserProfile(c.Context(), userID, service.UpdateUserDTO{
		Name:      req.Name,
		AvatarURL: req.AvatarURL,
	})
	if err != nil {
		return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "Could not update user profile"})
	}

	return c.Status(fiber.StatusOK).JSON(updatedUser)
}

func (h *UserHandler) GetAvatarUploadURL(c *fiber.Ctx) error {
	userIDStr := c.Locals("userID").(string)
	objectKey := "user-avatars/" + userIDStr + "/" + uuid.New().String() + ".jpg"

	uploadURL, err := h.filePresigner.GeneratePresignedUploadURL(objectKey)
	if err != nil {
		return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "Could not generate upload URL"})
	}

	finalImageURL := os.Getenv("S3_ENDPOINT") + "/" + h.filePresigner.BucketName + "/" + objectKey

	return c.JSON(fiber.Map{
		"upload_url":      uploadURL,
		"final_image_url": finalImageURL,
	})
}

func (h *UserHandler) RegisterDeviceToken(c *fiber.Ctx) error {
	userIDStr := c.Locals("userID").(string)
	userID, _ := uuid.Parse(userIDStr)

	var req RegisterTokenRequest
	if err := c.BodyParser(&req); err != nil {
		return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "Cannot parse JSON"})
	}
	if err := h.validate.Struct(&req); err != nil {
		return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "Invalid input", "details": err.Error()})
	}

	err := h.userService.RegisterDeviceToken(c.Context(), userID, req.DeviceToken)
	if err != nil {
		return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "Could not register device token"})
	}

	return c.Status(fiber.StatusOK).JSON(fiber.Map{"message": "Device token registered successfully"})
}

func (h *UserHandler) GetUserProfileByID(c *fiber.Ctx) error {
	userIDStr := c.Params("id")
	userID, err := uuid.Parse(userIDStr)
	if err != nil {
		return c.Status(fiber.StatusBadRequest).JSON(fiber.Map{"error": "Invalid user ID format"})
	}

	user, err := h.userService.GetUserProfileByID(c.Context(), userID)
	if err != nil {
		if err.Error() == "user not found" {
			return c.Status(fiber.StatusNotFound).JSON(fiber.Map{"error": err.Error()})
		}
		log.Printf("Error getting user profile by ID: %v", err)
		return c.Status(fiber.StatusInternalServerError).JSON(fiber.Map{"error": "Could not fetch user profile"})
	}
	finalAvatar := ""
	if user.AvatarURL != nil {
		finalAvatar = *user.AvatarURL
	} else {
		name := ""
		if user.Name != nil && *user.Name != "" {
			name = url.QueryEscape(*user.Name)
		}
		finalAvatar = "https://ui-avatars.com/api/?name=" + name
	}

	response := UserProfileResponse{
		ID:        user.ID,
		Name:      user.Name,
		Email:     user.Email,
		Role:      user.Role,
		AvatarURL: finalAvatar,
	}

	return c.Status(fiber.StatusOK).JSON(response)
}
