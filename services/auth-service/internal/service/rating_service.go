package service

import (
	"auth-service/internal/model"
	"auth-service/internal/repository"
	"context"
	"database/sql"
	"errors"
	"time"

	"github.com/google/uuid"
)

var (
	ErrUserNotInSession         = errors.New("user did not participate in this session")
	ErrAlreadyRated             = errors.New("user has already rated this session")
	ErrSessionNotReadyForRating = errors.New("session is not started yet")
)

type RatingService interface {
	RateSession(ctx context.Context, sessionID, userID uuid.UUID, rating int, comment *string) error
}

type ratingService struct {
	ratingRepo  repository.RatingRepository
	sessionRepo repository.SessionRepository
}

func NewRatingService(ratingRepo repository.RatingRepository, sessionRepo repository.SessionRepository) RatingService {
	return &ratingService{
		ratingRepo:  ratingRepo,
		sessionRepo: sessionRepo,
	}
}

func (s *ratingService) RateSession(ctx context.Context, sessionID, userID uuid.UUID, rating int, comment *string) error {
	joined, err := s.ratingRepo.CheckIfUserJoinedSession(ctx, userID, sessionID)
	if err != nil {
		return err
	}
	if !joined {
		return ErrUserNotInSession
	}

	session, err := s.sessionRepo.FindByID(ctx, sessionID)
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return ErrSessionNotFound
		}
		return err
	}
	if session.StartAt.After(time.Now()) {
		return ErrSessionNotReadyForRating
	}

	newRating := &model.Rating{
		SessionID: sessionID,
		UserID:    userID,
		Rating:    rating,
		Comment:   comment,
	}
	err = s.ratingRepo.Create(ctx, newRating)
	if err != nil {
		return ErrAlreadyRated
	}

	return nil
}
