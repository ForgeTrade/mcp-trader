fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_client(false)  // Server-only
        .build_server(true)
        .compile(
            &["../../pkg/proto/provider.proto"],
            &["../../pkg/proto/"],
        )?;
    Ok(())
}
