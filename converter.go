package main

import (
	"encoding/json"
	"fmt"
	"log"
)

type AnthropicMessage struct {
	Role    string `json:"role"`
	Content any    `json:"content"`
}

type ContentBlock struct {
	Type      string `json:"type"`
	Text      string `json:"text,omitempty"`
	ID        string `json:"id,omitempty"`
	Name      string `json:"name,omitempty"`
	Input     any    `json:"input,omitempty"`
	ToolUseID string `json:"tool_use_id,omitempty"`
	IsError   bool   `json:"is_error,omitempty"`
}

type OpenAIMessage struct {
	Role       string      `json:"role"`
	Content    string      `json:"content,omitempty"`
	ToolCalls  []ToolCall  `json:"tool_calls,omitempty"`
	ToolCallID string      `json:"tool_call_id,omitempty"`
}

type ToolCall struct {
	ID       string `json:"id"`
	Type     string `json:"type"`
	Function struct {
		Name      string `json:"name"`
		Arguments string `json:"arguments"`
	} `json:"function"`
}

type Tool struct {
	Type     string `json:"type"`
	Function struct {
		Name        string         `json:"name"`
		Description string         `json:"description"`
		Parameters  map[string]any `json:"parameters"`
	} `json:"function"`
}

type OpenAIRequest struct {
	Model       string        `json:"model"`
	Messages    []OpenAIMessage `json:"messages"`
	Stream      bool          `json:"stream"`
	MaxTokens   int           `json:"max_tokens"`
	Temperature *float64      `json:"temperature,omitempty"`
	TopP        *float64      `json:"top_p,omitempty"`
	Tools       []Tool        `json:"tools,omitempty"`
	ToolChoice  any           `json:"tool_choice,omitempty"`
	Stop        []string      `json:"stop,omitempty"`
}

func extractText(content any) string {
	switch v := content.(type) {
	case string:
		return v
	case []any:
		var parts []string
		for _, item := range v {
			if block, ok := item.(map[string]any); ok {
				if t, ok := block["type"].(string); ok && t == "text" {
					if text, ok := block["text"].(string); ok {
						parts = append(parts, text)
					}
				}
			}
		}
		result := ""
		for i, p := range parts {
			if i > 0 {
				result += "\n"
			}
			result += p
		}
		return result
	default:
		return fmt.Sprintf("%v", content)
	}
}

func convertMessages(messages []AnthropicMessage, system any) []OpenAIMessage {
	var result []OpenAIMessage

	if system != nil {
		sysText := extractText(system)
		if sysText != "" {
			result = append(result, OpenAIMessage{Role: "system", Content: sysText})
		}
	}

	for _, msg := range messages {
		role := msg.Role
		if role == "" {
			role = "user"
		}

		content := msg.Content

		if role == "user" {
			if arr, ok := content.([]any); ok {
				var textParts []string
				var toolResults []OpenAIMessage

				for _, item := range arr {
					if block, ok := item.(map[string]any); ok {
						btype, _ := block["type"].(string)

						if btype == "text" {
							if text, ok := block["text"].(string); ok {
								textParts = append(textParts, text)
							}
						} else if btype == "tool_result" {
							tcContent := block["content"]
							var contentStr string

							switch v := tcContent.(type) {
							case string:
								contentStr = v
							case []any:
								var parts []string
								for _, b := range v {
									if m, ok := b.(map[string]any); ok {
										if t, ok := m["type"].(string); ok && t == "text" {
											if text, ok := m["text"].(string); ok {
												parts = append(parts, text)
											}
										}
									}
								}
								for i, p := range parts {
									if i > 0 {
										contentStr += "\n"
									}
									contentStr += p
								}
							default:
								if b, err := json.Marshal(tcContent); err == nil {
									contentStr = string(b)
								}
							}

							if isError, ok := block["is_error"].(bool); ok && isError {
								contentStr = "[ERROR] " + contentStr
							}

							toolResults = append(toolResults, OpenAIMessage{
								Role:       "tool",
								ToolCallID: block["tool_use_id"].(string),
								Content:    contentStr,
							})
						}
					}
				}

				result = append(result, toolResults...)

				combined := ""
				for i, p := range textParts {
					if i > 0 {
						combined += "\n"
					}
					combined += p
				}
				if combined != "" {
					result = append(result, OpenAIMessage{Role: "user", Content: combined})
				}
				if len(toolResults) == 0 && combined == "" {
					result = append(result, OpenAIMessage{Role: "user", Content: ""})
				}
			} else {
				result = append(result, OpenAIMessage{Role: "user", Content: fmt.Sprintf("%v", content)})
			}
		} else if role == "assistant" {
			if arr, ok := content.([]any); ok {
				var textParts []string
				var toolCalls []ToolCall

				for _, item := range arr {
					if block, ok := item.(map[string]any); ok {
						btype, _ := block["type"].(string)

						if btype == "text" {
							if text, ok := block["text"].(string); ok {
								textParts = append(textParts, text)
							}
						} else if btype == "tool_use" {
							id, _ := block["id"].(string)
							if id == "" {
								id = genID("call")
							}
							name, _ := block["name"].(string)
							input := block["input"]

							argsJSON, _ := json.Marshal(input)

							toolCalls = append(toolCalls, ToolCall{
								ID:   id,
								Type: "function",
								Function: struct {
									Name      string `json:"name"`
									Arguments string `json:"arguments"`
								}{
									Name:      name,
									Arguments: string(argsJSON),
								},
							})
						}
					}
				}

				assistantMsg := OpenAIMessage{Role: "assistant"}
				if len(textParts) > 0 {
					content := ""
					for i, p := range textParts {
						if i > 0 {
							content += "\n"
						}
						content += p
					}
					assistantMsg.Content = content
				}
				if len(toolCalls) > 0 {
					assistantMsg.ToolCalls = toolCalls
				}
				result = append(result, assistantMsg)
			} else {
				result = append(result, OpenAIMessage{Role: "assistant", Content: fmt.Sprintf("%v", content)})
			}
		}
	}

	return result
}

func convertTools(tools []any) []Tool {
	if len(tools) == 0 {
		return nil
	}
	var result []Tool
	for _, t := range tools {
		if tool, ok := t.(map[string]any); ok {
			name, _ := tool["name"].(string)
			desc, _ := tool["description"].(string)
			inputSchema := tool["input_schema"]
			if inputSchema == nil {
				inputSchema = map[string]any{"type": "object", "properties": map[string]any{}}
			}

			result = append(result, Tool{
				Type: "function",
				Function: struct {
					Name        string         `json:"name"`
					Description string         `json:"description"`
					Parameters  map[string]any `json:"parameters"`
				}{
					Name:        name,
					Description: desc,
					Parameters:  inputSchema.(map[string]any),
				},
			})
		}
	}
	return result
}

func convertToolChoice(tc any) any {
	if tc == nil {
		return nil
	}
	if m, ok := tc.(map[string]any); ok {
		t, _ := m["type"].(string)
		if t == "auto" {
			return "auto"
		}
		if t == "any" {
			return "required"
		}
		if t == "tool" {
			name, _ := m["name"].(string)
			return map[string]any{
				"type": "function",
				"function": map[string]string{
					"name": name,
				},
			}
		}
	}
	return "auto"
}

func buildOpenAIRequest(body map[string]any) OpenAIRequest {
	model := mapModel(body["model"].(string))
	messagesAny := body["messages"]
	var messages []AnthropicMessage
	msgBytes, _ := json.Marshal(messagesAny)
	json.Unmarshal(msgBytes, &messages)

	system := body["system"]
	convertedMsgs := convertMessages(messages, system)

	maxTokens := clampMaxTokens(int(body["max_tokens"].(float64)), model)

	req := OpenAIRequest{
		Model:     model,
		Messages:  convertedMsgs,
		Stream:    body["stream"] != false,
		MaxTokens: maxTokens,
	}

	if temp, ok := body["temperature"].(float64); ok {
		req.Temperature = &temp
	} else if v, ok := Cfg.DefaultParams["temperature"].(float64); ok {
		req.Temperature = &v
	}

	if topP, ok := body["top_p"].(float64); ok {
		req.TopP = &topP
	}

	if tools, ok := body["tools"].([]any); ok && len(tools) > 0 {
		convertedTools := convertTools(tools)
		if len(convertedTools) > 0 {
			req.Tools = convertedTools
			if tc := convertToolChoice(body["tool_choice"]); tc != nil {
				req.ToolChoice = tc
			}
		}
	}

	if stopSeqs, ok := body["stop_sequences"].([]any); ok && len(stopSeqs) > 0 {
		var stops []string
		for _, s := range stopSeqs {
			if str, ok := s.(string); ok {
				stops = append(stops, str)
			}
		}
		req.Stop = stops
	}

	log.Printf("Forwarding to model=%s, max_tokens=%d", model, maxTokens)
	return req
}

func mapFinishReason(fr string) string {
	switch fr {
	case "tool_calls", "function_call":
		return "tool_use"
	case "length":
		return "max_tokens"
	default:
		return "end_turn"
	}
}

func buildAnthropicResponse(openaiResp map[string]any, originalModel string) map[string]any {
	if err, ok := openaiResp["error"]; ok {
		return map[string]any{
			"type":  "error",
			"error": err,
		}
	}

	choices, ok := openaiResp["choices"].([]any)
	if !ok || len(choices) == 0 {
		return map[string]any{
			"type":  "error",
			"error": map[string]any{"type": "api_error", "message": "No choices in response"},
		}
	}

	choice := choices[0].(map[string]any)
	message := choice["message"].(map[string]any)

	var content []map[string]any

	if text, ok := message["content"].(string); ok && text != "" {
		content = append(content, map[string]any{"type": "text", "text": text})
	}

	if toolCalls, ok := message["tool_calls"].([]any); ok {
		for _, tc := range toolCalls {
			if tcm, ok := tc.(map[string]any); ok {
				id, _ := tcm["id"].(string)
				if id == "" {
					id = genID("toolu")
				}
				fn := tcm["function"].(map[string]any)
				name, _ := fn["name"].(string)
				argsStr, _ := fn["arguments"].(string)

				var args map[string]any
				json.Unmarshal([]byte(argsStr), &args)

				content = append(content, map[string]any{
					"type":  "tool_use",
					"id":    id,
					"name":  name,
					"input": args,
				})
			}
		}
	}

	if len(content) == 0 {
		content = append(content, map[string]any{"type": "text", "text": ""})
	}

	usage, _ := openaiResp["usage"].(map[string]any)
	inputTokens := int(0)
	outputTokens := int(0)
	if usage != nil {
		if v, ok := usage["prompt_tokens"].(float64); ok {
			inputTokens = int(v)
		}
		if v, ok := usage["completion_tokens"].(float64); ok {
			outputTokens = int(v)
		}
	}

	finishReason := "end_turn"
	if fr, ok := choice["finish_reason"].(string); ok {
		finishReason = mapFinishReason(fr)
	}

	return map[string]any{
		"id":           genID("msg"),
		"type":         "message",
		"role":         "assistant",
		"content":      content,
		"model":        originalModel,
		"stop_reason":  finishReason,
		"stop_sequence": nil,
		"usage": map[string]any{
			"input_tokens":                  inputTokens,
			"output_tokens":                 outputTokens,
			"cache_creation_input_tokens":   0,
			"cache_read_input_tokens":       0,
		},
	}
}
