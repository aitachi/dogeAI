package main

import (
	"log"
	"net/http"
	"os"
	"time"

	"go-proxy/config"
	"go-proxy/routes"

	"github.com/gorilla/mux"
	"github.com/rs/cors"
)

func main() {
	// Load configuration
	cfg, err := config.LoadConfig()
	if err != nil {
		log.Fatalf("Failed to load config: %v", err)
	}

	// Create router
	r := mux.NewRouter()

	// Base routes
	r.HandleFunc("/", routes.HandleRoot)
	r.HandleFunc("/health", routes.HandleHealth)
	r.HandleFunc("/v1/models", routes.HandleModels)
	r.HandleFunc("/v1/models/", routes.HandleModelDetail)

	// Message API
	r.HandleFunc("/v1/messages", routes.HandleMessages(cfg)).Methods("POST")
	r.HandleFunc("/v1/messages/count_tokens", routes.HandleCountTokens).Methods("POST")
	r.HandleFunc("/v1/messages/batches", routes.HandleBatches).Methods("POST")
	r.HandleFunc("/v1/complete", routes.HandleComplete(cfg)).Methods("POST")

	// Hello endpoints
	r.HandleFunc("/api/hello", routes.HandleAPIHello)
	r.HandleFunc("/v1/oauth/hello", routes.HandleOAuthHello)

	// Monitor routes
	r.HandleFunc("/api/metrics", routes.HandleMetrics)
	r.HandleFunc("/api/calls/recent", routes.HandleRecentCalls)
	r.HandleFunc("/api/stats", routes.HandleStats)
	r.HandleFunc("/api/health", routes.HandleAPIHealth)
	r.HandleFunc("/monitor.html", routes.HandleMonitorPage)

	// OAuth routes
	r.HandleFunc("/oauth/authorize", routes.HandleOAuthAuthorize)
	r.HandleFunc("/oauth/code/callback", routes.HandleOAuthCodeCallback)
	r.HandleFunc("/generate-code", routes.HandleGenerateCode)
	r.HandleFunc("/oauth/token", routes.HandleOAuthToken).Methods("POST")

	// Platform API routes
	r.HandleFunc("/api/bootstrap", routes.HandleBootstrap)
	r.HandleFunc("/api/auth", routes.HandleAuth)
	r.HandleFunc("/api/auth/session", routes.HandleAuthSession)
	r.HandleFunc("/api/account", routes.HandleAccount)
	r.HandleFunc("/api/settings", routes.HandleSettings)
	r.HandleFunc("/api/organizations", routes.HandleOrganizations)
	r.HandleFunc("/api/organizations/", routes.HandleOrganizationDetail).Methods("GET", "POST")
	r.HandleFunc("/api/report", routes.HandleReport).Methods("POST")
	r.HandleFunc("/api/telemetry", routes.HandleTelemetry).Methods("POST")
	r.HandleFunc("/api/events", routes.HandleEvents).Methods("POST")
	r.HandleFunc("/api/statsig", routes.HandleStatsig).Methods("POST")

	// Claude Code routes
	r.HandleFunc("/api/claude_code/settings", routes.HandleClaudeSettings)
	r.HandleFunc("/api/claude_code/policy_limits", routes.HandleClaudePolicyLimits)
	r.HandleFunc("/api/claude_code/penguin_mode", routes.HandleClaudePenguinMode)
	r.HandleFunc("/.well-known/openid-configuration", routes.HandleOpenIDConfig)
	r.HandleFunc("/.well-known/oauth-authorization-server", routes.HandleOAuthMetadata)
	r.HandleFunc("/userinfo", routes.HandleUserInfo).Methods("GET", "POST")
	r.HandleFunc("/api/me", routes.HandleAPIME).Methods("GET", "POST")
	r.HandleFunc("/v1/me", routes.HandleV1ME).Methods("GET", "POST")
	r.HandleFunc("/v1/dashboard/billing/usage", routes.HandleBillingUsage)
	r.HandleFunc("/v1/usage", routes.HandleUsage).Methods("GET", "POST")
	r.HandleFunc("/v1/organizations/", routes.HandleV1Organization).Methods("GET", "POST")
	r.HandleFunc("/v1/organizations/{org_id}/api_keys", routes.HandleCreateAPIKey).Methods("POST")
	r.HandleFunc("/api/organizations/{org_id}/api_keys", routes.HandleCreateAPIKeyHyphen).Methods("POST")

	// Test endpoints
	r.HandleFunc("/test", routes.HandleTest(cfg))
	r.HandleFunc("/test-anthropic", routes.HandleTestAnthropic)

	// Catch-all
	r.PathPrefix("/{path:.*}").HandlerFunc(routes.HandleCatchAll)

	// CORS middleware
	handler := cors.Default().Handler(r)

	// Start server
	port := os.Getenv("PORT")
	if port == "" {
		port = "3001"
	}

	log.Printf("Starting Go Claude Proxy on port %s...", port)
	log.Printf("API Base: %s", cfg.APIBase)
	log.Printf("Default Model: %s", cfg.DefaultModel)

	server := &http.Server{
		Addr:         ":" + port,
		Handler:      handler,
		ReadTimeout:  300 * time.Second,
		WriteTimeout: 300 * time.Second,
		IdleTimeout:  120 * time.Second,
	}

	if err := server.ListenAndServe(); err != nil {
		log.Fatalf("Server failed: %v", err)
	}
}
