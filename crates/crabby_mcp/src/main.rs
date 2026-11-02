//! `crabby-mcp`: MCP server for Crabby Engine games, over stdio.
//!
//!   crabby-mcp              all tools
//!   crabby-mcp --read-only  only tools that don't change games

fn main() {
    let mut config = crabby_mcp::Config::from_env();
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--read-only" => config.read_only = true,
            "--help" | "-h" => {
                println!(
                    "crabby-mcp: MCP server (stdio) for Crabby Engine games\n\n  --read-only  hide tools that change games"
                );
                return;
            }
            other => {
                eprintln!("crabby-mcp: unknown argument `{other}` (try --help)");
                std::process::exit(2);
            }
        }
    }
    if let Err(e) = crabby_mcp::serve_stdio(config) {
        eprintln!("crabby-mcp: {e}");
        std::process::exit(1);
    }
}
