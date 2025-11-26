package main

import (
	"context"
	"database/sql"
	"fmt"
	"log"
	"os"

	"github.com/gofiber/contrib/otelfiber/v2"
	"github.com/gofiber/fiber/v2"
	"github.com/gofiber/fiber/v2/middleware/adaptor"
	"github.com/jmoiron/sqlx"
	"github.com/joho/godotenv"
	"github.com/pressly/goose/v3"
	"github.com/prometheus/client_golang/prometheus/promhttp"

	_ "github.com/jackc/pgx/v5/stdlib"

	"auth-service/internal/api"
	"auth-service/internal/events"
	"auth-service/internal/repository"
	"auth-service/internal/service"
	"auth-service/internal/tracing"
	_ "auth-service/migrations"
)

func main() {
	if err := godotenv.Load(".env.dev"); err != nil {
		fmt.Println("No .env.dev file found, reading from environment variables provided by Docker")
	}

	api.SetupGlobalHandler("auth-service")

	shutdownTracer, err := tracing.InitTracerProvider("auth-service")
	if err != nil {
		log.Fatalf("Failed to initialize OpenTelemetry: %v", err)
	}
	defer func() {
		if err := shutdownTracer(context.Background()); err != nil {
			log.Printf("Error shutting down tracer provider: %v", err)
		}
	}()

	if len(os.Args) > 1 && os.Args[1] == "migrate" {
		handleMigrations()
		return
	}

	db := connectDB()
	defer db.Close()

	natsURL := os.Getenv("NATS_URL")
	if natsURL == "" {
		natsURL = "nats://localhost:4222"
	}
	eventPublisher, err := events.NewNatsPublisher(natsURL)
	if err != nil {
		log.Fatalf("Failed to connect to NATS: %v", err)
	}
	log.Println("Successfully connected to NATS.")

	userRepo := repository.NewPostgresUserRepository(db)
	sessionRepo := repository.NewPostgresSessionRepository(db)
	tokenRepo := repository.NewPostgresTokenRepository(db)
	chatRepo := repository.NewPostgresChatRepository(db)
	ratingRepo := repository.NewPostgresRatingRepository(db)

	authService := service.NewAuthService(userRepo, tokenRepo)
	sessionService := service.NewSessionService(sessionRepo, eventPublisher)
	ratingService := service.NewRatingService(ratingRepo, sessionRepo)

	_, err = events.NewChatSubscriber(natsURL, chatRepo)
	if err != nil {
		log.Printf("WARNING: Failed to start chat subscriber: %v", err)
		// Continue running even if subscriber fails, NATS may not be ready
	}

	authHandler := api.NewAuthHandler(authService)
	sessionHandler := api.NewSessionHandler(sessionService, authService)
	ratingHandler := api.NewRatingHandler(ratingService)

	app := fiber.New()
	app.Use(otelfiber.Middleware())
	app.Use(api.PrometheusMiddleware())

	app.Get("/health", func(c *fiber.Ctx) error {
		return c.JSON(fiber.Map{"status": "ok", "service": "auth-service"})
	})

	app.Get("/metrics", adaptor.HTTPHandler(promhttp.Handler()))

	v1 := app.Group("/v1")

	authRoutes := v1.Group("/auth")
	authRoutes.Post("/register", authHandler.Register)
	authRoutes.Post("/login", authHandler.Login)
	authRoutes.Post("/refresh", authHandler.Refresh)
	authRoutes.Post("/logout", authHandler.Logout)

	userRoutes := v1.Group("/users")
	userRoutes.Use(api.AuthMiddleware())
	userRoutes.Get("/me", sessionHandler.GetUserProfile)

	sessionsRoutes := v1.Group("/sessions")
	sessionsRoutes.Use(api.AuthMiddleware())
	sessionsRoutes.Get("/", sessionHandler.ListUpcomingSessions)
	sessionsRoutes.Get("/history", sessionHandler.ListHistory)
	sessionsRoutes.Get("/:id", api.InternalAuthMiddleware(), sessionHandler.GetSessionDetails)

	v1.Get("/categories", sessionHandler.GetCategories)

	sessionsRoutes.Post("/", sessionHandler.CreateSession)
	sessionsRoutes.Post("/:id/join", sessionHandler.JoinSession)
	sessionsRoutes.Post("/:id/rate", ratingHandler.RateSession)

	port := os.Getenv("APP_PORT")
	if port == "" {
		port = "8001"
	}

	log.Printf("Listening auth-service on port %s", port)
	log.Fatal(app.Listen(":" + port))
}

func connectDB() *sqlx.DB {
	dbUser := os.Getenv("DB_USER")
	dbPassword := os.Getenv("DB_PASSWORD")
	dbHost := os.Getenv("DB_HOST")
	dbPort := os.Getenv("DB_PORT")
	dbName := os.Getenv("DB_NAME")

	dbURL := fmt.Sprintf("postgres://%s:%s@%s:%s/%s?sslmode=disable",
		dbUser, dbPassword, dbHost, dbPort, dbName,
	)

	db, err := sqlx.Connect("pgx", dbURL)
	if err != nil {
		log.Fatalf("Failed to connect to database: %v", err)
	}
	log.Println("Successfully connected to the database.")
	return db
}

func handleMigrations() {
	fmt.Println("Running database migrations...")
	dbUser := os.Getenv("DB_USER")
	dbPassword := os.Getenv("DB_PASSWORD")
	dbHost := os.Getenv("DB_HOST")
	dbPort := os.Getenv("DB_PORT")
	dbName := os.Getenv("DB_NAME")

	dbURL := fmt.Sprintf("postgres://%s:%s@%s:%s/%s?sslmode=disable",
		dbUser, dbPassword, dbHost, dbPort, dbName,
	)

	db, err := sql.Open("pgx", dbURL)
	if err != nil {
		log.Fatalf("failed to connect to database for migration: %v", err)
	}
	defer db.Close()

	if err := goose.SetDialect("postgres"); err != nil {
		log.Fatalf("failed to set goose dialect: %v", err)
	}

	if err := goose.Up(db, "migrations"); err != nil {
		log.Fatalf("goose: failed to run migrations: %v", err)
	}

	fmt.Println("Migrations applied successfully!")
}
