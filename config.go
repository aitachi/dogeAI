package main

import (
	"crypto/rand"
	"encoding/hex"
	"encoding/json"
	"log"
	"os"
	"path/filepath"
)

type Config struct {
	APIBase            string            `json:"api_base"`
	APIKey             string            `json:"api_key"`
	ModelMap           map[string]string `json:"model_map"`
	MaxOutputTokensLimit map[string]int  `json:"max_output_tokens_limit"`
	DefaultModel       string            `json:"default_model"`
	DefaultMaxTokens   int               `json:"default_max_tokens"`
	DefaultParams      map[string]any    `json:"default_params"`
	HTTPPort           int               `json:"http_port"`
	HTTPSPort          int               `json:"https_port"`
}

var (
	Cfg             *Config
	FakeAccountUUID string
	FakeOrgID       string
	FakeEmail       string
	FakeName        string
)

func init() {
	log.SetFlags(log.Ldate | log.Ltime | log.Lshortfile)

	configPath := filepath.Join(os.Getenv("GOPATH"), "src", "dogeai", "config.json")
	if wd, err := os.Getwd(); err == nil {
		configPath = filepath.Join(wd, "config.json")
	}

	data, err := os.ReadFile(configPath)
	if err != nil {
		log.Fatalf("Failed to read config: %v", err)
	}

	if err := json.Unmarshal(data, &Cfg); err != nil {
		log.Fatalf("Failed to parse config: %v", err)
	}

	FakeAccountUUID = "user_" + randHex(24)
	FakeOrgID = "org_proxy_default"
	FakeEmail = "proxy@local.dev"
	FakeName = "Proxy User"

	log.Printf("Target: %s", Cfg.APIBase)
	log.Printf("Default model: %s", Cfg.DefaultModel)
	log.Printf("HTTP port: %d", Cfg.HTTPPort)
	log.Printf("HTTPS port: %d", Cfg.HTTPSPort)
}

func randHex(n int) string {
	b := make([]byte, n/2)
	rand.Read(b)
	return hex.EncodeToString(b)[:n]
}
