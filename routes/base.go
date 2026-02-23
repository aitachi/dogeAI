package routes

import (
	"crypto/rand"
	"encoding/hex"
	"encoding/json"
	"net/http"
	"time"

	"go-proxy/config"
)

// HandleRoot handles the root endpoint
func HandleRoot(w http.ResponseWriter, r *http.Request) {
	sendJSON(w, map[string]string{"status": "ok", "proxy": "aitachi-cloud-go-v1.0"})
}

// HandleHealth handles the health endpoint
func HandleHealth(w http.ResponseWriter, r *http.Request) {
	sendJSON(w, map[string]string{"status": "ok"})
}

// HandleModels handles the model list endpoint
func HandleModels(w http.ResponseWriter, r *http.Request) {
	cfg, _ := config.LoadConfig()
	models := make([]map[string]any, 0, len(cfg.ModelMap))
	for modelName := range cfg.ModelMap {
		models = append(models, map[string]any{
			"id":        modelName,
			"object":    "model",
			"created":   time.Now().Unix() - 86400,
			"owned_by":  "anthropic",
		})
	}
	sendJSON(w, map[string]any{"object": "list", "data": models})
}

// HandleModelDetail handles model detail endpoint
func HandleModelDetail(w http.ResponseWriter, r *http.Request) {
	sendJSON(w, map[string]any{
		"object":   "model",
		"created":  time.Now().Unix() - 86400,
		"owned_by": "anthropic",
	})
}

// HandleTest handles test endpoint
func HandleTest(cfg *config.Config) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		sendJSON(w, map[string]any{
			"status": "ok",
			"config": map[string]any{
				"api_base":           cfg.APIBase,
				"default_model":      cfg.DefaultModel,
				"max_output_tokens":  cfg.MaxOutput,
			},
		})
	}
}

// HandleTestAnthropic handles test-anthropic endpoint
func HandleTestAnthropic(w http.ResponseWriter, r *http.Request) {
	sendJSON(w, map[string]any{"status": "ok", "message": "Go proxy test endpoint"})
}

// genID generates a random ID
func genID() string {
	b := make([]byte, 12)
	rand.Read(b)
	return hex.EncodeToString(b)
}

// genUUID generates a UUID-like string
func genUUID() string {
	b := make([]byte, 16)
	rand.Read(b)
	return hex.EncodeToString(b)
}

// sendJSON sends a JSON response
func sendJSON(w http.ResponseWriter, data any) {
	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(data)
}
