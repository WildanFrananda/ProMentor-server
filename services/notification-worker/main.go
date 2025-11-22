package main

import (
	"log"
	"notification-worker/worker"
	"os"
	"os/signal"
	"syscall"

	"github.com/joho/godotenv"
)

func main() {
	godotenv.Load(".env.dev")

	natsURL := os.Getenv("NATS_URL")

	if natsURL == "" {
		log.Fatal("NATS_URL environment variable is not set")
	}

	if err := worker.Start(natsURL); err != nil {
		log.Fatalf("Failed to start worker: %v", err)
	}

	log.Println("Notification worker started, waiting for events...")

	quit := make(chan os.Signal, 1)
	signal.Notify(quit, syscall.SIGINT, syscall.SIGTERM)
	<-quit

	log.Println("Shutting down notification worker...")
}
