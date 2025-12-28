#!/bin/bash

################################################################################
# Setup
################################################################################
TAG_PREFIX=v
FILES=($(find action -name "*action.yml*"))

################################################################################
# Functions
################################################################################
info() {
  echo -e "\033[1;34m${@}\033[0m"
}

error() {
  echo -e "\033[1;31m${@}\033[0m"
}

die() {
  echo ""
  error $@
  echo ""
  exit 1
}

fetch_tags() {
  git fetch https://github.com/robgonnella/releasaurus main --tags
}

get_next_tag() {
  local next_release=$(cargo run --bin releasaurus -- show next-release \
  --forge local \
  --repo .)

  local next_version=$(echo "$next_release" | jq -r 'if (length > 0) then .[0].release.version else "" end')
  local next_tag="${TAG_PREFIX}${next_version}"

  echo $next_tag
}

check() {
  local next_tag="$1"
  local errors=()

  for file in ${FILES[@]}; do
    info "checking action file version usage: $file"
    if ! grep -q "$next_tag" "$file"; then
      errors+=("Error: Value '$next_tag' not found in file '$file'")
    fi
  done

  if [ "${#errors}" -ne 0 ]; then
    for e in "${errors[@]}"; do
      error "$e"
    done
    echo ""
    error "Action version file checks failed:"
    echo ""
    echo ""
    die "Run \"./scripts/action-versions.sh update\" locally then commit and push up changes"
  fi

  info "All action files are up-to-date with $next_tag"
}

update() {
  local next_tag="$1"

  for file in ${FILES[@]}; do
    info "updating $file to version: $next_tag"
    sed -i '' "s/v[0-9]\.[0-9]\.[0-9]/$next_tag/g" $file
  done
}

main() {
  local cmd="$1"

  info "fetching tags"
  fetch_tags

  info "analyzing repo..."
  local next_tag=$(get_next_tag)

  if [ -z "$next_tag" ]; then
    info "no next version detected"
    exit 0
  fi

  info "found projected next tag: $next_tag"

  if [ "$cmd" = "check" ]; then
    check "$next_tag"
  elif [ "$cmd" = "update" ]; then
    update "$next_tag"
  else
    error "unsupported subcommand: $cmd"
    die "supported values are: [check, update]"
  fi
}

################################################################################
# Main
################################################################################
main $@
