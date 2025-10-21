package main

import (
	"flag"
	"fmt"
	"log"
	"net"
	"os"
	"path/filepath"

	pb "github.com/forgequant/mcp-trader/providers/hello-go/internal/pb"
	"github.com/forgequant/mcp-trader/providers/hello-go/internal/server"
	"google.golang.org/grpc"
)

func main() {
	port := flag.Int("port", 50051, "gRPC server port")
	schemaDir := flag.String("schema-dir", "../../pkg/schemas", "Path to JSON schema directory")
	flag.Parse()

	// Resolve schema directory to absolute path
	absSchemaDir, err := filepath.Abs(*schemaDir)
	if err != nil {
		log.Fatalf("Failed to resolve schema directory: %v", err)
	}

	// Check if schema directory exists
	if _, err := os.Stat(absSchemaDir); os.IsNotExist(err) {
		log.Fatalf("Schema directory does not exist: %s", absSchemaDir)
	}

	// Create provider server
	providerServer, err := server.NewProviderServer(absSchemaDir)
	if err != nil {
		log.Fatalf("Failed to create provider server: %v", err)
	}

	// Create gRPC server
	grpcServer := grpc.NewServer()
	pb.RegisterProviderServer(grpcServer, providerServer)

	// Start listening
	address := fmt.Sprintf(":%d", *port)
	listener, err := net.Listen("tcp", address)
	if err != nil {
		log.Fatalf("Failed to listen on %s: %v", address, err)
	}

	log.Printf("hello-go provider listening on %s", address)
	log.Printf("Schema directory: %s", absSchemaDir)

	if err := grpcServer.Serve(listener); err != nil {
		log.Fatalf("Failed to serve: %v", err)
	}
}
