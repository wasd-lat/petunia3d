#!/bin/sh
set -eu

MODE="${PRUMO_UNINSTALL_MODE:-full}"
DRY_RUN=0
HOME_OVERRIDE=""
BINARY=""
REMOVE_BINARY=0

usage() {
  cat <<'USAGE'
Usage: uninstall.sh [--home <path>] [--binary <path>] [--remove-binary] [--mode pure|full] [--dry-run]

Modes:
  pure   remove only Prumo installation state, connectors, cache, and config
  full   additionally remove the prumo binary from PATH
USAGE
}

while [ $# -gt 0 ]; do
  case "$1" in
    --home)
      HOME_OVERRIDE="$2"
      shift 2
      ;;
    --binary)
      BINARY="$2"
      shift 2
      ;;
    --remove-binary)
      REMOVE_BINARY=1
      shift
      ;;
    --mode)
      MODE="$2"
      shift 2
      ;;
    --dry-run)
      DRY_RUN=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      printf '%s\n' "Unknown argument: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

resolve_home() {
  if [ -n "$HOME_OVERRIDE" ]; then
    printf '%s\n' "$HOME_OVERRIDE"
    return 0
  fi
  if [ -n "${PRUMO_HOME:-}" ]; then
    printf '%s\n' "$PRUMO_HOME"
    return 0
  fi
  printf '%s/.prumo\n' "$HOME"
}

resolve_binary() {
  if [ -n "$BINARY" ]; then
    printf '%s\n' "$BINARY"
    return 0
  fi
  command -v prumo || true
}

PRUMO_HOME_VALUE="$(resolve_home)"
BIN_PATH="$(resolve_binary)"

if [ "$REMOVE_BINARY" -eq 1 ] && [ -z "$BIN_PATH" ]; then
  printf '%s\n' "No prumo binary found in PATH; refusing to guess." >&2
  exit 1
fi

case "$MODE" in
  pure|full) ;;
  *)
    printf '%s\n' "Invalid mode: $MODE (expected pure or full)" >&2
    exit 1
    ;;
esac

if [ "$DRY_RUN" -eq 1 ]; then
  printf '%s\n' "Would run uninstall in ${PRUMO_HOME_VALUE} with mode ${MODE}."
  if [ "$REMOVE_BINARY" -eq 1 ]; then
    printf '%s\n' "Would remove binary ${BIN_PATH}."
  fi
  printf '%s\n' "Repository data is never removed by this script."
  exit 0
fi

if [ -z "$BIN_PATH" ]; then
  printf '%s\n' "No prumo binary available; nothing to execute." >&2
  exit 1
fi

# Never touch the current repository: only managed installation state is purged.
PRUMO_HOME="$PRUMO_HOME_VALUE" "$BIN_PATH" uninstall --connectors --purge-cache --purge-global-config

if [ "$MODE" = "full" ] && [ "$REMOVE_BINARY" -eq 1 ]; then
  rm -f "$BIN_PATH"
  printf '%s\n' "Removed binary ${BIN_PATH}."
fi

printf '%s\n' "Prumo uninstalled from ${PRUMO_HOME_VALUE}; repository data was preserved."
