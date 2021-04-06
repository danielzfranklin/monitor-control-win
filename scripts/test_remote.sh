#!/usr/bin/env bash

REMOTE_HOST="daniel-surface.local"
LOCAL_PATH="$HOME"
REMOTE_PATH="/cygdrive/c/users/daniel/"
PROJECT_DIR_NAME="monitor-control-win"

rsync --archive --recursive --update --info=progress2 \
  "$LOCAL_PATH/$PROJECT_DIR_NAME" "$REMOTE_HOST:$REMOTE_PATH" && \
ssh $REMOTE_HOST "cd $REMOTE_PATH/$PROJECT_DIR_NAME && env CARGO_TARGET_DIR=\"/cygdrive/c/users/daniel/cargo_target\" cargo test"
