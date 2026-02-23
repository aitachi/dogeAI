package routes

import (
	"fmt"
	"net/http"
	"time"
)

// HandleBootstrap handles bootstrap endpoint
func HandleBootstrap(w http.ResponseWriter, r *http.Request) {
	sendJSON(w, map[string]any{
		"account":       userProfile(),
		"organizations": []any{orgInfo()},
		"statsig":       map[string]any{},
	})
}

// HandleAuth handles auth endpoint
func HandleAuth(w http.ResponseWriter, r *http.Request) {
	sendJSON(w, map[string]any{
		"account":      userProfile(),
		"account_flags": []any{},
	})
}

// HandleAuthSession handles auth session endpoint
func HandleAuthSession(w http.ResponseWriter, r *http.Request) {
	sendJSON(w, map[string]any{
		"account": userProfile(),
		"session": map[string]any{
			"id":         fmt.Sprintf("sess_%s", genUUID()),
			"expires_at": time.Now().Add(30 * 24 * time.Hour).Format("2006-01-02T15:04:05Z"),
		},
		"account_flags": []any{},
	})
}

// HandleAccount handles account endpoint
func HandleAccount(w http.ResponseWriter, r *http.Request) {
	sendJSON(w, userProfile())
}

// HandleSettings handles settings endpoint
func HandleSettings(w http.ResponseWriter, r *http.Request) {
	sendJSON(w, map[string]any{})
}

// HandleOrganizations handles organizations endpoint
func HandleOrganizations(w http.ResponseWriter, r *http.Request) {
	sendJSON(w, map[string]any{"data": []any{orgInfo()}})
}

// HandleOrganizationDetail handles organization detail endpoint
func HandleOrganizationDetail(w http.ResponseWriter, r *http.Request) {
	sendJSON(w, orgInfo())
}

// HandleReport handles report endpoint
func HandleReport(w http.ResponseWriter, r *http.Request) { sendOK(w) }

// HandleTelemetry handles telemetry endpoint
func HandleTelemetry(w http.ResponseWriter, r *http.Request) { sendOK(w) }

// HandleEvents handles events endpoint
func HandleEvents(w http.ResponseWriter, r *http.Request) { sendOK(w) }

// HandleStatsig handles statsig endpoint
func HandleStatsig(w http.ResponseWriter, r *http.Request) { sendOK(w) }

// HandleCreateAPIKey handles API key creation
func HandleCreateAPIKey(w http.ResponseWriter, r *http.Request) {
	sendJSON(w, map[string]any{
		"id":         fmt.Sprintf("apikey_%s", genUUID()),
		"type":       "api_key",
		"api_key":    genFakeAPIKey(),
		"name":       "claude-code-session-key",
		"created_at": time.Now().Unix(),
		"status":     "active",
	})
}

// HandleCreateAPIKeyHyphen handles API key creation (hyphen version)
func HandleCreateAPIKeyHyphen(w http.ResponseWriter, r *http.Request) {
	HandleCreateAPIKey(w, r)
}

// sendOK sends a simple OK response
func sendOK(w http.ResponseWriter) {
	sendJSON(w, map[string]any{"ok": true})
}

// userProfile returns a fake user profile
func userProfile() map[string]any {
	return map[string]any{
		"id":          FAKE_ACCOUNT_UUID,
		"name":        "Claude Code User",
		"email":       "user@example.com",
		"type":        "individual",
		"created_at":  "2024-01-01T00:00:00Z",
		"preferences": map[string]any{},
	}
}

// orgInfo returns a fake organization info
func orgInfo() map[string]any {
	return map[string]any{
		"id":           FAKE_ACCOUNT_UUID,
		"name":         "Personal",
		"type":         "individual",
		"created_at":   "2024-01-01T00:00:00Z",
		"preferences":  map[string]any{},
		"capabilities": []string{"claude_code", "user:sessions:claude_code", "mcp_servers"},
	}
}

// genFakeAPIKey generates a fake API key
func genFakeAPIKey() string {
	return fmt.Sprintf("sk-ant-api03-%s", genUUID())
}
