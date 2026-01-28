#!/usr/bin/env bash

set -e

PROJECTS_DIR="$HOME/Desktop/Projects"

# Ensure the dir exists (create if missing)
mkdir -p "$PROJECTS_DIR"

# Launch Ghostty: first tab runs helix in the desired dir
ghostty --working-directory="$PROJECTS_DIR" -e hx &

# Wait longer for reliability (Hyprland window creation can vary)
sleep 2.0

# Focus the new Ghostty window
hyprctl dispatch focuswindow "class:com.mitchellh.ghostty" || true  # fallback if class differs

# Simulate keys to create new tab + cd + clear
sleep 0.6
wtype -M super t
sleep 0.8
wtype cd\ "$PROJECTS_DIR" && echo
wtype -k Return
sleep 0.2
wtype clear
