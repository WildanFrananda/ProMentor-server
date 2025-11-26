package api

import (
	"bff-service/internal/api/handlers"

	"github.com/gofiber/fiber/v2"
)

func SetupRoutes(app *fiber.App, authURL string, userURL string) {
	v1 := app.Group("/v1")

	auth := v1.Group("/auth")
	auth.Post("/register", handlers.ProxyTo(authURL, "/v1/auth/register"))
	auth.Post("/login", handlers.ProxyTo(authURL, "/v1/auth/login"))
	auth.Post("/refresh", handlers.ProxyTo(authURL, "/v1/auth/refresh"))
	auth.Post("/logout", handlers.ProxyTo(authURL, "/v1/auth/logout"))

	profile := v1.Group("/profile")
	profile.Get("/me", handlers.HandleGetMyProfile(authURL))
	profile.Put("/me", handlers.HandleUpdateMyProfile(userURL))
	profile.Post("/avatar/upload-url", handlers.GetAvatarUploadURLWithProxy(userURL))
	profile.Post("/device-token", handlers.ProxyTo(userURL, "/v1/users/me/device-token"))

	sessions := v1.Group("/sessions")
	sessions.Get("/", handlers.ProxyTo(authURL, "/v1/sessions"))
	sessions.Post("/", handlers.ProxyTo(authURL, "/v1/sessions"))
	sessions.Post("/:id/join", handlers.ProxyTo(authURL, "/v1/sessions/:id/join"))
	sessions.Get("/history", handlers.ProxyTo(authURL, "/v1/sessions/history"))
	sessions.Post("/:id/rate", handlers.ProxyTo(authURL, "/v1/sessions/:id/rate"))

	v1.Get("/session-details/:id", handlers.GetSessionDetails(authURL, userURL))
	v1.Get("/categories", handlers.ProxyTo(authURL, "/v1/categories"))
	v1.Put("/proxy/upload", handlers.HandleUploadProxy)
	v1.Get("/proxy/image", handlers.HandleDownloadProxy)
}
