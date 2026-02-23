package config

import (
	"encoding/json"
	"fmt"
	"os"
)

// Config represents the proxy configuration
type Config struct {
	APIBase    string            `json:"api_base"`
	APIKey     string            `json:"api_key"`
	ModelMap   map[string]string `json:"model_map"`
	MaxOutput  map[string]int    `json:"max_output_tokens_limit"`
	DefaultModel string          `json:"default_model"`
	DefaultMaxTokens int         `json:"default_max_tokens"`
}

// LoadConfig loads the configuration from a JSON file
func LoadConfig() (*Config, error) {
	data, err := os.ReadFile("/root/dogeAI/config.json")
	if err != nil {
		return nil, fmt.Errorf("failed to read config: %w", err)
	}

	var config Config
	if err := json.Unmarshal(data, &config); err != nil {
		return nil, fmt.Errorf("failed to parse config: %w", err)
	}

	return &config, nil
}
