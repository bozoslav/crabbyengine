//! `ponko-mcp`: MCP server for Crabby Engine games, over stdio.
//!
//!   ponko-mcp              all tools
//!   ponko-mcp --read-only  only tools that don't change games

fn main() {
    let mut config = ponko_mcp::Config::from_env();
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--read-only" => config.read_only = true,
            "--help" | "-h" => {
                println!(
                    "ponko-mcp: MCP server (stdio) for Crabby Engine games\n\n  --read-only  hide tools that change games"
                );
                return;
            }
            other => {
                eprintln!("ponko-mcp: unknown argument `{other}` (try --help)");
                std::process::exit(2);
            }
        }
    }
    if let Err(e) = ponko_mcp::serve_stdio(config) {
        eprintln!("ponko-mcp: {e}");
        std::process::exit(1);
    }
}
