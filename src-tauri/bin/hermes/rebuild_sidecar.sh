#!/bin/zsh
unsetopt nomatch
set -e
cd "$(dirname "$0")"
echo "🚀 Starting Hermes Sidecar Induction (Workspace Portal)..."

UV="/Users/azaanaliraza/.local/bin/uv"

# 1. Clean
rm -rf build dist
rm -f *.spec
echo "✓ Previous artifacts purged."

# 2. Environment (Local to workspace)
$UV venv --python 3.12 --clear
source .venv/bin/activate
echo "✓ Python 3.12 environment initialized."

# 3. Stack Installation
if [ -f "requirements.txt" ]; then
  $UV pip install -r requirements.txt
else
  echo "⚠ requirements.txt not found, using pyproject.toml / direct install"
  $UV pip install "."
fi

$UV pip install "llama-cpp-python[server]" uvicorn fastapi sse-starlette starlette pydantic-settings
echo "✓ Networking stack induction complete."

# 4. PyInstaller Rebuild
$UV run pyinstaller --onefile \
  --name hermes \
  --hidden-import=llama_cpp.server \
  --hidden-import=uvicorn \
  --hidden-import=fastapi \
  --hidden-import=sse_starlette \
  --hidden-import=starlette \
  --hidden-import=hermes \
  --add-data "config:config" \
  --add-data "skills:skills" \
  --clean \
  run_agent.py
echo "✓ Neural sidecar bundle created."

# 5. Injection (Direct to sidecar portal)
cp dist/hermes ./hermes
chmod +x ./hermes
echo "✅ Hermes sidecar successfully injected into the PROJECT portal."
