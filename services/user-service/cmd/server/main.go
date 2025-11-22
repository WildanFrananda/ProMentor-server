package main

import (
	"fmt"
	"log"
	"os"
	"user-service/internal/api"
	"user-service/internal/repository"
	"user-service/internal/s3"
	"user-service/internal/service"

	"github.com/gofiber/fiber/v2"
	_ "github.com/jackc/pgx/v5/stdlib"
	"github.com/jmoiron/sqlx"
	"github.com/joho/godotenv"
)

func main() {
	godotenv.Load(".env.dev")

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
		log.Fatalf("Failed connect to database: %v", err)
	}
	defer db.Close()
	log.Println("Database connected.")

	filePresigner, err := s3.NewFilePresigner()

	if err != nil {
		log.Fatalf("Failed to initialize S3 presigner: %v", err)
	}

	log.Println("Successfully initialized S3 presigner.")

	userRepo := repository.NewPostgresUserRepository(db)
	userService := service.NewUserService(userRepo)
	userHandler := api.NewUserHandler(userService, filePresigner)

	app := fiber.New()

	app.Get("/health", func(c *fiber.Ctx) error {
		return c.JSON(fiber.Map{
			"status":  "ok",
			"service": "user-service",
		})
	})

	v1 := app.Group("/v1")
	v1.Get("/users/:id", api.InternalAuthMiddleware(), userHandler.GetUserProfileByID)
	usersRoutes := v1.Group("/users")
	usersRoutes.Use(api.AuthMiddleware())
	usersRoutes.Put("/me", userHandler.UpdateUserProfile)
	usersRoutes.Post("/me/avatar/upload-url", userHandler.GetAvatarUploadURL)
	usersRoutes.Post("/me/device-token", userHandler.RegisterDeviceToken)

	port := os.Getenv("APP_PORT")
	if port == "" {
		port = "8002"
	}

	log.Printf("Listening user-service on port %s", port)
	log.Fatal(app.Listen(":" + port))
}
