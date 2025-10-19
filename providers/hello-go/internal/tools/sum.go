package tools

import (
	"encoding/json"
	"fmt"
)

// Sum implements the sum.v1 tool
type Sum struct{}

// SumInput represents the validated input for the sum tool
type SumInput struct {
	Numbers []float64 `json:"numbers"`
}

// SumOutput represents the output of the sum tool
type SumOutput struct {
	Sum float64 `json:"sum"`
}

// Name returns the tool name
func (s *Sum) Name() string {
	return "sum.v1"
}

// Description returns the tool description
func (s *Sum) Description() string {
	return "Calculates the sum of an array of numbers"
}

// Invoke executes the sum tool
// Input is expected to be pre-validated against sum.input.schema.json
func (s *Sum) Invoke(payloadBytes []byte) ([]byte, error) {
	var input SumInput
	if err := json.Unmarshal(payloadBytes, &input); err != nil {
		return nil, fmt.Errorf("invalid input: %w", err)
	}

	var total float64
	for _, num := range input.Numbers {
		total += num
	}

	output := SumOutput{
		Sum: total,
	}

	resultBytes, err := json.Marshal(output)
	if err != nil {
		return nil, fmt.Errorf("failed to marshal output: %w", err)
	}

	return resultBytes, nil
}
