#!/bin/bash
set -e

if [ ! -f /app/ml_models/model.safetensors ] || [ ! -f /app/models/tokenizer.json ]; then
  echo "Downloading embedding models..."
  /app/download_models
  
  if [ ! -f /app/ml_models/model.safetensors ]; then
    echo "Error: Failed to download model.safetensors"
    exit 1
  fi
  
  if [ ! -f /app/ml_models/tokenizer.json ]; then
    echo "Error: Failed to download tokenizer.json"
    exit 1
  fi
  
  echo "Models downloaded successfully"
fi

exec /app/bb
