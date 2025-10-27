fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Support both local dev (../../pkg/proto) and Docker build (pkg/proto)
    let provider_proto_path = if std::path::Path::new("../../pkg/proto/provider.proto").exists() {
        "../../pkg/proto/provider.proto"
    } else {
        "pkg/proto/provider.proto"
    };

    let provider_proto_dir = if std::path::Path::new("../../pkg/proto").exists() {
        "../../pkg/proto/"
    } else {
        "pkg/proto/"
    };

    tonic_build::configure()
        .build_server(true)
        .build_client(false) // Provider only needs server stubs
        .compile(&[provider_proto_path], &[provider_proto_dir])?;

    // Compile market-report.proto for unified reporting API
    tonic_build::configure()
        .build_server(true)
        .build_client(false)
        .compile(&["proto/market-report.proto"], &["proto/"])?;

    Ok(())
}
