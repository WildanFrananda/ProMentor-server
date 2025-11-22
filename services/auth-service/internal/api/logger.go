package api

import (
	"context"
	"log/slog"
	"os"

	"go.opentelemetry.io/otel/trace"
)

type OtelHandler struct {
	next slog.Handler
}

func NewOtelHandler(next slog.Handler) *OtelHandler {
	return &OtelHandler{next: next}
}

func (h *OtelHandler) Enabled(ctx context.Context, level slog.Level) bool {
	return h.next.Enabled(ctx, level)
}

func (h *OtelHandler) Handle(ctx context.Context, r slog.Record) error {
	spanCtx := trace.SpanContextFromContext(ctx)

	if spanCtx.IsValid() {
		r.AddAttrs(slog.String("trace_id", spanCtx.TraceID().String()))
		r.AddAttrs(slog.String("span_id", spanCtx.SpanID().String()))
	}

	return h.next.Handle(ctx, r)
}

func (h *OtelHandler) WithGroup(name string) slog.Handler {
	return NewOtelHandler(h.next.WithGroup(name))
}

func (h *OtelHandler) WithAttrs(attrs []slog.Attr) slog.Handler {
	return NewOtelHandler(h.next.WithAttrs(attrs))
}

func SetupGlobalHandler(serviceName string) {
	jsonHandler := slog.NewJSONHandler(os.Stdout, nil)
	otelHandler := NewOtelHandler(jsonHandler)
	logger := slog.New(otelHandler).With(slog.String("service", serviceName))
	slog.SetDefault(logger)

	slog.Info("Logger initialized", "service", serviceName)
}
