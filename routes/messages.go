package routes

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"log"
	"net/http"
	"time"

	"go-proxy/config"
)

// MessageRequest represents the Claude API message request
type MessageRequest struct {
	Model     string `json:"model"`
	MaxTokens int    `json:"max_tokens"`
	Stream    bool   `json:"stream"`
	Messages  []struct {
		Role    string `json:"role"`
		Content any    `json:"content"`
	} `json:"messages"`
}

// ContentBlock represents a content block
type ContentBlock struct {
	Type string `json:"type"`
	Text string `json:"text,omitempty"`
}

// MessageResponse represents the Claude API message response
type MessageResponse struct {
	ID           string         `json:"id"`
	Type         string         `json:"type"`
	Role         string         `json:"role"`
	Content      []ContentBlock `json:"content"`
	Model        string         `json:"model"`
	StopReason   string         `json:"stop_reason"`
	StopSequence *string        `json:"stop_sequence,omitempty"`
	Usage        Usage          `json:"usage"`
}

// Usage represents token usage
type Usage struct {
	InputTokens  int `json:"input_tokens"`
	OutputTokens int `json:"output_tokens"`
}

// ErrorResponse represents an error response
type ErrorResponse struct {
	Type    string   `json:"type"`
	Error   ErrorDetail `json:"error"`
}

type ErrorDetail struct {
	Type    string `json:"type"`
	Message string `json:"message"`
}

// HandleMessages handles the /v1/messages endpoint with real API call
func HandleMessages(cfg *config.Config) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		startTime := time.Now()
		requestID := fmt.Sprintf("req_%d", time.Now().UnixNano())

		// Set Anthropic-compatible headers
		w.Header().Set("x-anthropic-ratelimit-requests-limit", "10000")
		w.Header().Set("x-anthropic-ratelimit-tokens-limit", "1000000")
		w.Header().Set("x-request-id", requestID)

		// Parse request body
		var req MessageRequest
		if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
			sendError(w, "invalid_request_error", fmt.Sprintf("Invalid JSON: %v", err), 400)
			return
		}

		// Map model
		targetModel := cfg.ModelMap[req.Model]
		if targetModel == "" {
			targetModel = cfg.DefaultModel
		}

		// Prepare upstream request
		upstreamReq := map[string]any{
			"model":      targetModel,
			"max_tokens": req.MaxTokens,
			"stream":     req.Stream,
			"messages":   req.Messages,
		}

		reqBody, _ := json.Marshal(upstreamReq)

		// Create HTTP request
		upstreamURL := fmt.Sprintf("%s/v1/messages", cfg.APIBase)
		reqHTTP, err := http.NewRequest("POST", upstreamURL, bytes.NewReader(reqBody))
		if err != nil {
			sendError(w, "internal_error", "Failed to create upstream request", 500)
			return
		}

		// Set headers
		reqHTTP.Header.Set("Content-Type", "application/json")
		reqHTTP.Header.Set("Authorization", fmt.Sprintf("Bearer %s", cfg.APIKey))
		reqHTTP.Header.Set("anthropic-version", "2023-06-01")

		log.Printf("[Messages] %s POST /v1/messages model=%s target=%s",
			requestID, req.Model, targetModel)

		// Execute request
		client := &http.Client{
			Timeout: 300 * time.Second,
		}

		resp, err := client.Do(reqHTTP)
		if err != nil {
			sendError(w, "api_error", fmt.Sprintf("Upstream request failed: %v", err), 502)
			return
		}
		defer resp.Body.Close()

		// Read response
		body, err := io.ReadAll(resp.Body)
		if err != nil {
			sendError(w, "internal_error", "Failed to read response", 500)
			return
		}

		// Handle streaming response
		if req.Stream || resp.Header.Get("content-type") == "text/event-stream" {
			w.Header().Set("Content-Type", "text/event-stream")
			w.Header().Set("Cache-Control", "no-cache")
			w.Header().Set("Connection", "keep-alive")
			w.WriteHeader(resp.StatusCode)
			w.Write(body)
			log.Printf("[Messages] %s -> %d (stream) in %v",
				requestID, resp.StatusCode, time.Since(startTime))
			return
		}

		// Non-streaming response
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(resp.StatusCode)
		w.Write(body)

		log.Printf("[Messages] %s -> %d in %v",
			requestID, resp.StatusCode, time.Since(startTime))
	}
}

// HandleCountTokens handles token counting
func HandleCountTokens(w http.ResponseWriter, r *http.Request) {
	var req map[string]any
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		sendError(w, "invalid_request_error", "Invalid JSON", 400)
		return
	}

	messages, _ := req["messages"].([]any)
	text, _ := json.Marshal(messages)
	estimated := max(1, len(text)/4)

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]any{
		"input_tokens": estimated,
	})
}

// HandleBatches handles batch requests
func HandleBatches(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(404)
	json.NewEncoder(w).Encode(map[string]any{
		"type":  "error",
		"error": map[string]any{
			"type":    "not_found_error",
			"message": "Batches not available",
		},
	})
}

// HandleComplete handles legacy complete endpoint
func HandleComplete(cfg *config.Config) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		var req map[string]any
		if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
			sendError(w, "invalid_request_error", "Invalid JSON", 400)
			return
		}

		// Convert to messages format
		prompt, _ := req["prompt"].(string)
		newReq := map[string]any{
			"model": req["model"],
			"max_tokens": req["max_tokens"],
			"stream": false,
			"messages": []map[string]any{
				{"role": "user", "content": prompt},
			},
		}

		newBody, _ := json.Marshal(newReq)

		// Create a new request with the converted body
		r.Body = io.NopCloser(bytes.NewReader(newBody))
		r.Header.Set("Content-Type", "application/json")

		// Call HandleMessages
		HandleMessages(cfg)(w, r)
	}
}

func sendError(w http.ResponseWriter, errorType, message string, code int) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(code)
	json.NewEncoder(w).Encode(map[string]any{
		"type":  "error",
		"error": map[string]any{
			"type":    errorType,
			"message": message,
		},
	})
}

func max(a, b int) int {
	if a > b {
		return a
	}
	return b
}
