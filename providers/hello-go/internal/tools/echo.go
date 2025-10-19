package tools

import (
	"encoding/json"
	"fmt"
)

// Echo implements the echo.v1 tool
type Echo struct{}

// EchoInput represents the validated input for the echo tool
type EchoInput struct {
	Message string `json:"message"`
}

// EchoOutput represents the output of the echo tool
type EchoOutput struct {
	Echo string `json:"echo"`
}

// Name returns the tool name
func (e *Echo) Name() string {
	return "echo.v1"
}

// Description returns the tool description
func (e *Echo) Description() string {
	return "Echoes back the provided message"
}

// Invoke executes the echo tool
// Input is expected to be pre-validated against echo.input.schema.json
func (e *Echo) Invoke(payloadBytes []byte) ([]byte, error) {
	var input EchoInput
	if err := json.Unmarshal(payloadBytes, &input); err != nil {
		return nil, fmt.Errorf("invalid input: %w", err)
	}

	output := EchoOutput{
		Echo: input.Message,
	}

	resultBytes, err := json.Marshal(output)
	if err != nil {
		return nil, fmt.Errorf("failed to marshal output: %w", err)
	}

	return resultBytes, nil
}
