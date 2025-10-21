package capabilities

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"

	pb "github.com/forgequant/mcp-trader/providers/hello-go/internal/pb"
)

const ProviderVersion = "0.1.0"

// Builder constructs provider capabilities from schema files
type Builder struct {
	schemaDir string
}

// NewBuilder creates a new capabilities builder
func NewBuilder(schemaDir string) *Builder {
	return &Builder{
		schemaDir: schemaDir,
	}
}

// Build constructs the Capabilities response
func (b *Builder) Build() (*pb.Capabilities, error) {
	// Load echo tool schema
	echoSchema, err := b.loadSchema("echo.input.schema.json")
	if err != nil {
		return nil, fmt.Errorf("failed to load echo schema: %w", err)
	}

	// Load sum tool schema
	sumSchema, err := b.loadSchema("sum.input.schema.json")
	if err != nil {
		return nil, fmt.Errorf("failed to load sum schema: %w", err)
	}

	capabilities := &pb.Capabilities{
		Tools: []*pb.Tool{
			{
				Name:        "echo.v1",
				Description: "Echoes back the provided message",
				InputSchema: &pb.Json{
					Value: echoSchema,
				},
			},
			{
				Name:        "sum.v1",
				Description: "Calculates the sum of an array of numbers",
				InputSchema: &pb.Json{
					Value: sumSchema,
				},
			},
		},
		Resources:       []*pb.Resource{},
		Prompts:         []*pb.Prompt{},
		ProviderVersion: ProviderVersion,
	}

	return capabilities, nil
}

// loadSchema reads a JSON schema file and returns it as bytes
func (b *Builder) loadSchema(filename string) ([]byte, error) {
	path := filepath.Join(b.schemaDir, filename)
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, fmt.Errorf("failed to read schema file %s: %w", path, err)
	}

	// Validate it's valid JSON
	var schemaObj map[string]any
	if err := json.Unmarshal(data, &schemaObj); err != nil {
		return nil, fmt.Errorf("invalid JSON in schema file %s: %w", path, err)
	}

	return data, nil
}
