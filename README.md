# Ferrite

Mod manager for minecraft servers written in rust.

Mod distribution platform support:

- [x] Modrinth
- [x] CurseForge
- [x] GitHub Releases
- [ ] Hangar

Server software support:

- [x] Quilt
- [x] Fabric
- [x] Forge
- [x] NeoForge
- [ ] Paper
- [x] Velocity

## Quick Reference

| Command                        | Alias          | Description                     |
| ------------------------------ | -------------- | ------------------------------- |
| `ferrite init`                 | -              | Initialize a new configuration  |
| `ferrite start`                | -              | Start the Minecraft server      |
| `ferrite add <id>`             | `i`, `install` | Add mod(s) by identifier        |
| `ferrite remove <name>`        | `rm`           | Remove mod(s) by name           |
| `ferrite disable <name>`       | -              | Disable mod(s) by name          |
| `ferrite override <name> <id>` | -              | Override a mod's version/source |
| `ferrite script <name>`        | -              | Run a setup script              |
| `ferrite list`                 | `ls`           | List all installed mods         |
| `ferrite upgrade`              | `update`       | Upgrade all mods to latest      |

## Command Details

### `ferrite init`

Initialize a new `ferrite.yaml` configuration. Without arguments, runs interactively.

```bash
# Interactive mode
ferrite init

# Non-interactive mode
ferrite init -v 1.20.1 -m quilt
ferrite init -v 1.20.1 1.20.2 -m fabric neoforge
```

**Mod loader options:** `quilt`, `fabric`, `forge`, `neoforge`, `velocity`

### `ferrite add`

Add mods by identifier. Supports multiple mods at once.

```bash
ferrite add sodium            # Modrinth slug
ferrite add 123456            # CurseForge project ID
ferrite add CaffeineMC/sodium # GitHub repository
ferrite add sodium lithium    # Multiple mods
```

**Identifier formats:**

- **Modrinth**: Project slug (e.g., `sodium`, `lithium`)
- **CurseForge**: Numeric project ID (e.g., `123456`)
- **GitHub**: `owner/repo` format (e.g., `FabricMC/fabric`)

### `ferrite override`

Override a mod to use a different version or source. Useful for compatibility layers.

```bash
ferrite override sodium sodium-fabric    # Use Modrinth slug
ferrite override sodium 123456           # Use CurseForge ID
ferrite override sodium user/sodium-fork # Use GitHub repo
```

### `ferrite script`

Run predefined setup scripts for common configurations.

```bash
ferrite script setup:quilt   # Quilt compatibility setup
ferrite script setup:sinytra # Sinytra Connector setup for Forge mods on Fabric
```

**Available scripts:**

- `setup:quilt` - Configures Fabric compatibility for Quilt servers
- `setup:sinytra` - Sets up Sinytra Connector for running Forge mods on Fabric

### `ferrite remove` / `ferrite disable`

Both commands accept mod names as they appear in `ferrite.yaml`.

```bash
ferrite remove sodium
ferrite disable lithium
```

- `remove` - Permanently removes the mod
- `disable` - Moves the mod to a disabled list (can be re-enabled manually)

### `ferrite list`

Display all installed mods with their source platform and identifiers.

### `ferrite upgrade`

Check and update all mods to their latest compatible versions.

## Example config

```yaml
# https://github.com/septechx/ferrite/blob/master/schema/ferrite.yaml
version: 4
autoupdate: true
key_store: Pass
output_path: mods
server:
  wrapper: java -Xmx4G -jar {} nogui
  executable: fabric-server-mc.1.21.11-loader.0.18.4-launcher.1.1.1.jar
ferium:
  game_versions:
    - 1.21.11
  mod_loaders:
    - Fabric
  overrides:
    TQTTVgYE: !GitHubRepository
      - gnembon
      - fabric-carpet
  mods:
      slug: carpet-tis-addition
    - name: Fast Backups
      identifier: !ModrinthProject ZHKrK8Rp
      slug: fastback
    - name: Fabric API
      identifier: !ModrinthProject P7dR8mSH
      slug: fabric-api
    - name: fabric-carpet
      identifier: !GitHubRepository
        - gnembon
        - fabric-carpet
      slug: fabric-carpet
    - name: FerriteCore
      identifier: !ModrinthProject uXXizFIs
      slug: ferrite-core
  disabled: []
```
