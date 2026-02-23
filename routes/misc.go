package routes

import (
	"encoding/json"
	"log"
	"net/http"
)

// HandleCatchAll handles all unmatched routes
func HandleCatchAll(w http.ResponseWriter, r *http.Request) {
	method := r.Method
	path := r.URL.Path[1:] // Remove leading slash
	host := r.Host

	log.Printf("[CATCH-ALL] %s /%s | Host=%s", method, path, host)

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]any{
		"status": "ok",
		"type":   "proxy_catch_all",
		"path":   "/" + path,
	})
}
