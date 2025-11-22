package service

import (
	"context"
	"database/sql"
	"errors"
	"user-service/internal/model"
	"user-service/internal/repository"

	"github.com/google/uuid"
)

type UpdateUserDTO struct {
	Name      *string
	AvatarURL *string
}

type UserService interface {
	UpdateUserProfile(ctx context.Context, userID uuid.UUID, dto UpdateUserDTO) (*model.User, error)
	RegisterDeviceToken(ctx context.Context, userID uuid.UUID, token string) error
	GetUserProfileByID(ctx context.Context, userID uuid.UUID) (*model.User, error)
}

type userService struct {
	userRepo repository.UserRepository
}

func NewUserService(userRepo repository.UserRepository) UserService {
	return &userService{userRepo: userRepo}
}

func (s *userService) UpdateUserProfile(ctx context.Context, userID uuid.UUID, dto UpdateUserDTO) (*model.User, error) {
	userToUpdate := &model.User{
		ID:        userID,
		Name:      dto.Name,
		AvatarURL: dto.AvatarURL,
	}

	if err := s.userRepo.Update(ctx, userToUpdate); err != nil {
		return nil, err
	}

	return s.userRepo.FindByID(ctx, userID)
}

func (s *userService) RegisterDeviceToken(ctx context.Context, userID uuid.UUID, token string) error {
	return s.userRepo.RegisterDeviceToken(ctx, userID, token)
}

func (s *userService) GetUserProfileByID(ctx context.Context, userID uuid.UUID) (*model.User, error) {
	user, err := s.userRepo.FindByID(ctx, userID)
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return nil, errors.New("User not found")
		}

		return nil, err
	}

	return user, nil
}
