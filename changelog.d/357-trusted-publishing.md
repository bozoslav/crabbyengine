### Changed
- The release workflow publishes to crates.io with Trusted Publishing (GitHub
  OIDC and `rust-lang/crates-io-auth-action`): the `CARGO_REGISTRY_TOKEN`
  secret is no longer read and can be deleted. Publishing needs approval of
  the protected `release` environment, so the workflow no longer has to be
  kept disabled between releases. A release is now: merge the release PR,
  push the tag, approve the environment.
- The release workflow's first job also fails unless CI succeeded on the
  tagged commit on `main` or its `release-X.Y` branch
  (`scripts/release-check.py ci`); a run still in progress gets its own
  message. Nothing is built before that.
- The release workflow redeploys the website and guide through a Cloudflare
  Pages deploy hook when the `PONKO_WEBSITE_DEPLOY_HOOK` secret exists.
  CONTRIBUTING.md's "Releasing" lists the settings a maintainer has to make
  once (trusted publishers, the `release` environment, the hook).
