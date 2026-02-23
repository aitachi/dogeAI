package routes

import (
	"fmt"
	"net/http"
)

// HandleAPIHello handles API hello endpoint
func HandleAPIHello(w http.ResponseWriter, r *http.Request) {
	sendJSON(w, map[string]string{"message": "hello"})
}

// HandleClaudeSettings handles Claude Code settings
func HandleClaudeSettings(w http.ResponseWriter, r *http.Request) {
	sendJSON(w, map[string]any{
		"expiry":        nil,
		"isolated":      false,
		"allowed_tools": []string{"computer", "text_editor", "bash"},
		"max_turns":     nil,
		"internet_policy": "allow",
	})
}

// HandleClaudePolicyLimits handles policy limits
func HandleClaudePolicyLimits(w http.ResponseWriter, r *http.Request) {
	sendJSON(w, map[string]any{
		"rate_limits": map[string]int{
			"requests_per_minute": 60,
			"tokens_per_minute":   1000000,
			"tokens_per_day":      50000000,
		},
		"usage": map[string]int{
			"tokens_used_today": 0,
			"requests_today":    0,
		},
		"limits": map[string]any{},
	})
}

// HandleClaudePenguinMode handles penguin mode
func HandleClaudePenguinMode(w http.ResponseWriter, r *http.Request) {
	sendJSON(w, map[string]any{"enabled": false})
}

// HandleOpenIDConfig handles OpenID configuration
func HandleOpenIDConfig(w http.ResponseWriter, r *http.Request) {
	base := getBaseURL(r)
	sendJSON(w, map[string]any{
		"issuer":                  base,
		"authorization_endpoint": base + "/oauth/authorize",
		"token_endpoint":          base + "/oauth/token",
		"userinfo_endpoint":       base + "/userinfo",
		"response_types_supported": []string{"code"},
		"grant_types_supported":    []string{"authorization_code", "refresh_token"},
		"code_challenge_methods_supported": []string{"S256"},
		"token_endpoint_auth_methods_supported": []string{"none"},
	})
}

// HandleOAuthMetadata handles OAuth metadata
func HandleOAuthMetadata(w http.ResponseWriter, r *http.Request) {
	HandleOpenIDConfig(w, r)
}

// HandleUserInfo handles userinfo endpoint
func HandleUserInfo(w http.ResponseWriter, r *http.Request) {
	sendJSON(w, userProfile())
}

// HandleAPIME handles API me endpoint
func HandleAPIME(w http.ResponseWriter, r *http.Request) {
	sendJSON(w, userProfile())
}

// HandleV1ME handles v1 me endpoint
func HandleV1ME(w http.ResponseWriter, r *http.Request) {
	sendJSON(w, userProfile())
}

// HandleBillingUsage handles billing usage
func HandleBillingUsage(w http.ResponseWriter, r *http.Request) {
	sendJSON(w, map[string]any{"daily_costs": []any{}, "total_usage": 0.0})
}

// HandleUsage handles usage endpoint
func HandleUsage(w http.ResponseWriter, r *http.Request) {
	sendJSON(w, map[string]any{"daily_costs": []any{}, "total_usage": 0})
}

// HandleV1Organization handles v1 organization detail
func HandleV1Organization(w http.ResponseWriter, r *http.Request) {
	sendJSON(w, orgInfo())
}

// getBaseURL gets the base URL from request
func getBaseURL(r *http.Request) string {
	scheme := "http"
	if r.TLS != nil {
		scheme = "https"
	}
	return fmt.Sprintf("%s://%s", scheme, r.Host)
}
