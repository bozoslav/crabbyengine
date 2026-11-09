### Added
- Retained compatibility baselines for the MCP tool contract, the native ABI
  and prefab files (the world save is loaded as a scene too), with tests that compare the current server, header and struct layouts
  against them. New MCP tools, optional parameters, C functions and enum
  constants pass; a removed or changed tool, parameter, function or struct
  layout, a new required parameter, or a parameter that accepts fewer values
  fails with a focused message. A C client compiled against the retained
  version-2 header runs against the current library in the native CI job.
  The baselines come from the engine's main branch, not from a published
  release. Compatibility promises are unchanged.
