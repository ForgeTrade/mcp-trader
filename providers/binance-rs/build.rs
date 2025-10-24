fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(true)
        .build_client(false) // Provider only needs server stubs
        .compile(&["../../pkg/proto/provider.proto"], &["../../pkg/proto/"])?;

    // Compile market-report.proto for unified reporting API
    tonic_build::configure()
        .build_server(true)
        .build_client(false)
        .compile(&["proto/market-report.proto"], &["proto/"])?;

    Ok(())
}
