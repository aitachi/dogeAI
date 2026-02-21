package main

import (
	"crypto/rand"
	"encoding/hex"
	"fmt"
	"log"
	"strings"
	"time"
)

func genID(prefix string) string {
	b := make([]byte, 12)
	rand.Read(b)
	return fmt.Sprintf("%s_%s", prefix, hex.EncodeToString(b)[:24])
}

func genRequestID() string {
	b := make([]byte, 16)
	rand.Read(b)
	return fmt.Sprintf("req_%s", hex.EncodeToString(b))
}

func mapModel(name string) string {
	if mapped, ok := Cfg.ModelMap[name]; ok {
		return mapped
	}
	nameLower := strings.ToLower(name)
	if strings.Contains(nameLower, "claude") || strings.Contains(nameLower, "sonnet") ||
		strings.Contains(nameLower, "opus") || strings.Contains(nameLower, "haiku") {
		log.Printf("Model '%s' not in map, using default '%s'", name, Cfg.DefaultModel)
		return Cfg.DefaultModel
	}
	log.Printf("Unknown model '%s', using default '%s'", name, Cfg.DefaultModel)
	return Cfg.DefaultModel
}

func clampMaxTokens(requested int, targetModel string) int {
	const hardMax = 8192
	limit := Cfg.MaxOutputTokensLimit[targetModel]
	if limit == 0 {
		limit = Cfg.DefaultMaxTokens
	}
	if limit > hardMax {
		limit = hardMax
	}
	if requested <= 0 {
		return limit
	}
	result := requested
	if result > limit {
		result = limit
		log.Printf("max_tokens clamped: %d -> %d", requested, result)
	}
	return result
}

func genFakeAPIKey() string {
	b := make([]byte, 38)
	rand.Read(b)
	h := hex.EncodeToString(b)
	return fmt.Sprintf("sk-ant-api03-%s-%s-%s-AA", h[:40], h[40:52], h[52:76])
}

func genOAuthCode() string {
	b := make([]byte, 32)
	rand.Read(b)
	return hex.EncodeToString(b)
}

func genOAuthToken(prefix string) string {
	b1 := make([]byte, 16)
	b2 := make([]byte, 8)
	rand.Read(b1)
	rand.Read(b2)
	return fmt.Sprintf("%s-%s-%s", prefix, hex.EncodeToString(b1), hex.EncodeToString(b2))
}

func userProfile() map[string]any {
	now := time.Now()
	return map[string]any{
		"uuid":        FakeAccountUUID,
		"id":          FakeAccountUUID,
		"type":        "user",
		"email":       FakeEmail,
		"email_address": FakeEmail,
		"name":        FakeName,
		"full_name":   FakeName,
		"display_name": FakeName,
		"created_at":  now.Add(-365 * 24 * time.Hour).Unix(),
		"chat_enabled": true,
		"memberships": []any{
			map[string]any{
				"organization": orgInfo(),
				"role":         "owner",
			},
		},
	}
}

func orgInfo() map[string]any {
	now := time.Now()
	return map[string]any{
		"id":       FakeOrgID,
		"uuid":     FakeOrgID,
		"type":     "organization",
		"name":     "Default Organization",
		"created_at": now.Add(-30 * 24 * time.Hour).Unix(),
		"settings": map[string]any{
			"tier":                   "scale",
			"claude_console_enabled": true,
		},
		"capabilities":       []string{"api_access", "model_access"},
		"api_disabled_reason": nil,
		"active_flags":       []any{},
		"billing_status":     "active",
	}
}
