package routes

import (
	"crypto/rand"
	"encoding/hex"
	"fmt"
	"net/http"
)

// HandleOAuthAuthorize handles OAuth authorization
func HandleOAuthAuthorize(w http.ResponseWriter, r *http.Request) {
	state := r.URL.Query().Get("state")
	redirectURI := r.URL.Query().Get("redirect_uri")
	code := genOAuthCode()

	if redirectURI != "" {
		separator := "&"
		if !contains(redirectURI, "?") {
			separator = "?"
		}
		http.Redirect(w, r, redirectURI+separator+"code="+code+"&state="+state, 302)
		return
	}
	w.Header().Set("Content-Type", "text/html")
	w.Write([]byte(buildCodePage(code)))
}

// HandleOAuthCodeCallback handles OAuth code callback
func HandleOAuthCodeCallback(w http.ResponseWriter, r *http.Request) {
	code := r.URL.Query().Get("code")
	if code == "" {
		code = genOAuthCode()
	}
	w.Header().Set("Content-Type", "text/html")
	w.Write([]byte(buildCodePage(code)))
}

// HandleGenerateCode generates an OAuth code
func HandleGenerateCode(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "text/plain")
	w.Write([]byte(genOAuthCode()))
}

// HandleOAuthToken handles OAuth token exchange
func HandleOAuthToken(w http.ResponseWriter, r *http.Request) {
	sendJSON(w, map[string]any{
		"access_token":  genOAuthToken("sk-ant-sid01"),
		"token_type":    "Bearer",
		"expires_in":    31536000,
		"refresh_token": genOAuthToken("sk-ant-rt01"),
		"scope":         "org:create_api_key user:profile user:inference user:sessions:claude_code user:mcp_servers",
		"account_uuid":  FAKE_ACCOUNT_UUID,
	})
}

// HandleOAuthHello handles OAuth hello endpoint
func HandleOAuthHello(w http.ResponseWriter, r *http.Request) {
	sendJSON(w, map[string]string{"message": "hello"})
}

func genOAuthCode() string {
	b := make([]byte, 24)
	rand.Read(b)
	return "oauth_" + hex.EncodeToString(b)
}

func genOAuthToken(prefix string) string {
	b := make([]byte, 32)
	rand.Read(b)
	suffix := hex.EncodeToString(b)
	return fmt.Sprintf("%s-%s-%s-%s-%s", prefix, suffix[0:8], suffix[8:12], suffix[12:16], suffix[16:32])
}

func contains(s, substr string) bool {
	return len(s) >= len(substr) && (s == substr || len(substr) == 0 ||
		(len(s) > 0 && indexOf(s, substr) >= 0))
}

func indexOf(s, substr string) int {
	for i := 0; i <= len(s)-len(substr); i++ {
		if s[i:i+len(substr)] == substr {
			return i
		}
	}
	return -1
}

const FAKE_ACCOUNT_UUID = "acct_1234567890abcdefghijklmnop"

func buildCodePage(code string) string {
	return fmt.Sprintf(`<!DOCTYPE html>
<html><head><meta charset="UTF-8"><title>Authorization Successful</title>
<style>body{font-family:system-ui;display:flex;justify-content:center;align-items:center;min-height:100vh;margin:0;background:#f7f7f8}
.card{background:white;padding:2.5rem;border-radius:12px;box-shadow:0 2px 16px rgba(0,0,0,0.08);text-align:center;max-width:480px}
h2{color:#1a1a2e;margin-bottom:0.5rem}.code-box{padding:1rem;background:#f0f0f5;border-radius:8px;font-family:monospace;margin:1rem;cursor:pointer}
button{padding:0.75rem 2rem;background:#5436DA;color:white;border:none;border-radius:8px;cursor:pointer}
</style></head><body><div class="card">
<h2>✓ Authorization Successful</h2><p>Copy this code and paste it in your terminal</p>
<code class="code-box" onclick="navigator.clipboard.writeText(this.textContent)">%s</code>
<button onclick="navigator.clipboard.writeText(document.querySelector('.code-box').textContent);this.textContent='✓ Copied!'">📋 Copy Code</button>
</div></body></html>`, code)
}
