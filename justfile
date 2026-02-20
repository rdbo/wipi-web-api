runasroot := `echo ${RUNASROOT:-doas}`

run-dev: build-dev
    cargo run

build-dev:
    # Build in debug mode
    cargo build
    # Set binary capabilities
    {{runasroot}} ./setcaps.sh

build-release:
    # Build in release mode
    cargo build
    # Set binary capabilities
    {{runasroot}} ./setcaps.sh
