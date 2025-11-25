package service

import (
	"auth-service/internal/events"
	"auth-service/internal/model"
	"auth-service/internal/repository"
	"context"
	"errors"

	"github.com/google/uuid"
)

var (
	ErrSessionNotFound = errors.New("session not found")
	ErrAlreadyJoined   = errors.New("user has already joined this session")
	ErrSessionFull     = errors.New("session is full")
)

type SessionService interface {
	CreateSession(ctx context.Context, session *model.Session) (*model.Session, error)
	JoinSession(ctx context.Context, sessionID, userID uuid.UUID) error
	ListUpcomingSessions(ctx context.Context, page int, limit int, query string) ([]model.SessionDetails, error)
	ListUserHistory(ctx context.Context, userID uuid.UUID) ([]model.SessionDetails, error)
	GetSessionDetails(ctx context.Context, sessionID uuid.UUID) (*model.Session, error)
}

type sessionService struct {
	sessionRepo repository.SessionRepository
	publisher   events.EventPublisher
}

func NewSessionService(repo repository.SessionRepository, pub events.EventPublisher) SessionService {
	return &sessionService{sessionRepo: repo, publisher: pub}
}

func (s *sessionService) CreateSession(ctx context.Context, session *model.Session) (*model.Session, error) {
	createdSession, err := s.sessionRepo.Create(ctx, session)

	if err != nil {
		return nil, err
	}

	go s.publisher.PublishSessionCreated(createdSession)

	return createdSession, nil
}

func (s *sessionService) JoinSession(ctx context.Context, sessionID, userID uuid.UUID) error {
	session, err := s.sessionRepo.FindByID(ctx, sessionID)

	if err != nil {
		return err
	}

	if session == nil {
		return ErrSessionNotFound
	}

	count, err := s.sessionRepo.CountParticipants(ctx, sessionID)

	if err != nil {
		return err
	}

	if count >= session.Capacity {
		return ErrSessionFull
	}

	err = s.sessionRepo.AddParticipant(ctx, sessionID, userID, "attendee")

	if err != nil {
		return ErrAlreadyJoined
	}

	go s.publisher.PublishSessionJoined(sessionID, userID)

	return nil
}

func (s *sessionService) ListUpcomingSessions(ctx context.Context, page int, limit int, query string) ([]model.SessionDetails, error) {
	offset := (page - 1) * limit
	return s.sessionRepo.ListUpcoming(ctx, limit, offset, query)
}

func (s *sessionService) ListUserHistory(ctx context.Context, userID uuid.UUID) ([]model.SessionDetails, error) {
	return s.sessionRepo.ListHistoryByUserID(ctx, userID)
}

func (s *sessionService) GetSessionDetails(ctx context.Context, sessionID uuid.UUID) (*model.Session, error) {
	session, err := s.sessionRepo.FindByID(ctx, sessionID)
	if err != nil {
		return nil, err
	}
	if session == nil {
		return nil, ErrSessionNotFound
	}
	return session, nil
}
