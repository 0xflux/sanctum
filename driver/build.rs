fn main() -> Result<(), wdk_build::ConfigError> {
    println!("Starting build process...");
    wdk_build::configure_wdk_binary_build()
}
