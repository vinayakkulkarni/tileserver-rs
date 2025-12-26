#!/bin/sh
# Docker entrypoint script for tileserver-rs
# Based on tileserver-gl's proven approach for Xvfb handling

# Check if first arg is an executable
if ! which -- "${1}" > /dev/null 2>&1; then
  # First arg is not an executable, start Xvfb and run tileserver-rs
  
  # Clean up stale lock file if it exists (handles container restarts)
  if [ -e /tmp/.X99-lock ]; then 
    rm -f /tmp/.X99-lock
  fi
  
  # Set display for headless OpenGL rendering
  export DISPLAY=:99
  
  # Start Xvfb in background (no unix socket for security)
  Xvfb "${DISPLAY}" -nolisten unix &
  
  # Small delay to ensure Xvfb is ready
  sleep 0.5
  
  # Execute the server with all arguments
  exec "$@"
fi

# First arg is an executable, just run it
exec "$@"
