package repository

import (
	"auth-service/internal/model"
	"context"
	"log"
	"os"
	"testing"
	"time"

	"github.com/google/uuid"
	_ "github.com/jackc/pgx/v5/stdlib"
	"github.com/jmoiron/sqlx"
	"github.com/pressly/goose/v3"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/suite"
	"github.com/testcontainers/testcontainers-go"
	"github.com/testcontainers/testcontainers-go/modules/postgres"
	"github.com/testcontainers/testcontainers-go/wait"
)

type UserRepositoryIntegrationTestSuite struct {
	suite.Suite
	db   *sqlx.DB
	repo UserRepository
	pgc  *postgres.PostgresContainer
	ctx  context.Context
}

func (s *UserRepositoryIntegrationTestSuite) SetupSuite() {
	s.ctx = context.Background()

	pgc, err := postgres.Run(s.ctx,
		"postgres:15-alpine",
		postgres.WithDatabase("test-db"),
		postgres.WithUsername("user"),
		postgres.WithPassword("password"),
		testcontainers.WithWaitStrategy(
			wait.ForLog("database system is ready to accept connections").
				WithOccurrence(2).
				WithStartupTimeout(5*time.Second),
		),
	)
	if err != nil {
		log.Fatalf("could not start postgres container: %s", err)
	}
	s.pgc = pgc

	connStr, err := pgc.ConnectionString(s.ctx, "sslmode=disable")
	assert.NoError(s.T(), err)

	db, err := sqlx.Connect("pgx", connStr)
	assert.NoError(s.T(), err)
	s.db = db

	err = goose.Up(db.DB, "../../migrations")
	assert.NoError(s.T(), err)

	s.repo = NewPostgresUserRepository(s.db)
}

func (s *UserRepositoryIntegrationTestSuite) TearDownSuite() {
	s.db.Close()
	if err := s.pgc.Terminate(s.ctx); err != nil {
		log.Fatalf("failed to terminate pg container: %s", err)
	}
}

func (s *UserRepositoryIntegrationTestSuite) TestUserRepository_CreateAndFindByEmail() {
	// Arrange
	testEmail := "integration@test.com"
	user := &model.User{
		Email:        testEmail,
		PasswordHash: "hashed_password",
		Name:         "Integration Test User",
	}

	// Act: Create new user
	newID, err := s.repo.Create(s.ctx, user)

	// Assert: Make sure user created successfully
	assert.NoError(s.T(), err)
	assert.NotEqual(s.T(), uuid.Nil, newID)

	// Act: Find user by email
	foundUser, err := s.repo.FindByEmail(s.ctx, testEmail)

	// Assert: Make sure user found successfully
	assert.NoError(s.T(), err)
	assert.NotNil(s.T(), foundUser)
	assert.Equal(s.T(), newID, foundUser.ID)
	assert.Equal(s.T(), testEmail, foundUser.Email)
}

func (s *UserRepositoryIntegrationTestSuite) TestUserRepository_FindByEmail_NotFound() {
	// Act
	foundUser, err := s.repo.FindByEmail(s.ctx, "nonexistent@test.com")

	// Assert
	assert.Error(s.T(), err)
	assert.Nil(s.T(), foundUser)
}

func TestUserRepositoryIntegration(t *testing.T) {
	if os.Getenv("DOCKER_HOST") == "" {
		t.Skip("Docker is not available, skipping integration test.")
	}
	suite.Run(t, new(UserRepositoryIntegrationTestSuite))
}
