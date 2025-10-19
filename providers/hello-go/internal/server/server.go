package server

import (
	"context"
	"encoding/json"
	"fmt"
	"log"

	"github.com/forgetrade/mcp-trader/providers/hello-go/internal/capabilities"
	pb "github.com/forgetrade/mcp-trader/providers/hello-go/internal/pb"
	"github.com/forgetrade/mcp-trader/providers/hello-go/internal/tools"
	"google.golang.org/protobuf/types/known/emptypb"
)

// Tool defines the interface all tools must implement
type Tool interface {
	Name() string
	Description() string
	Invoke(payload []byte) ([]byte, error)
}

// ProviderServer implements the Provider gRPC service
type ProviderServer struct {
	pb.UnimplementedProviderServer
	capabilities *pb.Capabilities
	tools        map[string]Tool
}

// NewProviderServer creates a new provider server
func NewProviderServer(schemaDir string) (*ProviderServer, error) {
	// Build capabilities from schema files
	builder := capabilities.NewBuilder(schemaDir)
	caps, err := builder.Build()
	if err != nil {
		return nil, fmt.Errorf("failed to build capabilities: %w", err)
	}

	// Register tools
	toolRegistry := make(map[string]Tool)
	echoTool := &tools.Echo{}
	sumTool := &tools.Sum{}

	toolRegistry[echoTool.Name()] = echoTool
	toolRegistry[sumTool.Name()] = sumTool

	return &ProviderServer{
		capabilities: caps,
		tools:        toolRegistry,
	}, nil
}

// ListCapabilities returns all capabilities exposed by this provider
func (s *ProviderServer) ListCapabilities(ctx context.Context, req *emptypb.Empty) (*pb.Capabilities, error) {
	log.Printf("ListCapabilities called")
	return s.capabilities, nil
}

// Invoke executes a tool with the given arguments
func (s *ProviderServer) Invoke(ctx context.Context, req *pb.InvokeRequest) (*pb.InvokeResponse, error) {
	log.Printf("Invoke called: tool=%s, correlation_id=%s", req.ToolName, req.CorrelationId)

	// Find the tool
	tool, exists := s.tools[req.ToolName]
	if !exists {
		return &pb.InvokeResponse{
			Error: fmt.Sprintf("tool not found: %s", req.ToolName),
		}, nil
	}

	// Invoke the tool
	resultBytes, err := tool.Invoke(req.Payload.Value)
	if err != nil {
		log.Printf("Tool invocation failed: %v", err)
		return &pb.InvokeResponse{
			Error: err.Error(),
		}, nil
	}

	// Validate the result is valid JSON
	var resultObj map[string]any
	if err := json.Unmarshal(resultBytes, &resultObj); err != nil {
		log.Printf("Tool returned invalid JSON: %v", err)
		return &pb.InvokeResponse{
			Error: fmt.Sprintf("tool returned invalid JSON: %v", err),
		}, nil
	}

	return &pb.InvokeResponse{
		Result: &pb.Json{Value: resultBytes},
	}, nil
}

// ReadResource reads a resource by URI (not implemented for hello-go)
func (s *ProviderServer) ReadResource(ctx context.Context, req *pb.ResourceRequest) (*pb.ResourceResponse, error) {
	log.Printf("ReadResource called: uri=%s, correlation_id=%s", req.Uri, req.CorrelationId)
	return &pb.ResourceResponse{
		Error: "resources not supported by hello-go provider",
	}, nil
}

// GetPrompt returns a prompt template (not implemented for hello-go)
func (s *ProviderServer) GetPrompt(ctx context.Context, req *pb.PromptRequest) (*pb.PromptResponse, error) {
	log.Printf("GetPrompt called: prompt_name=%s, correlation_id=%s", req.PromptName, req.CorrelationId)
	return &pb.PromptResponse{
		Error: "prompts not supported by hello-go provider",
	}, nil
}

// Stream streams events from provider (not implemented for hello-go)
func (s *ProviderServer) Stream(req *pb.StreamRequest, stream pb.Provider_StreamServer) error {
	log.Printf("Stream called: topic=%s", req.Topic)
	return fmt.Errorf("streaming not supported by hello-go provider")
}
