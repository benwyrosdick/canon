# Run `just` to list recipes.

[private]
default:
    @just --list

# Run the game (extra args pass through, e.g. `just run --local --seed=42`)
run *args:
    cargo run --bin canon-3d -- {{args}}

# Run an optimized build of the game
run-release *args:
    cargo run --release --locked --bin canon-3d -- {{args}}

# Run the relay server locally (optional listen address, e.g. 0.0.0.0:9000)
relay *args:
    cargo run --locked --bin canon-relay -- {{args}}

# Build debug binaries
build:
    cargo build

# Build optimized binaries
build-release:
    cargo build --release --locked

# Tag HEAD as v<Cargo.toml version> and push it (triggers the Release workflow)
release:
    #!/usr/bin/env bash
    set -euo pipefail
    version="$(cargo pkgid)"
    version="${version##*#}"
    version="${version##*@}"
    tag="v$version"
    if [[ -n "$(git status --porcelain)" ]]; then
        echo "error: working tree has uncommitted changes; commit or stash before releasing $tag" >&2
        exit 1
    fi
    if git ls-remote --exit-code --tags origin "refs/tags/$tag" >/dev/null; then
        echo "error: tag $tag already exists on GitHub; bump the version in Cargo.toml" >&2
        exit 1
    fi
    if git rev-parse -q --verify "refs/tags/$tag" >/dev/null; then
        echo "error: tag $tag already exists locally (not on GitHub); delete it with: git tag -d $tag" >&2
        exit 1
    fi
    git tag -a "$tag" -m "Release $tag"
    git push origin "$tag"
    echo "Pushed $tag"

# Run tests
test:
    cargo test --locked

# Format and lint
check:
    cargo fmt --check
    cargo clippy --locked --all-targets -- -D warnings

# Format code
fmt:
    cargo fmt

# Build the native app package for this OS (dist/)
package:
    #!/usr/bin/env bash
    set -euo pipefail
    case "$(uname -s)" in
        Darwin) bash scripts/build-macos.sh ;;
        Linux) bash scripts/build-linux.sh ;;
        *) pwsh scripts/build-windows.ps1 ;;
    esac

# Build and install the game: /Applications on macOS, current-user menu entry on Linux
install: package
    #!/usr/bin/env bash
    set -euo pipefail
    case "$(uname -s)" in
        Darwin)
            rm -rf "/Applications/3D Canon.app"
            cp -R "dist/macos/3D Canon.app" /Applications/
            echo "Installed /Applications/3D Canon.app"
            ;;
        Linux)
            stage="$(ls -dt dist/linux/3D-Canon-*/ | head -n1)"
            bash "$stage/install.sh"
            ;;
        *)
            echo "No installer on this OS; see dist/windows." >&2
            exit 1
            ;;
    esac

# Build and run the relay in Docker
relay-docker:
    docker compose up --build -d
    docker compose logs -f

# Remove build output and dist/
clean:
    cargo clean
    rm -rf dist
