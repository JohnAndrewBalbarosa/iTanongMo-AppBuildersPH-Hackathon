"""Private OpenAI-compatible Qwen endpoint for iTanongMo.

Deploy this separately from the 512 MiB web VPS. Modal proxy authentication keeps
the model endpoint private; the Rust app sends MODAL_KEY/MODAL_SECRET credentials.
"""

import subprocess

import modal

MODEL_ID = "Qwen/Qwen3-4B"
MODEL_REVISION = "main"

app = modal.App("itanongmo-isolated-ai")
model_volume = modal.Volume.from_name("itanongmo-qwen-cache", create_if_missing=True)
image = (
    modal.Image.from_registry("vllm/vllm-openai:v0.11.0")
    .env({"HF_HOME": "/models", "VLLM_LOGGING_LEVEL": "WARNING"})
)


@app.function(
    image=image,
    gpu="L4",
    timeout=900,
    scaledown_window=300,
    volumes={"/models": model_volume},
)
@modal.web_server(port=8000, startup_timeout=600, requires_proxy_auth=True)
def serve_model() -> None:
    subprocess.Popen(
        [
            "vllm",
            "serve",
            MODEL_ID,
            "--revision",
            MODEL_REVISION,
            "--host",
            "0.0.0.0",
            "--port",
            "8000",
            "--dtype",
            "auto",
            "--max-model-len",
            "8192",
            "--generation-config",
            "vllm",
            "--enable-prefix-caching",
        ],
        check=False,
    )
