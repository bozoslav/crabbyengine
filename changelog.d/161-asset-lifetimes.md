### Added
- `Graphics::asset_handles(class)` (and `Audio::asset_handles()` for
  sounds) lists every live handle of an `AssetClass`
  with its estimated bytes, the frame it was made on, its age and its
  `AssetOwner` (game code, the asset file that loaded it, or the engine),
  oldest first: after `AssetInventory::growth_since` names the class that
  grows, it shows which handles are piling up and where they came from.
- `Graphics::unload_mesh(path)` frees everything loading a glTF file made:
  its mesh, its materials and its textures, except images another loaded
  file shares.
- C and C++: `PonkoAssetClass`, `PonkoAssetOwner`, `PonkoAssetCount`,
  `PonkoAssetRecord`, `ponko_ctx_asset_count`, `ponko_ctx_asset_handles`,
  `ponko_ctx_asset_owner` and `ponko_ctx_unload_mesh`; in C++
  `ctx::asset_count`, `asset_handles`, `asset_owner_name` and
  `unload_mesh`.
- The guide's handles chapter covers loaded-file caches and finding a
  leak.

### Fixed
- An instance batch whose mesh was freed, and whose mesh slot was then
  given to a new mesh, drew the new mesh. A batch with a freed mesh now
  draws nothing from the next frame on, and draws again if that mesh comes
  back under its id (an evicted resident mesh reloaded).
- Freeing one material of a loaded glTF file and loading the file again
  made a whole new set of its materials and textures, leaving the old ones
  live and unreachable. Only what was freed is made again now.
