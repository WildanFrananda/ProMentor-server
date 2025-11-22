package tracing

import (
	"context"
	"log"
	"os"
	"time"

	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"

	"go.opentelemetry.io/otel"
	"go.opentelemetry.io/otel/exporters/otlp/otlptrace/otlptracegrpc"
	"go.opentelemetry.io/otel/propagation"
	"go.opentelemetry.io/otel/sdk/resource"
	sdktrace "go.opentelemetry.io/otel/sdk/trace"
	semconv "go.opentelemetry.io/otel/semconv/v1.26.0"
)

func InitTracerProvider(serviceName string) (func(context.Context) error, error) {
	ctx := context.Background()
	otelAgentAddr := os.Getenv("OTEL_EXPORTER_OTLP_ENDPOINT")

	if otelAgentAddr == "" {
		otelAgentAddr = "jaeger:4317"
	}

	conn, err := grpc.NewClient(
		otelAgentAddr,
		grpc.WithTransportCredentials(insecure.NewCredentials()),
	)

	if err != nil {
		return nil, err
	}

	exporter, err := otlptracegrpc.New(ctx, otlptracegrpc.WithGRPCConn(conn))

	if err != nil {
		return nil, err
	}

	res, err := resource.New(ctx,
		resource.WithAttributes(
			semconv.ServiceNameKey.String(serviceName),
			semconv.TelemetrySDKNameKey.String("opentelemetry"),
			semconv.TelemetrySDKLanguageKey.String("go"),
		),
	)

	if err != nil {
		return nil, err
	}

	tp := sdktrace.NewTracerProvider(
		sdktrace.WithBatcher(exporter),
		sdktrace.WithResource(res),
	)

	otel.SetTracerProvider(tp)
	otel.SetTextMapPropagator(propagation.NewCompositeTextMapPropagator(propagation.TraceContext{}, propagation.Baggage{}))

	log.Printf("Initializing OpenTelemetry for service '%s', sending traces to %s", serviceName, otelAgentAddr)

	return func(ctx context.Context) error {
		ctx, cancel := context.WithTimeout(ctx, time.Second*5)

		defer cancel()

		log.Printf("Shutting down OpenTelemetry provider for service '%s'...", serviceName)

		return tp.Shutdown(ctx)
	}, nil
}
