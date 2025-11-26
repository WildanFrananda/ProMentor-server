package service

import (
	"context"
	"crypto/sha256"
	"encoding/hex"
	"errors"
	"fmt"
	"time"

	"auth-service/internal/jwt"
	"auth-service/internal/model"
	"auth-service/internal/repository"

	"github.com/google/uuid"
	"golang.org/x/crypto/bcrypt"
)

var (
	ErrInvalidCredentials = errors.New("invalid email or password")
	ErrTokenInvalid       = errors.New("token is invalid or expired")
)

type AuthService interface {
	RegisterUser(ctx context.Context, email, password, name string) (*model.User, error)
	LoginUser(ctx context.Context, email, password string) (accessToken string, refreshToken string, err error)
	GetUserProfile(ctx context.Context, userID uuid.UUID) (*model.User, error)
	RefreshToken(ctx context.Context, refreshTokenString string) (newAccessToken string, err error)
	LogoutUser(ctx context.Context, refreshTokenString string) error
}

type authService struct {
	userRepo  repository.UserRepository
	tokenRepo repository.TokenRepository
}

func NewAuthService(userRepo repository.UserRepository, tokenRepo repository.TokenRepository) AuthService {
	return &authService{
		userRepo:  userRepo,
		tokenRepo: tokenRepo,
	}
}

func (s *authService) RegisterUser(ctx context.Context, email, password, name string) (*model.User, error) {
	hashedPassword, err := bcrypt.GenerateFromPassword([]byte(password), bcrypt.DefaultCost)

	if err != nil {
		return nil, err
	}

	user := &model.User{
		Email:        email,
		PasswordHash: string(hashedPassword),
		Name:         name,
		Role:         "attendee",
	}

	newID, err := s.userRepo.Create(ctx, user)
	if err != nil {
		return nil, err
	}

	user.ID = newID

	return user, nil
}

func (s *authService) LoginUser(ctx context.Context, email, password string) (string, string, error) {
	user, err := s.userRepo.FindByEmail(ctx, email)
	if err != nil {
		return "", "", ErrInvalidCredentials
	}

	if err := bcrypt.CompareHashAndPassword([]byte(user.PasswordHash), []byte(password)); err != nil {
		return "", "", ErrInvalidCredentials
	}

	accessToken, refreshToken, err := jwt.GenerateTokens(user)
	if err != nil {
		return "", "", err
	}

	hash := sha256.Sum256([]byte(refreshToken))
	tokenHash := hex.EncodeToString(hash[:])

	refreshTokenModel := &model.RefreshToken{
		UserID:    user.ID,
		TokenHash: tokenHash,
		ExpiresAt: time.Now().Add(time.Hour * 24 * 30),
	}

	if err := s.tokenRepo.Create(ctx, refreshTokenModel); err != nil {
		return "", "", err
	}

	return accessToken, refreshToken, nil
}

func (s *authService) GetUserProfile(ctx context.Context, userID uuid.UUID) (*model.User, error) {
	user, err := s.userRepo.FindByID(ctx, userID)
	if err != nil {
		return nil, err
	}
	fmt.Println("Debug: Auth Service GetUserProfile: ", user)
	return user, nil
}

func (s *authService) RefreshToken(ctx context.Context, refreshTokenString string) (string, error) {
	claims, err := jwt.ValidateToken(refreshTokenString)

	if err != nil {
		return "", ErrTokenInvalid
	}

	hash := sha256.Sum256([]byte(refreshTokenString))
	tokenHash := hex.EncodeToString(hash[:])

	_, err = s.tokenRepo.FindByTokenHash(ctx, tokenHash)
	if err != nil {
		return "", ErrTokenInvalid
	}

	userID, _ := uuid.Parse(claims["sub"].(string))
	user, err := s.userRepo.FindByID(ctx, userID)

	if err != nil {
		return "", ErrTokenInvalid
	}

	newAccessToken, _, err := jwt.GenerateTokens(user)

	if err != nil {
		return "", err
	}

	return newAccessToken, nil
}

func (s *authService) LogoutUser(ctx context.Context, refreshTokenString string) error {
	hash := sha256.Sum256([]byte(refreshTokenString))
	tokenHash := hex.EncodeToString(hash[:])

	return s.tokenRepo.Delete(ctx, tokenHash)
}
