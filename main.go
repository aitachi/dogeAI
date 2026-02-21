package main

import (
	"bufio"
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"log"
	"net/http"
	"strings"
	"time"

	"github.com/gin-gonic/gin"
)

func anthropicHeaders() map[string]string {
	reqID := genRequestID()
	return map[string]string{
		"x-request-id":                          reqID,
		"request-id":                            reqID,
		"anthropic-version":                     "2023-06-01",
		"x-anthropic-ratelimit-requests-limit":  "10000",
		"x-anthropic-ratelimit-requests-remaining": "9999",
		"x-anthropic-ratelimit-requests-reset":     "2026-12-31T23:59:59Z",
		"x-anthropic-ratelimit-tokens-limit":      "1000000",
		"x-anthropic-ratelimit-tokens-remaining":  "999999",
		"x-anthropic-ratelimit-tokens-reset":      "2026-12-31T23:59:59Z",
	}
}

func setHeaders(c *gin.Context) {
	h := anthropicHeaders()
	for k, v := range h {
		c.Header(k, v)
	}
}

func logRequest(c *gin.Context) {
	path := c.Request.URL.Path
	query := c.Request.URL.RawQuery
	if query != "" {
		if len(query) > 80 {
			query = query[:80]
		}
		query = "?" + query
	}
	auth := c.GetHeader("authorization")
	xkey := c.GetHeader("x-api-key")
	if len(auth) > 50 {
		auth = auth[:50] + "..."
	}
	if len(xkey) > 50 {
		xkey = xkey[:50] + "..."
	}
	log.Printf(">>> %s %s%s | Host=%s | Auth=%s | x-api-key=%s",
		c.Request.Method, path, query, c.GetHeader("host"), auth, xkey)
}

func main() {
	certFile, keyFile, err := ensureSSLCert()
	if err != nil {
		log.Printf("SSL cert error: %v (running HTTP only)", err)
		certFile = ""
		keyFile = ""
	}

	gin.SetMode(gin.ReleaseMode)
	r := gin.New()
	r.Use(gin.Recovery(), loggerMiddleware(), corsMiddleware())

	r.GET("/", healthCheck)
	r.GET("/v1/models", listModels)
	r.POST("/v1/messages", handleMessages)
	r.GET("/v1/models/:path", getModel)
	r.GET("/api/hello", apiHello)
	r.GET("/v1/oauth/hello", oauthHello)
	r.GET("/api/bootstrap", apiBootstrap)
	r.GET("/api/auth", apiAuth)
	r.GET("/api/auth/session", apiAuthSession)
	r.GET("/api/account", apiAccount)
	r.GET("/api/organizations", apiOrganizations)
	r.POST("/api/organizations/:org_id/api_keys", createAPIKey)
	r.POST("/api/organizations/:org_id/api-keys", createAPIKey)
	r.GET("/.well-known/openid-configuration", openidConfig)
	r.GET("/.well-known/oauth-authorization-server", oauthMetadata)
	r.GET("/userinfo", getUserinfo)
	r.POST("/userinfo", getUserinfo)
	r.GET("/api/me", getAPIMe)
	r.POST("/api/me", getAPIMe)
	r.GET("/v1/me", getV1Me)
	r.POST("/v1/me", getV1Me)
	r.POST("/oauth/token", oauthToken)
	r.GET("/oauth/authorize", oauthAuthorize)
	r.GET("/oauth/code/callback", oauthCodeCallback)
	r.GET("/generate-code", generateCode)
	r.GET("/api/claude_code/settings", claudeCodeSettings)
	r.GET("/api/claude_code/policy_limits", claudeCodePolicyLimits)
	r.GET("/api/claude_code/penguin_mode", claudeCodePenguinMode)
	r.POST("/api/report", silentOK)
	r.POST("/api/telemetry", silentOK)
	r.POST("/api/events", silentOK)
	r.POST("/api/statsig", silentOK)
	r.GET("/v1/dashboard/billing/usage", billingUsage)
	r.GET("/v1/usage", usageEndpoint)
	r.POST("/v1/usage", usageEndpoint)
	r.GET("/v1/organizations/:path", handleOrganization)
	r.POST("/v1/organizations/:org_id/api_keys", createAPIKey)
	r.POST("/v1/messages/count_tokens", countTokens)
	r.POST("/v1/messages/batches", batchesEndpoint)
	r.POST("/v1/complete", handleComplete)
	r.GET("/api/settings", func(c *gin.Context) {
		log.Printf("DEBUG: apiSettings called")
		c.JSON(http.StatusOK, gin.H{})
	})
	r.Any("/:path", catchAll)

	go func() {
		log.Printf("HTTP server starting on :%d", Cfg.HTTPPort)
		if err := r.Run(fmt.Sprintf(":%d", Cfg.HTTPPort)); err != nil {
			log.Printf("HTTP server error: %v", err)
		}
	}()

	if certFile != "" {
		log.Printf("HTTPS server starting on :%d", Cfg.HTTPSPort)
		if err := r.RunTLS(fmt.Sprintf(":%d", Cfg.HTTPSPort), certFile, keyFile); err != nil {
			log.Printf("HTTPS server error: %v", err)
		}
	} else {
		select {}
	}
}

func loggerMiddleware() gin.HandlerFunc {
	return func(c *gin.Context) {
		logRequest(c)
		c.Next()
		log.Printf("<<< %s %s -> %d", c.Request.Method, c.Request.URL.Path, c.Writer.Status())
	}
}

func corsMiddleware() gin.HandlerFunc {
	return func(c *gin.Context) {
		c.Header("Access-Control-Allow-Origin", "*")
		c.Header("Access-Control-Allow-Methods", "*")
		c.Header("Access-Control-Allow-Headers", "*")

		if c.Request.Method == "OPTIONS" {
			c.AbortWithStatus(http.StatusOK)
			return
		}
		c.Next()
	}
}

func healthCheck(c *gin.Context) {
	c.JSON(http.StatusOK, gin.H{"status": "ok", "proxy": "claude-code-to-qwen"})
}

func listModels(c *gin.Context) {
	var models []gin.H
	for name := range Cfg.ModelMap {
		models = append(models, gin.H{
			"id":       name,
			"object":   "model",
			"created":  time.Now().Add(-24 * time.Hour).Unix(),
			"owned_by": "anthropic",
		})
	}
	c.JSON(http.StatusOK, gin.H{"object": "list", "data": models})
}

func getModel(c *gin.Context) {
	c.JSON(http.StatusOK, gin.H{
		"id":       c.Param("path"),
		"object":   "model",
		"created":  time.Now().Add(-24 * time.Hour).Unix(),
		"owned_by": "anthropic",
	})
}

func handleMessages(c *gin.Context) {
	setHeaders(c)

	var body map[string]any
	if err := c.BindJSON(&body); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{
			"type":  "error",
			"error": gin.H{"type": "invalid_request_error", "message": err.Error()},
		})
		return
	}

	originalModel := body["model"].(string)
	if originalModel == "" {
		originalModel = "claude-sonnet-4-5-20250929"
	}
	isStream := body["stream"] != false

	log.Printf("Request: model=%s stream=%v", originalModel, isStream)

	openAIReq := buildOpenAIRequest(body)

	reqBody, _ := json.Marshal(openAIReq)
	req, _ := http.NewRequest("POST", Cfg.APIBase+"/chat/completions", bytes.NewReader(reqBody))
	req.Header.Set("Authorization", "Bearer "+Cfg.APIKey)
	req.Header.Set("Content-Type", "application/json")

	client := &http.Client{Timeout: 300 * time.Second}
	resp, err := client.Do(req)
	if err != nil {
		c.JSON(http.StatusBadGateway, gin.H{
			"type":  "error",
			"error": gin.H{"type": "api_error", "message": err.Error()},
		})
		return
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		body, _ := io.ReadAll(resp.Body)
		c.JSON(resp.StatusCode, gin.H{
			"type":  "error",
			"error": gin.H{"type": "api_error", "message": string(body)},
		})
		return
	}

	if isStream {
		c.Header("Content-Type", "text/event-stream")
		c.Header("Cache-Control", "no-cache")
		c.Header("Connection", "keep-alive")
		streamSSE(c, resp.Body, originalModel)
	} else {
		var openAIResp map[string]any
		json.NewDecoder(resp.Body).Decode(&openAIResp)
		anthropicResp := buildAnthropicResponse(openAIResp, originalModel)
		c.JSON(http.StatusOK, anthropicResp)
	}
}

func streamSSE(c *gin.Context, body io.Reader, originalModel string) {
	flusher, ok := c.Writer.(http.Flusher)
	if !ok {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "Streaming not supported"})
		return
	}

	msgID := genID("msg")
	fmt.Fprintf(c.Writer, "event: message_start\ndata: %s\n\n",
		toJSON(map[string]any{
			"type": "message_start",
			"message": map[string]any{
				"id":      msgID,
				"type":    "message",
				"role":    "assistant",
				"content": []any{},
				"model":   originalModel,
				"stop_reason": nil,
				"stop_sequence": nil,
				"usage": map[string]any{
					"input_tokens":                0,
					"output_tokens":               0,
					"cache_creation_input_tokens": 0,
					"cache_read_input_tokens":     0,
				},
			},
		}))
	fmt.Fprintf(c.Writer, "event: ping\ndata: {\"type\":\"ping\"}\n\n")
	flusher.Flush()

	scanner := bufio.NewScanner(body)
	textOpen := false
	textIdx := 0
	nextIdx := 0
	toolBlocks := make(map[int]int)
	closed := make(map[int]bool)
	finishReason := ""
	_ = finishReason
	outputTokens := 0

	for scanner.Scan() {
		line := strings.TrimSpace(scanner.Text())
		if line == "" {
			continue
		}
		if !strings.HasPrefix(line, "data:") {
			continue
		}
		payload := strings.TrimSpace(strings.TrimPrefix(line, "data:"))
		if payload == "[DONE]" {
			break
		}

		var chunk map[string]any
		if err := json.Unmarshal([]byte(payload), &chunk); err != nil {
			continue
		}

		choices, _ := chunk["choices"].([]any)
		if len(choices) == 0 {
			if usage, ok := chunk["usage"].(map[string]any); ok {
				if ct, ok := usage["completion_tokens"].(float64); ok {
					outputTokens = int(ct)
				}
			}
			continue
		}

		choice := choices[0].(map[string]any)
		delta, _ := choice["delta"].(map[string]any)

		if textPiece, ok := delta["content"].(string); ok && textPiece != "" {
			if !textOpen {
				textIdx = nextIdx
				nextIdx++
				fmt.Fprintf(c.Writer, "event: content_block_start\ndata: %s\n\n",
					toJSON(map[string]any{
						"type":          "content_block_start",
						"index":         textIdx,
						"content_block": map[string]any{"type": "text", "text": ""},
					}))
				textOpen = true
			}
			fmt.Fprintf(c.Writer, "event: content_block_delta\ndata: %s\n\n",
				toJSON(map[string]any{
					"type":  "content_block_delta",
					"index": textIdx,
					"delta": map[string]any{"type": "text_delta", "text": textPiece},
				}))
			flusher.Flush()
		}

		if toolCalls, ok := delta["tool_calls"].([]any); ok {
			for _, tc := range toolCalls {
				tcm := tc.(map[string]any)
				tcIdx := int(tcm["index"].(float64))

				if _, exists := toolBlocks[tcIdx]; !exists {
					if textOpen && !closed[textIdx] {
						fmt.Fprintf(c.Writer, "event: content_block_stop\ndata: {\"type\":\"content_block_stop\",\"index\":%d}\n\n", textIdx)
						closed[textIdx] = true
						textOpen = false
					}

					bi := nextIdx
					nextIdx++
					toolID, _ := tcm["id"].(string)
					if toolID == "" {
						toolID = genID("toolu")
					}
					fn := tcm["function"].(map[string]any)
					name, _ := fn["name"].(string)

					fmt.Fprintf(c.Writer, "event: content_block_start\ndata: %s\n\n",
						toJSON(map[string]any{
							"type":  "content_block_start",
							"index": bi,
							"content_block": map[string]any{
								"type":  "tool_use",
								"id":    toolID,
								"name":  name,
								"input": map[string]any{},
							},
						}))
					toolBlocks[tcIdx] = bi
					flusher.Flush()
				}

				if args, ok := tcm["function"].(map[string]any)["arguments"].(string); ok && args != "" {
					bi := toolBlocks[tcIdx]
					fmt.Fprintf(c.Writer, "event: content_block_delta\ndata: %s\n\n",
						toJSON(map[string]any{
							"type":  "content_block_delta",
							"index": bi,
							"delta": map[string]any{"type": "input_json_delta", "partial_json": args},
						}))
					flusher.Flush()
				}
			}
		}

		if fr, ok := choice["finish_reason"].(string); ok {
			finishReason = fr
		}
	}

	if nextIdx == 0 {
		fmt.Fprintf(c.Writer, "event: content_block_start\ndata: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\n")
		fmt.Fprintf(c.Writer, "event: content_block_stop\ndata: {\"type\":\"content_block_stop\",\"index\":0}\n\n")
	} else {
		if textOpen && !closed[textIdx] {
			fmt.Fprintf(c.Writer, "event: content_block_stop\ndata: {\"type\":\"content_block_stop\",\"index\":%d}\n\n", textIdx)
		}
		for _, bi := range toolBlocks {
			if !closed[bi] {
				fmt.Fprintf(c.Writer, "event: content_block_stop\ndata: {\"type\":\"content_block_stop\",\"index\":%d}\n\n", bi)
			}
		}
	}

	stopReason := mapFinishReason(finishReason)
	fmt.Fprintf(c.Writer, "event: message_delta\ndata: %s\n\n",
		toJSON(map[string]any{
			"type":  "message_delta",
			"delta": map[string]any{"stop_reason": stopReason, "stop_sequence": nil},
			"usage": map[string]any{"output_tokens": outputTokens},
		}))
	fmt.Fprintf(c.Writer, "event: message_stop\ndata: {\"type\":\"message_stop\"}\n\n")
	flusher.Flush()
}

func apiHello(c *gin.Context) {
	c.JSON(http.StatusOK, gin.H{"message": "hello"})
}

func oauthHello(c *gin.Context) {
	c.JSON(http.StatusOK, gin.H{"message": "hello"})
}

func apiBootstrap(c *gin.Context) {
	c.JSON(http.StatusOK, gin.H{
		"account":       userProfile(),
		"organizations": []any{orgInfo()},
		"statsig":       map[string]any{},
	})
}

func apiAuth(c *gin.Context) {
	c.JSON(http.StatusOK, gin.H{
		"account":       userProfile(),
		"account_flags": []any{},
	})
}

func apiAuthSession(c *gin.Context) {
	c.JSON(http.StatusOK, gin.H{
		"account":       userProfile(),
		"session":       gin.H{"id": genID("sess"), "expires_at": time.Now().Add(30 * 24 * time.Hour).Format(time.RFC3339) + "Z"},
		"account_flags": []any{},
	})
}

func apiAccount(c *gin.Context) {
	c.JSON(http.StatusOK, userProfile())
}

func apiOrganizations(c *gin.Context) {
	c.JSON(http.StatusOK, gin.H{"data": []any{orgInfo()}})
}

func createAPIKey(c *gin.Context) {
	setHeaders(c)
	c.JSON(http.StatusOK, gin.H{
		"id":         genID("apikey"),
		"type":       "api_key",
		"api_key":    genFakeAPIKey(),
		"name":       "claude-code-session-key",
		"created_at": time.Now().Unix(),
		"status":     "active",
	})
}

func openidConfig(c *gin.Context) {
	base := getBaseURL(c)
	c.JSON(http.StatusOK, gin.H{
		"issuer":                           base,
		"authorization_endpoint":           base + "/oauth/authorize",
		"token_endpoint":                   base + "/oauth/token",
		"userinfo_endpoint":                base + "/userinfo",
		"response_types_supported":         []string{"code"},
		"grant_types_supported":            []string{"authorization_code", "refresh_token"},
		"code_challenge_methods_supported": []string{"S256"},
		"token_endpoint_auth_methods_supported": []string{"none"},
	})
}

func oauthMetadata(c *gin.Context) {
	base := getBaseURL(c)
	c.JSON(http.StatusOK, gin.H{
		"issuer":                           base,
		"authorization_endpoint":           base + "/oauth/authorize",
		"token_endpoint":                   base + "/oauth/token",
		"response_types_supported":         []string{"code"},
		"grant_types_supported":            []string{"authorization_code", "refresh_token"},
		"code_challenge_methods_supported": []string{"S256"},
		"token_endpoint_auth_methods_supported": []string{"none"},
	})
}

func getUserinfo(c *gin.Context) {
	c.JSON(http.StatusOK, userProfile())
}

func getAPIMe(c *gin.Context) {
	c.JSON(http.StatusOK, userProfile())
}

func getV1Me(c *gin.Context) {
	c.JSON(http.StatusOK, userProfile())
}

func oauthToken(c *gin.Context) {
	c.JSON(http.StatusOK, gin.H{
		"access_token":  genOAuthToken("sk-ant-sid01"),
		"token_type":    "Bearer",
		"expires_in":    31536000,
		"refresh_token": genOAuthToken("sk-ant-rt01"),
		"scope":         "org:create_api_key user:profile user:inference user:sessions:claude_code user:mcp_servers",
		"account_uuid":  FakeAccountUUID,
	})
}

func oauthAuthorize(c *gin.Context) {
	code := genOAuthCode()
	state := c.Query("state")
	redirectURI := c.Query("redirect_uri")

	if redirectURI != "" {
		sep := "&"
		if !strings.Contains(redirectURI, "?") {
			sep = "?"
		}
		c.Redirect(http.StatusFound, redirectURI+sep+"code="+code+"&state="+state)
		return
	}

	c.HTML(http.StatusOK, "", buildCodePage(code))
}

func oauthCodeCallback(c *gin.Context) {
	code := c.Query("code")
	if code == "" {
		code = genOAuthCode()
	}
	c.HTML(http.StatusOK, "", buildCodePage(code))
}

func generateCode(c *gin.Context) {
	c.String(http.StatusOK, genOAuthCode())
}

func claudeCodeSettings(c *gin.Context) {
	c.JSON(http.StatusOK, gin.H{
		"expiry":          nil,
		"isolated":        false,
		"allowed_tools":   []string{"computer", "text_editor", "bash"},
		"max_turns":       nil,
		"internet_policy": "allow",
	})
}

func claudeCodePolicyLimits(c *gin.Context) {
	c.JSON(http.StatusOK, gin.H{
		"rate_limits": gin.H{
			"requests_per_minute": 60,
			"tokens_per_minute":   1000000,
			"tokens_per_day":      50000000,
		},
		"usage": gin.H{
			"tokens_used_today": 0,
			"requests_today":    0,
		},
		"limits": gin.H{},
	})
}

func claudeCodePenguinMode(c *gin.Context) {
	c.JSON(http.StatusOK, gin.H{"enabled": false})
}

func silentOK(c *gin.Context) {
	c.JSON(http.StatusOK, gin.H{"ok": true})
}

func handleOrganization(c *gin.Context) {
	setHeaders(c)
	c.JSON(http.StatusOK, orgInfo())
}

func countTokens(c *gin.Context) {
	var body map[string]any
	c.BindJSON(&body)
	messages, _ := json.Marshal(body["messages"])
	estimated := max(1, len(messages)/4)
	c.JSON(http.StatusOK, gin.H{"input_tokens": estimated})
}

func handleComplete(c *gin.Context) {
	var body map[string]any
	c.BindJSON(&body)
	prompt, _ := body["prompt"].(string)
	body["messages"] = []map[string]any{{"role": "user", "content": prompt}}
	body["stream"] = false
	c.Request.Body = nil
	handleMessages(c)
}

func catchAll(c *gin.Context) {
	setHeaders(c)
	c.JSON(http.StatusOK, gin.H{
		"status": "ok",
		"type":   "proxy_catch_all",
		"path":   "/" + c.Param("path"),
	})
}

func apiSettings(c *gin.Context) {
	c.JSON(http.StatusOK, gin.H{})
}

func billingUsage(c *gin.Context) {
	setHeaders(c)
	c.JSON(http.StatusOK, gin.H{
		"daily_costs":  []any{},
		"total_usage":  0.0,
	})
}

func usageEndpoint(c *gin.Context) {
	setHeaders(c)
	c.JSON(http.StatusOK, gin.H{
		"daily_costs": []any{},
		"total_usage": 0,
	})
}

func batchesEndpoint(c *gin.Context) {
	setHeaders(c)
	c.JSON(http.StatusNotFound, gin.H{
		"type":  "error",
		"error": gin.H{
			"type":    "not_found_error",
			"message": "Batches not available",
		},
	})
}

func getBaseURL(c *gin.Context) string {
	scheme := "http"
	if c.Request.TLS != nil {
		scheme = "https"
	}
	return scheme + "://" + c.Request.Host
}

func buildCodePage(code string) string {
	return fmt.Sprintf(`<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Authorization Successful</title>
<style>
  body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
         display: flex; justify-content: center; align-items: center;
         min-height: 100vh; margin: 0; background: #f7f7f8; }
  .card { background: white; padding: 2.5rem; border-radius: 12px;
           box-shadow: 0 2px 16px rgba(0,0,0,0.08); text-align: center; max-width: 480px; }
  h2 { color: #1a1a2e; margin-bottom: 0.5rem; }
  .subtitle { color: #666; margin-bottom: 1.5rem; }
  .code-box { display: block; padding: 1rem 1.5rem; background: #f0f0f5;
               border-radius: 8px; font-family: 'SF Mono', Monaco, monospace;
               font-size: 0.85rem; word-break: break-all; margin: 1rem 0;
               cursor: pointer; border: 2px solid transparent; transition: border-color 0.2s; }
  .code-box:hover { border-color: #5436DA; }
  button { padding: 0.75rem 2rem; background: #5436DA; color: white;
            border: none; border-radius: 8px; cursor: pointer; font-size: 1rem;
            font-weight: 500; transition: background 0.2s; }
  button:hover { background: #4128b0; }
  .hint { color: #888; font-size: 0.85rem; margin-top: 1rem; }
  .success { color: #22c55e; font-size: 0.85rem; display: none; margin-top: 0.5rem; }
</style>
</head>
<body>
<div class="card">
  <h2>✓ Authorization Successful</h2>
  <p class="subtitle">Copy this code and paste it in your terminal</p>
  <code class="code-box" id="code" onclick="copyCode()">%s</code>
  <button onclick="copyCode()">📋 Copy Code</button>
  <p class="success" id="success">Copied! Now paste it in your terminal.</p>
  <p class="hint">Click the code or button to copy</p>
</div>
<script>
function copyCode() {
  const code = document.getElementById('code').textContent;
  navigator.clipboard.writeText(code).then(() => {
    document.getElementById('success').style.display = 'block';
    document.querySelector('button').textContent = '✓ Copied!';
    document.querySelector('button').style.background = '#22c55e';
  }).catch(() => {
    const range = document.createRange();
    range.selectNode(document.getElementById('code'));
    window.getSelection().removeAllRanges();
    window.getSelection().addRange(range);
  });
}
</script>
</body>
</html>`, code)
}

func min(a, b int) int {
	if a < b {
		return a
	}
	return b
}

func max(a, b int) int {
	if a > b {
		return a
	}
	return b
}

func toJSON(v any) string {
	b, _ := json.Marshal(v)
	return string(b)
}
